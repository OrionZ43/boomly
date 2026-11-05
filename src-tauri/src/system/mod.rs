//! Участок B2 — системная интеграция: настройки приложения (`settings.json`), трей,
//! сворачивание в трей, автозапуск с Windows, single-instance. Здесь же открываются
//! хранилища библиотеки и настроек при старте.

pub mod commands;
pub mod store;
pub mod tray;
pub mod types;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use tauri::{AppHandle, Emitter, Manager, Runtime, WindowEvent};
use tauri_plugin_autostart::ManagerExt as _;

use crate::error::{AppError, AppResult};
use crate::events;
use crate::library::Library;
use types::AppSettings;

pub const SETTINGS_FILE: &str = "settings.json";
pub const MAIN_WINDOW: &str = "main";
/// Аргумент, с которым Windows запускает нас при входе в систему.
pub const AUTOSTART_ARG: &str = "--autostart";

/// Регистрирует плагины и setup участка. Вызывается из `lib.rs` первым.
pub fn init<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        // Должен быть первым: второй экземпляр завершается, первый показывает окно.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::Builder::new().arg(AUTOSTART_ARG).build())
        // Свой мини-плагин, чтобы не занимать единственный `Builder::setup` приложения.
        .plugin(
            tauri::plugin::Builder::<R>::new("boomly-system")
                .setup(|app, _api| {
                    let dir = crate::paths::data_dir(app)?;
                    if let Err(e) = app.state::<SettingsStore>().open(&dir) {
                        eprintln!("[boomly] settings.json: {e}");
                    }
                    // Без библиотеки приложение бессмысленно, но падать молча хуже —
                    // пишем в лог и стартуем; ошибку увидят по пустой библиотеке.
                    if let Err(e) = app.state::<Library>().open(&dir) {
                        eprintln!("[boomly] library.json: {e}");
                    }
                    let settings = app.state::<SettingsStore>().get();
                    if let Err(e) = sync_autostart(app, settings.autostart) {
                        eprintln!("[boomly] автозапуск: {e}");
                    }
                    if let Err(e) = tray::create(app, &settings) {
                        eprintln!("[boomly] трей: {e}");
                    }
                    Ok(())
                })
                .on_window_ready(|window| {
                    if window.label() != MAIN_WINDOW {
                        return;
                    }
                    let app = window.app_handle().clone();
                    if app.state::<SettingsStore>().get().start_minimized {
                        let _ = window.hide();
                    }
                    let w = window.clone();
                    window.on_window_event(move |e| {
                        if let WindowEvent::CloseRequested { api, .. } = e
                            && app.state::<SettingsStore>().get().close_to_tray
                        {
                            api.prevent_close();
                            let _ = w.hide();
                        }
                    });
                })
                .build(),
        )
}

/// Показать главное окно и вывести на передний план.
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(MAIN_WINDOW) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Показать, если скрыто или свёрнуто; иначе спрятать в трей.
pub fn toggle_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(MAIN_WINDOW) {
        let visible = w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false);
        if visible {
            let _ = w.hide();
        } else {
            show_main(app);
        }
    }
}

/// Приводит запись автозапуска в реестре (HKCU\...\Run) к настройке. При включении
/// перезаписываем всегда — путь к exe мог смениться.
fn sync_autostart<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> AppResult<()> {
    let al = app.autolaunch();
    let res = if enabled {
        al.enable()
    } else if al.is_enabled().unwrap_or(false) {
        al.disable()
    } else {
        Ok(())
    };
    res.map_err(|e| AppError::new("io", format!("автозапуск: {e}")))
}

#[derive(Default)]
struct Inner {
    settings: AppSettings,
    file: Option<PathBuf>,
}

#[derive(Default)]
pub struct SettingsStore {
    inner: Mutex<Inner>,
}

impl SettingsStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn get(&self) -> AppSettings {
        self.lock().settings.clone()
    }

    pub fn open(&self, data_dir: &Path) -> AppResult<()> {
        let file = data_dir.join(SETTINGS_FILE);
        let settings = match store::read_json(&file)?.into_value() {
            None => AppSettings::default(),
            Some(v) => parse_settings(v),
        };
        *self.lock() = Inner { settings, file: Some(file) };
        Ok(())
    }

    /// Сохраняет без применения к системе (для тестов и внутреннего использования).
    pub fn save(&self, s: AppSettings) -> AppResult<AppSettings> {
        let mut inner = self.lock();
        if let Some(f) = &inner.file {
            store::write_json_atomic(f, &s)?;
        }
        inner.settings = s.clone();
        Ok(s)
    }

    /// Применяет и сохраняет настройки, шлёт `app://settings-changed`.
    pub fn set<R: Runtime>(&self, app: &AppHandle<R>, s: AppSettings) -> AppResult<AppSettings> {
        let prev = self.get();
        let autostart_changed = prev.autostart != s.autostart;
        if autostart_changed {
            sync_autostart(app, s.autostart)?;
        }
        let saved = match self.save(s) {
            Ok(v) => v,
            Err(e) => {
                if autostart_changed {
                    let _ = sync_autostart(app, prev.autostart);
                }
                return Err(e);
            }
        };
        if prev.language != saved.language {
            tray::update_language(app, saved.language);
        }
        let _ = app.emit(events::APP_SETTINGS_CHANGED, &saved);
        Ok(saved)
    }
}

/// Читает настройки поле за полем: одно непонятное значение (например, язык из будущей
/// версии) не сбрасывает остальные к умолчаниям.
pub fn parse_settings(v: serde_json::Value) -> AppSettings {
    if let Ok(s) = serde_json::from_value::<AppSettings>(v.clone()) {
        return s;
    }
    let mut out = serde_json::to_value(AppSettings::default()).expect("serializable");
    if let (Some(src), Some(dst)) = (v.as_object(), out.as_object_mut()) {
        for (k, val) in src {
            if !dst.contains_key(k) {
                continue;
            }
            let prev = dst.insert(k.clone(), val.clone());
            if serde_json::from_value::<AppSettings>(serde_json::Value::Object(dst.clone())).is_err()
                && let Some(p) = prev
            {
                dst.insert(k.clone(), p);
            }
        }
    }
    serde_json::from_value(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::types::*;
    use super::*;
    use serde_json::json;

    #[test]
    fn settings_roundtrip_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let s = SettingsStore::new();
        s.open(dir.path()).unwrap();
        assert_eq!(s.get(), AppSettings::default());
        let mut next = s.get();
        next.language = Language::En;
        next.close_to_tray = false;
        s.save(next.clone()).unwrap();
        let s2 = SettingsStore::new();
        s2.open(dir.path()).unwrap();
        assert_eq!(s2.get(), next);
    }

    #[test]
    fn unknown_value_keeps_other_fields() {
        let s = parse_settings(json!({"language": "de", "autostart": true, "accent": "purple", "extra": 1}));
        assert_eq!(s.language, Language::Ru);
        assert!(s.autostart);
        assert_eq!(s.accent, "purple");
    }

    #[test]
    fn missing_fields_take_defaults() {
        let s = parse_settings(json!({"startMinimized": true}));
        assert!(s.start_minimized);
        assert!(s.close_to_tray);
        assert!(s.copy_imported_files);
    }
}
