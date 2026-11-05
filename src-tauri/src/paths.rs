//! Пути данных. Владелец: A. Данные лежат в `%APPDATA%\Boomly` (не в папке по identifier),
//! установщик и деинсталлятор их не трогают.
use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

use crate::error::{AppError, AppResult};

pub const DATA_DIR_NAME: &str = "Boomly";

/// `%APPDATA%\Boomly` (создаётся при необходимости).
pub fn data_dir<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .data_dir()
        .map_err(|e| AppError::new("internal", e.to_string()))?
        .join(DATA_DIR_NAME);
    std::fs::create_dir_all(&dir).map_err(|e| AppError::new("io", e.to_string()))?;
    Ok(dir)
}

/// `%APPDATA%\Boomly\sounds` — скопированные звуки.
pub fn sounds_dir<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    let dir = data_dir(app)?.join("sounds");
    std::fs::create_dir_all(&dir).map_err(|e| AppError::new("io", e.to_string()))?;
    Ok(dir)
}
