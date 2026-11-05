//! Надёжное хранение JSON-файлов в `%APPDATA%\Boomly` (участок B2).
//!
//! Запись атомарная: сериализуем во временный файл рядом, `fsync`, затем `rename` поверх
//! основного (на Windows это `MoveFileEx` с заменой). Предыдущая удачная версия остаётся
//! в `<имя>.bak`. Чтение: основной файл → если он битый, откладываем его в
//! `<имя>.corrupt-<ms>` и берём `.bak`. Данные пользователя мы никогда молча не стираем.
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::error::{AppError, AppResult};

pub fn io_err(e: impl std::fmt::Display) -> AppError {
    AppError::new("io", e.to_string())
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

pub fn backup_path(path: &Path) -> PathBuf {
    sibling(path, ".bak")
}

fn tmp_path(path: &Path) -> PathBuf {
    sibling(path, ".tmp")
}

/// Windows иногда держит файл (антивирус, индексатор) — несколько коротких повторов.
fn retry<T>(mut f: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    let mut last = None;
    for attempt in 0..5 {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                last = Some(e);
                std::thread::sleep(Duration::from_millis(20 * (attempt + 1)));
            }
        }
    }
    Err(last.expect("at least one attempt"))
}

/// Атомарно записывает `value` в `path` (pretty JSON), сохранив прежнюю версию в `.bak`.
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| AppError::new("internal", e.to_string()))?;
    write_bytes_atomic(path, &bytes)
}

pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io_err)?;
    }
    let tmp = tmp_path(path);
    {
        let mut f = fs::File::create(&tmp).map_err(io_err)?;
        f.write_all(bytes).map_err(io_err)?;
        f.sync_all().map_err(io_err)?;
    }
    // Бэкап только если текущий файл читается как JSON — не затираем хороший .bak битым.
    if let Ok(cur) = fs::read(path)
        && serde_json::from_slice::<Value>(&cur).is_ok()
    {
        let _ = retry(|| fs::write(backup_path(path), &cur));
    }
    retry(|| fs::rename(&tmp, path)).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        io_err(e)
    })
}

/// Откуда взялись прочитанные данные.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loaded {
    /// Файла нет — первый запуск.
    Missing,
    Main(Value),
    /// Основной файл был битым (отложен в `.corrupt-*`), взят `.bak`.
    Backup(Value),
}

impl Loaded {
    pub fn into_value(self) -> Option<Value> {
        match self {
            Loaded::Missing => None,
            Loaded::Main(v) | Loaded::Backup(v) => Some(v),
        }
    }
}

fn read_value(path: &Path) -> Option<std::io::Result<Value>> {
    match fs::read(path) {
        Ok(bytes) => Some(serde_json::from_slice(&bytes).map_err(std::io::Error::other)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => Some(Err(e)),
    }
}

/// Читает JSON с откатом на `.bak`. Ошибка — только если файл есть, но не читается
/// (нет доступа): тогда лучше не стартовать с пустыми данными и не перезаписать их.
pub fn read_json(path: &Path) -> AppResult<Loaded> {
    match read_value(path) {
        None => {}
        Some(Ok(v)) => return Ok(Loaded::Main(v)),
        Some(Err(e)) if e.kind() == std::io::ErrorKind::Other || e.kind() == std::io::ErrorKind::InvalidData => {
            let aside = sibling(path, &format!(".corrupt-{}", now_ms()));
            eprintln!("[boomly] {} повреждён ({e}), откладываю в {}", path.display(), aside.display());
            retry(|| fs::rename(path, &aside)).map_err(io_err)?;
        }
        Some(Err(e)) => return Err(io_err(format!("{}: {e}", path.display()))),
    }
    match read_value(&backup_path(path)) {
        Some(Ok(v)) => Ok(Loaded::Backup(v)),
        _ => Ok(Loaded::Missing),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn write_then_read() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.json");
        assert_eq!(read_json(&p).unwrap(), Loaded::Missing);
        write_json_atomic(&p, &json!({"x": 1})).unwrap();
        assert_eq!(read_json(&p).unwrap(), Loaded::Main(json!({"x": 1})));
        assert!(!tmp_path(&p).exists(), "временный файл не должен оставаться");
    }

    #[test]
    fn keeps_previous_version_in_bak() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.json");
        write_json_atomic(&p, &json!({"v": 1})).unwrap();
        write_json_atomic(&p, &json!({"v": 2})).unwrap();
        let bak: Value = serde_json::from_slice(&fs::read(backup_path(&p)).unwrap()).unwrap();
        assert_eq!(bak, json!({"v": 1}));
    }

    #[test]
    fn corrupt_main_falls_back_to_bak_and_is_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.json");
        write_json_atomic(&p, &json!({"v": 1})).unwrap();
        write_json_atomic(&p, &json!({"v": 2})).unwrap();
        // Обрыв записи «по-старому»: файл обрезан.
        fs::write(&p, b"{\"v\": 3, \"sou").unwrap();
        assert_eq!(read_json(&p).unwrap(), Loaded::Backup(json!({"v": 1})));
        assert!(!p.exists());
        let aside: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("a.json.corrupt-"))
            .collect();
        assert_eq!(aside.len(), 1, "битый файл сохраняется для разбора");
    }

    #[test]
    fn stale_tmp_does_not_affect_reading() {
        // Крах между записью .tmp и rename: основной файл цел.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.json");
        write_json_atomic(&p, &json!({"v": 1})).unwrap();
        fs::write(tmp_path(&p), b"{\"v\": 2, \"half").unwrap();
        assert_eq!(read_json(&p).unwrap(), Loaded::Main(json!({"v": 1})));
        write_json_atomic(&p, &json!({"v": 3})).unwrap();
        assert_eq!(read_json(&p).unwrap(), Loaded::Main(json!({"v": 3})));
    }

    #[test]
    fn corrupt_bak_is_not_overwritten_by_corrupt_main() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.json");
        write_json_atomic(&p, &json!({"v": 1})).unwrap();
        write_json_atomic(&p, &json!({"v": 2})).unwrap(); // .bak = v1
        fs::write(&p, b"garbage").unwrap();
        write_json_atomic(&p, &json!({"v": 3})).unwrap(); // битый main не должен попасть в .bak
        let bak: Value = serde_json::from_slice(&fs::read(backup_path(&p)).unwrap()).unwrap();
        assert_eq!(bak, json!({"v": 1}));
    }
}
