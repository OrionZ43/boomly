//! Tauri-команды настроек и системы (участок B2).
use tauri::{AppHandle, State};

use super::types::*;
use super::SettingsStore;
use crate::error::{AppError, AppResult};

#[tauri::command]
pub fn app_get_settings(store: State<'_, SettingsStore>) -> AppSettings {
    store.get()
}

/// Сохраняет настройки целиком и применяет (автозапуск и т.п.).
#[tauri::command]
pub fn app_set_settings(store: State<'_, SettingsStore>, settings: AppSettings) -> AppResult<AppSettings> {
    Ok(store.set(settings))
}

#[tauri::command]
pub fn app_info(app: AppHandle) -> AppResult<AppInfo> {
    let data_dir = crate::paths::data_dir(&app)?;
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: data_dir.to_string_lossy().into_owned(),
    })
}

/// Открыть `%APPDATA%\Boomly` в Проводнике.
#[tauri::command]
pub fn app_open_data_dir() -> AppResult<()> {
    Err(AppError::not_implemented("app_open_data_dir"))
}
