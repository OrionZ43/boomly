//! Типы контракта библиотеки. Менять только вместе с docs/contracts.md и src/lib/api/types.ts.
use serde::{Deserialize, Serialize};

/// Что делает повторный клик/хоткей по уже играющему звуку.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlayMode {
    /// Повторный запуск останавливает звук.
    #[default]
    Toggle,
    /// Повторный запуск начинает сначала.
    Restart,
    /// Повторный запуск накладывает ещё одну копию.
    Overlay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    /// UUID v4.
    pub id: String,
    pub name: String,
    /// Абсолютный путь к файлу (копия в %APPDATA%\Boomly\sounds или оригинал).
    pub path: String,
    /// Цвет плитки — ключ палитры дизайн-системы (`cyan`, `purple`, ...) или `#rrggbb`.
    pub color: String,
    pub emoji: Option<String>,
    /// Громкость звука 0.0..=2.0 (1.0 — как есть).
    pub volume: f32,
    pub mode: PlayMode,
    pub favorite: bool,
    /// Заполняется при импорте, если удалось прочитать длительность.
    pub duration_ms: Option<u64>,
    /// Unix-время в мс.
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub id: String,
    pub name: String,
    pub emoji: Option<String>,
    /// Порядок плиток на доске. Звук может быть на нескольких досках.
    pub sound_ids: Vec<String>,
}

/// Содержимое `%APPDATA%\Boomly\library.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryData {
    /// Версия схемы файла, сейчас 1.
    pub version: u32,
    pub sounds: Vec<Sound>,
    /// Порядок досок — порядок в списке.
    pub boards: Vec<Board>,
}

impl Default for LibraryData {
    fn default() -> Self {
        Self { version: 1, sounds: vec![], boards: vec![] }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    /// Файлы и/или папки (папки — рекурсивно, mp3/wav/ogg/flac).
    pub paths: Vec<String>,
    /// На какую доску добавить; `None` — только в библиотеку.
    pub board_id: Option<String>,
    /// Копировать файлы в %APPDATA%\Boomly\sounds (по умолчанию из настроек).
    pub copy_files: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub added: Vec<Sound>,
    /// Пути, которые не удалось импортировать (не тот формат, нет доступа).
    pub skipped: Vec<String>,
}
