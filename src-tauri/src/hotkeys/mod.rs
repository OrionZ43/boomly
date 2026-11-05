//! Участок B2 — глобальные хоткеи (tauri-plugin-global-shortcut). Хранятся в
//! `%APPDATA%\Boomly\hotkeys.json`. Действие выполняется в Rust (через `AudioEngine`),
//! затем шлётся `hotkeys://triggered`. Сейчас — заглушка.

pub mod commands;
pub mod types;

use std::sync::Mutex;

/// Регистрирует плагины и setup участка. Вызывается из `lib.rs`.
pub fn init<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
}

use types::HotkeyBinding;

#[derive(Default)]
pub struct Hotkeys {
    bindings: Mutex<Vec<HotkeyBinding>>,
}

impl Hotkeys {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Vec<HotkeyBinding> {
        self.bindings.lock().unwrap().clone()
    }
}
