//! Участок B2 — библиотека звуков: `%APPDATA%\Boomly\library.json` + папка `sounds`.
//!
//! Публичные методы `Library::sound` и `Library::snapshot` — контракт (их зовут audio и
//! hotkeys). Каждая правка: копия данных → изменение → атомарная запись на диск → только
//! после успешной записи подмена в памяти. Так память и файл не расходятся.

pub mod commands;
pub mod migrate;
pub mod model;
pub mod types;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use crate::error::{AppError, AppResult};
use crate::system::store::{self, Loaded, io_err, now_ms};
use types::*;

pub const LIBRARY_FILE: &str = "library.json";
pub const SOUNDS_DIR: &str = "sounds";
/// Глубина обхода папок при импорте (защита от петель и случайного импорта диска целиком).
const MAX_DEPTH: usize = 12;

#[derive(Default)]
struct Inner {
    data: LibraryData,
    /// `None` — хранилище не открыто (тесты контрактов, до setup): правки только в памяти.
    file: Option<PathBuf>,
    sounds_dir: Option<PathBuf>,
    /// Открыть хранилище не удалось: правки запрещены, иначе они молча пропадут.
    unavailable: Option<String>,
}

#[derive(Default)]
pub struct Library {
    inner: Mutex<Inner>,
}

impl Library {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn snapshot(&self) -> LibraryData {
        self.lock().data.clone()
    }

    pub fn sound(&self, id: &str) -> Option<Sound> {
        self.lock().data.sounds.iter().find(|s| s.id == id).cloned()
    }

    /// Открывает хранилище в `data_dir` (вызывается из setup участка B2).
    /// Битый файл не роняет приложение: откладывается в сторону, берётся `.bak`.
    /// Если открыть не удалось (нет доступа и т.п.), изменяющие команды отвечают `io`.
    pub fn open(&self, data_dir: &Path) -> AppResult<()> {
        let res = Self::load(data_dir);
        let mut inner = self.lock();
        match res {
            Ok(fresh) => *inner = fresh,
            Err(e) => inner.unavailable = Some(e.message.clone()),
        }
        match &inner.unavailable {
            Some(m) => Err(AppError::new("io", m.clone())),
            None => Ok(()),
        }
    }

    fn load(data_dir: &Path) -> AppResult<Inner> {
        let file = data_dir.join(LIBRARY_FILE);
        let sounds_dir = data_dir.join(SOUNDS_DIR);
        fs::create_dir_all(&sounds_dir).map_err(io_err)?;
        let loaded = store::read_json(&file)?;
        let mut must_save = matches!(loaded, Loaded::Backup(_));
        let mut data = LibraryData::default();
        if let Some(value) = loaded.into_value() {
            let keep_aside = |reason: &str| -> AppResult<()> {
                let aside = file.with_file_name(format!("{LIBRARY_FILE}.unreadable-{}", now_ms()));
                eprintln!("[boomly] library.json: {reason}, копия оригинала — {}", aside.display());
                store::write_json_atomic(&aside, &value)
            };
            match migrate::migrate(value.clone()) {
                Err(e) => {
                    // Корректный JSON, но не объект — разбирать нечего; оригинал сохраняем.
                    keep_aside(&e.message)?;
                    must_save = true;
                }
                Ok(m) => {
                    if let Some(newer) = m.newer {
                        // Файл от более новой версии (откатили приложение). Оригинал — в сторону,
                        // всегда свежий, чтобы после обратного обновления ничего не потерялось.
                        let keep = file.with_file_name(format!("{LIBRARY_FILE}.v{newer}.bak"));
                        store::write_json_atomic(&keep, &value)?;
                    }
                    if let Some(from) = m.from {
                        let keep = file.with_file_name(format!("{LIBRARY_FILE}.v{from}.bak"));
                        store::write_json_atomic(&keep, &value)?;
                        must_save = true;
                    }
                    let (parsed, dropped) = migrate::parse_lenient(&m.value);
                    if dropped > 0 {
                        keep_aside(&format!("пропущено испорченных записей: {dropped}"))?;
                        must_save = true;
                    }
                    data = parsed;
                }
            }
        }
        model::sanitize(&mut data);
        if must_save {
            store::write_json_atomic(&file, &data)?;
        }
        Ok(Inner { data, file: Some(file), sounds_dir: Some(sounds_dir), unavailable: None })
    }

    fn persist(inner: &Inner, data: &LibraryData) -> AppResult<()> {
        if let Some(why) = &inner.unavailable {
            return Err(AppError::new("io", format!("библиотека недоступна: {why}")));
        }
        match &inner.file {
            Some(f) => store::write_json_atomic(f, data),
            None => Ok(()),
        }
    }

    /// Правка с сохранением. Возвращает результат `f` и новый снимок (для события).
    pub fn mutate<T>(&self, f: impl FnOnce(&mut LibraryData) -> AppResult<T>) -> AppResult<(T, LibraryData)> {
        let mut inner = self.lock();
        let mut next = inner.data.clone();
        let out = f(&mut next)?;
        Self::persist(&inner, &next)?;
        inner.data = next.clone();
        Ok((out, next))
    }

