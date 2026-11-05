//! Tauri-команды библиотеки (участок B2). Все изменяющие команды после записи
//! шлют событие `library://changed` с полной `LibraryData`.
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use super::types::*;
use super::{Library, model};
use crate::audio::AudioEngine;
use crate::error::{AppError, AppResult};
use crate::events::LIBRARY_CHANGED;
use crate::hotkeys::Hotkeys;
use crate::system::SettingsStore;

fn changed<R: Runtime>(app: &AppHandle<R>, data: &LibraryData) {
    if let Err(e) = app.emit(LIBRARY_CHANGED, data) {
        eprintln!("[boomly] emit {LIBRARY_CHANGED}: {e}");
    }
}

#[tauri::command]
pub fn library_get(library: State<'_, Library>) -> LibraryData {
    library.snapshot()
}

/// Импорт может копировать сотни мегабайт — выполняем вне главного потока.
#[tauri::command]
pub async fn library_import(app: AppHandle, request: ImportRequest) -> AppResult<ImportResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let copy = request.copy_files.unwrap_or_else(|| app.state::<SettingsStore>().get().copy_imported_files);
        let (res, snap) = app.state::<Library>().import(&request, copy)?;
        changed(&app, &snap);
        Ok(res)
    })
    .await
    .map_err(|e| AppError::new("internal", e.to_string()))?
}

/// Заменяет звук целиком (по `id`).
#[tauri::command]
pub fn library_update_sound(app: AppHandle, library: State<'_, Library>, sound: Sound) -> AppResult<Sound> {
    let (s, snap) = library.mutate(|d| model::update_sound(d, sound))?;
    changed(&app, &snap);
    Ok(s)
}

/// Удаляет звуки из библиотеки и со всех досок; скопированные файлы и хоткеи — тоже.
/// Асинхронная: снятие хоткеев ждёт главный поток (см. `Hotkeys`).
#[tauri::command]
pub async fn library_delete_sounds(app: AppHandle, ids: Vec<String>) -> AppResult<()> {
    // Сначала стоп: открытый файл Windows не даст удалить.
    let engine = app.state::<AudioEngine>();
    for id in &ids {
        let _ = engine.stop(id);
    }
    let (_, snap) = app.state::<Library>().delete_sounds(&ids)?;
    app.state::<Hotkeys>().remove_for_sounds(&app, &ids);
    changed(&app, &snap);
    Ok(())
}

#[tauri::command]
pub fn library_create_board(
    app: AppHandle,
    library: State<'_, Library>,
    name: String,
    emoji: Option<String>,
) -> AppResult<Board> {
    let (b, snap) = library.mutate(|d| model::create_board(d, &name, emoji))?;
    changed(&app, &snap);
    Ok(b)
}

/// Заменяет доску целиком: имя, эмодзи, порядок и состав `soundIds`.
#[tauri::command]
pub fn library_update_board(app: AppHandle, library: State<'_, Library>, board: Board) -> AppResult<Board> {
    let (b, snap) = library.mutate(|d| model::update_board(d, board))?;
    changed(&app, &snap);
    Ok(b)
}

/// Удаляет доску; звуки остаются в библиотеке.
#[tauri::command]
pub fn library_delete_board(app: AppHandle, library: State<'_, Library>, id: String) -> AppResult<()> {
    let (_, snap) = library.mutate(|d| model::delete_board(d, &id))?;
    changed(&app, &snap);
    Ok(())
}

/// Новый порядок досок.
#[tauri::command]
pub fn library_reorder_boards(app: AppHandle, library: State<'_, Library>, ids: Vec<String>) -> AppResult<()> {
    let (_, snap) = library.mutate(|d| {
        model::reorder_boards(d, &ids);
        Ok(())
    })?;
    changed(&app, &snap);
    Ok(())
}
