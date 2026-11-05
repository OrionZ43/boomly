//! Tauri-команды настроек и системы (участок B2).
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use super::SettingsStore;
use super::types::*;
use crate::error::{AppError, AppResult};

#[tauri::command]
pub fn app_get_settings(store: State<'_, SettingsStore>) -> AppSettings {
    store.get()
}

/// Сохраняет настройки целиком и применяет (автозапуск, язык трея).
#[tauri::command]
pub async fn app_set_settings(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    settings: AppSettings,
) -> AppResult<AppSettings> {
    store.set(&app, settings)
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
pub fn app_open_data_dir(app: AppHandle) -> AppResult<()> {
    let dir = crate::paths::data_dir(&app)?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::new("io", e.to_string()))
}