    /// Удаляет звуки; скопированные в `sounds` файлы удаляются тоже, если на тот же файл
    /// не ссылается другой звук.
    pub fn delete_sounds(&self, ids: &[String]) -> AppResult<(Vec<Sound>, LibraryData)> {
        let (removed, snap) = self.mutate(|d| Ok(model::delete_sounds(d, ids)))?;
        let sounds_dir = self.lock().sounds_dir.clone();
        if let Some(dir) = sounds_dir {
            let still_used: HashSet<PathBuf> =
                snap.sounds.iter().filter_map(|s| Path::new(&s.path).canonicalize().ok()).collect();
            for s in &removed {
                let p = Path::new(&s.path);
                let shared = p.canonicalize().is_ok_and(|c| still_used.contains(&c));
                if !shared
                    && is_inside(p, &dir)
                    && let Err(e) = fs::remove_file(p)
                {
                    // Например, файл ещё открыт. Звука в библиотеке уже нет — это главное.
                    eprintln!("[boomly] не удалось удалить {}: {e}", p.display());
                }
            }
        }
        Ok((removed, snap))
    }

    /// Импорт файлов и папок. Копирование идёт без блокировки библиотеки.
    pub fn import(&self, req: &ImportRequest, copy: bool) -> AppResult<(ImportResult, LibraryData)> {
        let sounds_dir = {
            let inner = self.lock();
            if let Some(why) = &inner.unavailable {
                return Err(AppError::new("io", format!("библиотека недоступна: {why}")));
            }
            if let Some(b) = &req.board_id
                && !inner.data.boards.iter().any(|x| &x.id == b)
            {
                return Err(AppError::new("not_found", format!("доска {b} не найдена")));
            }
            inner.sounds_dir.clone()
        };
        if copy && sounds_dir.is_none() {
            return Err(AppError::new("internal", "хранилище библиотеки не открыто"));
        }

        let (files, mut skipped) = collect_files(&req.paths);
        let mut prepared: Vec<Sound> = Vec::with_capacity(files.len());
        let mut copied: Vec<PathBuf> = vec![];
        let now = now_ms();
        for src in files {
            let id = model::new_id();
            let path = if copy {
                let dir = sounds_dir.as_ref().expect("checked above");
                let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("bin").to_ascii_lowercase();
                let dest = dir.join(format!("{id}.{ext}"));
                if let Err(e) = fs::copy(&src, &dest) {
                    eprintln!("[boomly] импорт {}: {e}", src.display());
                    let _ = fs::remove_file(&dest);
                    skipped.push(src.to_string_lossy().into_owned());
                    continue;
                }
                copied.push(dest.clone());
                dest
            } else {
                src.clone()
            };
            let name: String = src
                .file_stem()
                .map(|n| n.to_string_lossy().trim().to_string())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "Звук".into())
                .chars()
                .take(100)
                .collect();
            prepared.push(Sound {
                id,
                name,
                path: path.to_string_lossy().into_owned(),
                color: String::new(),
                emoji: None,
                volume: 1.0,
                mode: PlayMode::default(),
                favorite: false,
                duration_ms: None,
                added_at: now,
            });
        }

        let res = self.mutate(|d| {
            let base = d.sounds.len();
            for (i, s) in prepared.iter_mut().enumerate() {
                s.color = model::IMPORT_COLORS[(base + i) % model::IMPORT_COLORS.len()].into();
            }
            d.sounds.extend(prepared.iter().cloned());
            if let Some(b) = &req.board_id
                && let Ok(board) = model::board_mut(d, b)
            {
                board.sound_ids.extend(prepared.iter().map(|s| s.id.clone()));
            }
            Ok(())
        });
        match res {
            Ok(((), snap)) => Ok((ImportResult { added: prepared, skipped }, snap)),
            Err(e) => {
                for p in copied {
                    let _ = fs::remove_file(p);
                }
                Err(e)
            }
        }
    }
}

fn is_inside(path: &Path, dir: &Path) -> bool {
    match (path.canonicalize(), dir.canonicalize()) {
        (Ok(p), Ok(d)) => p.starts_with(d),
        _ => false,
    }
}

/// Раскрывает пути в список аудиофайлов. Явно указанный неподходящий файл — в `skipped`;
/// посторонние файлы внутри папок (обложки, txt) пропускаются молча.
pub fn collect_files(paths: &[String]) -> (Vec<PathBuf>, Vec<String>) {
    let mut files = vec![];
    let mut skipped = vec![];
    let mut seen = HashSet::new();
    for raw in paths {
        let p = PathBuf::from(raw);
        match fs::metadata(&p) {
            Ok(m) if m.is_dir() => walk(&p, 0, &mut files, &mut seen),
            Ok(m) if m.is_file() && model::is_audio_file(&p) => push_unique(p, &mut files, &mut seen),
            _ => skipped.push(raw.clone()),
        }
    }
    (files, skipped)
}

fn push_unique(p: PathBuf, files: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    let abs = std::path::absolute(&p).unwrap_or(p);
    let key = abs.canonicalize().unwrap_or_else(|_| abs.clone());
    if seen.insert(key) {
        files.push(abs);
    }
}

fn walk(dir: &Path, depth: usize, files: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    for e in entries {
        let Ok(ft) = e.file_type() else { continue };
        let p = e.path();
        // Ссылки на папки не обходим — защита от петель.
        if ft.is_dir() {
            walk(&p, depth + 1, files, seen);
        } else if model::is_audio_file(&p) && fs::metadata(&p).map(|m| m.is_file()).unwrap_or(false) {
            push_unique(p, files, seen);
        }
    }
}

#[cfg(test)]
mod tests;
