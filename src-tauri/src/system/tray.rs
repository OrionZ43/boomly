//! Иконка в трее: левый клик — показать окно; меню «Показать/скрыть», «Стоп всё», «Выход».
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use super::types::{AppSettings, Language};
use crate::audio::AudioEngine;
use crate::events;

pub const TRAY_ID: &str = "boomly-tray";
const ID_TOGGLE: &str = "tray-toggle";
const ID_STOP: &str = "tray-stop-all";
const ID_QUIT: &str = "tray-quit";

struct Labels {
    toggle: &'static str,
    stop: &'static str,
    quit: &'static str,
}

fn labels(lang: Language) -> Labels {
    match lang {
        Language::Ru => Labels { toggle: "Показать / скрыть", stop: "Стоп всё", quit: "Выход" },
        Language::En => Labels { toggle: "Show / hide", stop: "Stop all", quit: "Quit" },
    }
}

fn build_menu<R: Runtime>(app: &AppHandle<R>, lang: Language) -> tauri::Result<Menu<R>> {
    let l = labels(lang);
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, ID_TOGGLE, l.toggle, true, None::<&str>)?,
            &MenuItem::with_id(app, ID_STOP, l.stop, true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, ID_QUIT, l.quit, true, None::<&str>)?,
        ],
    )
}

pub fn create<R: Runtime>(app: &AppHandle<R>, settings: &AppSettings) -> tauri::Result<()> {
    let mut b = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Boomly")
        .menu(&build_menu(app, settings.language)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id().as_ref() {
            ID_TOGGLE => super::toggle_main(app),
            ID_STOP => {
                if let Err(err) = app.state::<AudioEngine>().stop_all() {
                    let _ = app.emit(events::AUDIO_ERROR, &err);
                }
            }
            ID_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                super::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.build(app)?;
    Ok(())
}

pub fn update_language<R: Runtime>(app: &AppHandle<R>, lang: Language) {
    if let Some(tray) = app.tray_by_id(TRAY_ID)
        && let Ok(menu) = build_menu(app, lang)
    {
        let _ = tray.set_menu(Some(menu));
    }
}
