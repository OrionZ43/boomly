//! Кэш декодированных звуков в памяти: путь → клипы на частотах выходов.
//!
//! Ключ проверяется по размеру и времени изменения файла (заменили файл — перечитаем).
//! Бюджет памяти ограничен; при превышении выбрасываются давно не игравшие звуки.
//! Длинные звуки (> `MAX_CACHED_MS`) не кэшируются — играются, но декодируются заново.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use super::decode::{self, Clip};
use crate::error::AppResult;

/// Бюджет кэша (≈ 18 минут стерео 48 кГц f32).
pub const DEFAULT_BUDGET_BYTES: usize = 400 * 1024 * 1024;
/// Звуки длиннее не держим в кэше.
pub const MAX_CACHED_MS: u64 = 3 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
}

fn stamp(path: &Path) -> Option<FileStamp> {
    let m = std::fs::metadata(path).ok()?;
    Some(FileStamp {
        len: m.len(),
        modified: m.modified().ok(),
    })
}

struct Entry {
    stamp: FileStamp,
    variants: HashMap<u32, Arc<Clip>>,
    last_used: u64,
}

impl Entry {
    fn bytes(&self) -> usize {
        self.variants.values().map(|c| c.bytes()).sum()
    }
}

pub struct ClipCache {
    entries: Mutex<HashMap<PathBuf, Entry>>,
    /// Слишком длинные для кэша — чтобы предзагрузка не декодировала их снова и снова.
    too_long: Mutex<HashMap<PathBuf, FileStamp>>,
    budget: usize,
    clock: AtomicU64,
}

impl Default for ClipCache {
    fn default() -> Self {
        Self::new(DEFAULT_BUDGET_BYTES)
    }
}

