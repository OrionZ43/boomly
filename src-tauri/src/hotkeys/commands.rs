//! Tauri-команды хоткеев (участок B2). Изменяющие команды асинхронные: регистрация в ОС
//! идёт через главный поток, а команда ждёт результата не блокируя его.
use tauri::{AppHandle, State};

use super::Hotkeys;
use super::types::*;
use crate::error::AppResult;

#[tauri::command]
pub fn hotkeys_get(hotkeys: State<'_, Hotkeys>) -> Vec<HotkeyBinding> {
    hotkeys.list()
}

/// Добавляет или заменяет привязку для того же действия. Ошибка `hotkey_conflict`, если
/// сочетание занято другим действием или другой программой.
#[tauri::command]
pub async fn hotkeys_set(
    app: AppHandle,
    hotkeys: State<'_, Hotkeys>,
    binding: HotkeyBinding,
) -> AppResult<Vec<HotkeyBinding>> {
    hotkeys.set(&app, binding)
}

#[tauri::command]
pub async fn hotkeys_remove(
    app: AppHandle,
    hotkeys: State<'_, Hotkeys>,
    accelerator: String,
) -> AppResult<Vec<HotkeyBinding>> {
    hotkeys.remove(&app, &accelerator)
}

/// `true` — временно снять все глобальные хоткеи (пока UI записывает новое сочетание).
#[tauri::command]
pub async fn hotkeys_set_capture(app: AppHandle, hotkeys: State<'_, Hotkeys>, active: bool) -> AppResult<()> {
    hotkeys.set_capture(&app, active);
    Ok(())
}
