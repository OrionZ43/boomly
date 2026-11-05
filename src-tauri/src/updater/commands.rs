//! Команды участка E.

/// Включено ли автообновление в этой сборке: есть ли публичный ключ в
/// `plugins.updater.pubkey`. Без ключа установка обновления всё равно упадёт на подписи,
/// поэтому UI в такой сборке обновления не ищет.
#[tauri::command]
pub fn updater_enabled(app: tauri::AppHandle) -> bool {
    super::pubkey_configured(app.config())
}