impl ClipCache {
    pub fn new(budget: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            too_long: Mutex::new(HashMap::new()),
            budget,
            clock: AtomicU64::new(0),
        }
    }

    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed)
    }

    /// Только из памяти, без диска (кроме `metadata` для проверки свежести).
    /// `None`, если хоть одной нужной частоты нет.
    pub fn try_get(&self, path: &Path, rates: &[u32]) -> Option<Vec<Arc<Clip>>> {
        let st = stamp(path)?;
        let mut map = self.entries.lock().unwrap();
        let e = map.get_mut(path)?;
        if e.stamp != st {
            map.remove(path);
            return None;
        }
        let clips: Option<Vec<_>> = rates.iter().map(|r| e.variants.get(r).cloned()).collect();
        if clips.is_some() {
            e.last_used = self.tick();
        }
        clips
    }

    /// Взять из кэша или декодировать (долго — не вызывать из потока UI/аудио).
    pub fn get_or_load(&self, path: &Path, rates: &[u32]) -> AppResult<Vec<Arc<Clip>>> {
        if let Some(c) = self.try_get(path, rates) {
            return Ok(c);
        }
        let st = stamp(path);
        let native = decode::decode_file(path)?;
        let mut out = Vec::with_capacity(rates.len());
        let mut made: HashMap<u32, Arc<Clip>> = HashMap::new();
        for &r in rates {
            let clip = match made.get(&r) {
                Some(c) => c.clone(),
                None => {
                    let c = Arc::new(decode::resample(&native, r)?);
                    made.insert(r, c.clone());
                    c
                }
            };
            out.push(clip);
        }
        let duration = native.duration_ms();
        drop(native);
        if let Some(st) = st {
            if duration > MAX_CACHED_MS {
                self.too_long.lock().unwrap().insert(path.to_path_buf(), st);
            } else {
                self.insert(path, st, made);
            }
        }
        Ok(out)
    }

    /// Предзагрузка: декодировать заранее, если звук ещё не в кэше и влезает.
    pub fn preload(&self, path: &Path, rates: &[u32]) -> AppResult<()> {
        if self.try_get(path, rates).is_some() {
            return Ok(());
        }
        if let (Some(st), Some(known)) = (stamp(path), self.too_long.lock().unwrap().get(path))
            && *known == st
        {
            return Ok(());
        }
        self.get_or_load(path, rates).map(|_| ())
    }

    fn insert(&self, path: &Path, st: FileStamp, variants: HashMap<u32, Arc<Clip>>) {
        let mut map = self.entries.lock().unwrap();
        let last_used = self.tick();
        match map.get_mut(path) {
            Some(e) if e.stamp == st => {
                e.variants.extend(variants);
                e.last_used = last_used;
            }
            _ => {
                map.insert(
                    path.to_path_buf(),
                    Entry {
                        stamp: st,
                        variants,
                        last_used,
                    },
                );
            }
        }
        // выселяем самые старые, пока не влезем в бюджет (только что добавленный — последним)
        let mut total: usize = map.values().map(Entry::bytes).sum();
        while total > self.budget && map.len() > 1 {
            let oldest = map
                .iter()
                .filter(|(p, _)| p.as_path() != path)
                .min_by_key(|(_, e)| e.last_used)
                .map(|(p, _)| p.clone());
            match oldest {
                Some(p) => {
                    if let Some(e) = map.remove(&p) {
                        total -= e.bytes();
                    }
                }
                None => break,
            }
        }
    }

    /// Оставить только варианты на указанных частотах (после смены устройств).
    pub fn retain_rates(&self, rates: &[u32]) {
        let mut map = self.entries.lock().unwrap();
        for e in map.values_mut() {
            e.variants.retain(|r, _| rates.contains(r));
        }
        map.retain(|_, e| !e.variants.is_empty());
    }

    pub fn bytes(&self) -> usize {
        self.entries
            .lock()
            .unwrap()
            .values()
            .map(Entry::bytes)
            .sum()
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(path: &Path, frames: usize) {
        let data_len = (frames * 2) as u32;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data_len).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&48_000u32.to_le_bytes());
        b.extend_from_slice(&96_000u32.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&data_len.to_le_bytes());
        b.resize(b.len() + frames * 2, 0);
        std::fs::write(path, b).unwrap();
    }

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("boomly-cache-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn caches_and_shares_same_rate() {
        let d = tmpdir("a");
        let p = d.join("a.wav");
        wav(&p, 4800);
        let cache = ClipCache::default();
        assert!(cache.try_get(&p, &[48_000]).is_none());
        let clips = cache.get_or_load(&p, &[48_000, 48_000]).unwrap();
        assert!(
            Arc::ptr_eq(&clips[0], &clips[1]),
            "одна частота — один клип"
        );
        let again = cache.try_get(&p, &[48_000]).unwrap();
        assert!(Arc::ptr_eq(&again[0], &clips[0]));
        // другой частоты нет — промах
        assert!(cache.try_get(&p, &[44_100]).is_none());
        let both = cache.get_or_load(&p, &[48_000, 44_100]).unwrap();
        assert_eq!(both[1].rate, 44_100);
        assert!(cache.try_get(&p, &[48_000, 44_100]).is_some());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn evicts_least_recently_used() {
        let d = tmpdir("b");
        let (a, b, c) = (d.join("a.wav"), d.join("b.wav"), d.join("c.wav"));
        for p in [&a, &b, &c] {
            wav(p, 48_000); // 1 с → 384 000 байт в кэше
        }
        let cache = ClipCache::new(900_000); // влезают два
        cache.get_or_load(&a, &[48_000]).unwrap();
        cache.get_or_load(&b, &[48_000]).unwrap();
        cache.try_get(&a, &[48_000]).unwrap(); // a свежее b
        cache.get_or_load(&c, &[48_000]).unwrap();
        assert_eq!(cache.len(), 2);
        assert!(cache.try_get(&a, &[48_000]).is_some());
        assert!(cache.try_get(&b, &[48_000]).is_none());
        assert!(cache.bytes() <= 900_000);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn changed_file_is_reloaded() {
        let d = tmpdir("c");
        let p = d.join("a.wav");
        wav(&p, 4800);
        let cache = ClipCache::default();
        cache.get_or_load(&p, &[48_000]).unwrap();
        wav(&p, 9600); // другой размер
        assert!(cache.try_get(&p, &[48_000]).is_none());
        assert_eq!(cache.get_or_load(&p, &[48_000]).unwrap()[0].frames(), 9600);
        std::fs::remove_dir_all(&d).ok();
    }
}
