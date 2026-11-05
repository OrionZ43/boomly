//! Tauri-команды хоткеев (участок B2).
use tauri::State;

use super::types::*;
use super::Hotkeys;
use crate::error::{AppError, AppResult};

#[tauri::command]
pub fn hotkeys_get(hotkeys: State<'_, Hotkeys>) -> Vec<HotkeyBinding> {
    hotkeys.list()
}

/// Добавляет или заменяет привязку для того же действия. Ошибка `hotkey_conflict`, если
/// сочетание занято другим действием или другой программой.
#[tauri::command]
pub fn hotkeys_set(
    _hotkeys: State<'_, Hotkeys>,
    _binding: HotkeyBinding,
) -> AppResult<Vec<HotkeyBinding>> {
    Err(AppError::not_implemented("hotkeys_set"))
}

#[tauri::command]
pub fn hotkeys_remove(
    _hotkeys: State<'_, Hotkeys>,
    _accelerator: String,
) -> AppResult<Vec<HotkeyBinding>> {
    Err(AppError::not_implemented("hotkeys_remove"))
}

/// `true` — временно снять все глобальные хоткеи (пока UI записывает новое сочетание).
#[tauri::command]
pub fn hotkeys_set_capture(_hotkeys: State<'_, Hotkeys>, _active: bool) -> AppResult<()> {
    Ok(())
}
