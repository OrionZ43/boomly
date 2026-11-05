//! Участок E — автообновление.
//!
//! Схема: `tauri-plugin-updater` качает `latest.json` из GitHub Releases
//! (`releases/latest/download/latest.json`, см. `plugins.updater` в tauri.conf.json),
//! проверяет подпись Ed25519 (minisign) публичным ключом из конфига, качает NSIS-установщик
//! и запускает его в режиме `passive`. Перезапуск — `tauri-plugin-process`.
//!
//! Логика (когда проверять, баннер, прогресс) живёт в UI: `src/lib/api/updater.ts` и
//! `src/lib/updater/**`. Rust даёт только команды плагинов
//! (разрешения — `src-tauri/capabilities/updater.json`) и `updater_enabled`.

pub mod commands;

/// Публичный ключ из `plugins.updater.pubkey`. Пока пользователь не вписал ключ,
/// там пустая строка — тогда UI считает автообновление выключенным.
pub fn pubkey_configured(config: &tauri::Config) -> bool {
    config
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.trim().is_empty())
}

/// Регистрирует плагины участка E. Вызывается из `lib.rs`.
pub fn init<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(updater: serde_json::Value) -> tauri::Config {
        let mut c = tauri::Config::default();
        c.plugins.0.insert("updater".into(), updater);
        c
    }

    #[test]
    fn empty_pubkey_means_disabled() {
        assert!(!pubkey_configured(&config(serde_json::json!({"pubkey": ""}))));
        assert!(!pubkey_configured(&config(serde_json::json!({}))));
        assert!(pubkey_configured(&config(serde_json::json!({"pubkey": "dW50cnVzdGVk"}))));
    }

    /// Конфиг в репозитории: endpoint на GitHub Releases, identifier не менялся, NSIS per-user.
    #[test]
    fn shipped_config_is_sane() {
        let raw: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        assert_eq!(raw["identifier"], "com.orionz43.boomly");
        assert_eq!(
            raw["plugins"]["updater"]["endpoints"][0],
            "https://github.com/OrionZ43/boomly/releases/latest/download/latest.json"
        );
        assert_eq!(raw["bundle"]["windows"]["nsis"]["installMode"], "currentUser");
        assert_eq!(raw["bundle"]["createUpdaterArtifacts"], true);
    }
}
