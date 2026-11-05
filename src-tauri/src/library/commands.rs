//! Tauri-команды библиотеки (участок B2). Все изменяющие команды после записи
//! шлют событие `library://changed` с полной `LibraryData`.
use tauri::State;

use super::types::*;
use super::Library;
use crate::error::{AppError, AppResult};

#[tauri::command]
pub fn library_get(library: State<'_, Library>) -> LibraryData {
    library.snapshot()
}

#[tauri::command]
pub fn library_import(_library: State<'_, Library>, _request: ImportRequest) -> AppResult<ImportResult> {
    Err(AppError::not_implemented("library_import"))
}

/// Заменяет звук целиком (по `id`).
#[tauri::command]
pub fn library_update_sound(_library: State<'_, Library>, _sound: Sound) -> AppResult<Sound> {
    Err(AppError::not_implemented("library_update_sound"))
}

/// Удаляет звуки из библиотеки и со всех досок; скопированные файлы — тоже.
#[tauri::command]
pub fn library_delete_sounds(_library: State<'_, Library>, _ids: Vec<String>) -> AppResult<()> {
    Err(AppError::not_implemented("library_delete_sounds"))
}

#[tauri::command]
pub fn library_create_board(
    _library: State<'_, Library>,
    _name: String,
    _emoji: Option<String>,
) -> AppResult<Board> {
    Err(AppError::not_implemented("library_create_board"))
}

/// Заменяет доску целиком: имя, эмодзи, порядок и состав `soundIds`.
#[tauri::command]
pub fn library_update_board(_library: State<'_, Library>, _board: Board) -> AppResult<Board> {
    Err(AppError::not_implemented("library_update_board"))
}

/// Удаляет доску; звуки остаются в библиотеке.
#[tauri::command]
pub fn library_delete_board(_library: State<'_, Library>, _id: String) -> AppResult<()> {
    Err(AppError::not_implemented("library_delete_board"))
}

/// Новый порядок досок.
#[tauri::command]
pub fn library_reorder_boards(_library: State<'_, Library>, _ids: Vec<String>) -> AppResult<()> {
    Err(AppError::not_implemented("library_reorder_boards"))
}
