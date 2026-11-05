//! Участок E — автообновление (tauri-plugin-updater). Подключение плагина и, при
//! необходимости, команды `updater_*` — здесь. UI-обёртка: src/lib/api/updater.ts.
//! Пока пусто: ключ подписи и endpoint ещё не настроены.

/// Регистрирует плагины и обработчики участка E. Вызывается из `lib.rs`.
pub fn init<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
}
