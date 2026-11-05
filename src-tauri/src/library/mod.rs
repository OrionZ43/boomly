//! Участок B2 — библиотека звуков: `%APPDATA%\Boomly\library.json` + папка `sounds`.
//! Сейчас — заглушка в памяти. Публичные методы `Library` — контракт (их зовут audio и hotkeys).

pub mod commands;
pub mod types;

use std::sync::Mutex;

use types::*;

#[derive(Default)]
pub struct Library {
    data: Mutex<LibraryData>,
}

impl Library {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> LibraryData {
        self.data.lock().unwrap().clone()
    }

    pub fn sound(&self, id: &str) -> Option<Sound> {
        self.data.lock().unwrap().sounds.iter().find(|s| s.id == id).cloned()
    }
}
