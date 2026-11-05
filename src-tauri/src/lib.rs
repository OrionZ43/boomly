//! Точка сборки Boomly. Владелец: A (ведущий инженер).
//!
//! Участки добавляют свои плагины в `<модуль>::init`, а новые команды — строкой в
//! `generate_handler!` ниже, в своём блоке. Остальное в этом файле — через A.

pub mod audio;
pub mod error;
pub mod events;
pub mod hotkeys;
pub mod library;
pub mod paths;
pub mod system;
pub mod updater;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();
    // single-instance должен регистрироваться первым — B2 делает это в system::init.
    builder = system::init(builder);
    builder = hotkeys::init(builder);
    builder = updater::init(builder);

    builder
        .plugin(tauri_plugin_opener::init())
        .manage(audio::AudioEngine::new())
        .manage(library::Library::new())
        .manage(hotkeys::Hotkeys::new())
        .manage(system::SettingsStore::new())
        .invoke_handler(tauri::generate_handler![
            // --- B: audio ---
            audio::commands::audio_list_devices,
            audio::commands::audio_get_settings,
            audio::commands::audio_set_devices,
            audio::commands::audio_set_volume,
            audio::commands::audio_set_mic_muted,
            audio::commands::audio_set_mic_passthrough,
            audio::commands::audio_cable_status,
            audio::commands::audio_play,
            audio::commands::audio_stop,
            audio::commands::audio_stop_all,
            audio::commands::audio_play_test,
            // --- B2: library ---
            library::commands::library_get,
            library::commands::library_import,
            library::commands::library_update_sound,
            library::commands::library_delete_sounds,
            library::commands::library_create_board,
            library::commands::library_update_board,
            library::commands::library_delete_board,
            library::commands::library_reorder_boards,
            // --- B2: hotkeys ---
            hotkeys::commands::hotkeys_get,
            hotkeys::commands::hotkeys_set,
            hotkeys::commands::hotkeys_remove,
            hotkeys::commands::hotkeys_set_capture,
            // --- B2: system ---
            system::commands::app_get_settings,
            system::commands::app_set_settings,
            system::commands::app_info,
            system::commands::app_open_data_dir,
            // --- E: updater (если понадобятся свои команды) ---
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod contract_tests {
    //! Проверяем, что JSON-форма типов совпадает с docs/contracts.md.
    use super::*;
    use serde_json::json;

    #[test]
    fn hotkey_action_shape() {
        let a = hotkeys::types::HotkeyAction::PlaySound { sound_id: "x".into() };
        assert_eq!(serde_json::to_value(&a).unwrap(), json!({"type": "playSound", "soundId": "x"}));
        assert_eq!(
            serde_json::to_value(hotkeys::types::HotkeyAction::StopAll).unwrap(),
            json!({"type": "stopAll"})
        );
    }

    #[test]
    fn device_selection_null_vs_missing() {
        let s: audio::types::DeviceSelection =
            serde_json::from_value(json!({"cableDeviceId": null, "micDeviceId": "m"})).unwrap();
        assert_eq!(s.monitor_device_id, None);
        assert_eq!(s.cable_device_id, Some(None));
        assert_eq!(s.mic_device_id, Some(Some("m".into())));
    }

    #[test]
    fn settings_camel_case() {
        let v = serde_json::to_value(audio::types::AudioSettings::default()).unwrap();
        assert!(v.get("micPassthrough").is_some());
        let v = serde_json::to_value(system::types::AppSettings::default()).unwrap();
        assert_eq!(v["language"], "ru");
        assert_eq!(v["effects"], "normal");
    }
}
