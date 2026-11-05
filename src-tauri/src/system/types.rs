//! Типы контракта настроек приложения. Менять только вместе с docs/contracts.md и src/lib/api/types.ts.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    #[default]
    Ru,
    En,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EffectsLevel {
    Minimal,
    #[default]
    Normal,
}

/// Содержимое `%APPDATA%\Boomly\settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub language: Language,
    pub effects: EffectsLevel,
    /// Акцент интерфейса — ключ палитры (`cyan` по умолчанию).
    pub accent: String,
    pub autostart: bool,
    /// Крестик сворачивает в трей, а не закрывает.
    pub close_to_tray: bool,
    pub start_minimized: bool,
    /// Копировать импортируемые файлы в папку приложения.
    pub copy_imported_files: bool,
    /// Мастер первого запуска пройден.
    pub first_run_completed: bool,
    /// Автопроверка обновлений (участок E).
    pub check_updates: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::Ru,
            effects: EffectsLevel::Normal,
            accent: "cyan".into(),
            autostart: false,
            close_to_tray: true,
            start_minimized: false,
            copy_imported_files: true,
            first_run_completed: false,
            check_updates: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    /// `%APPDATA%\Boomly`
    pub data_dir: String,
}
