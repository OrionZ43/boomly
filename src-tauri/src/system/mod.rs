//! Участок B2 — системная интеграция: настройки приложения, трей, автозапуск,
//! single-instance. Сейчас — заглушка: настройки в памяти.

pub mod commands;
pub mod types;

use std::sync::Mutex;

/// Регистрирует плагины и setup участка. Вызывается из `lib.rs`.
pub fn init<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
}

use types::AppSettings;

#[derive(Default)]
pub struct SettingsStore {
    settings: Mutex<AppSettings>,
}

impl SettingsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self) -> AppSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set(&self, s: AppSettings) -> AppSettings {
        *self.settings.lock().unwrap() = s.clone();
        s
    }
}
