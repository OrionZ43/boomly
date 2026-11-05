//! Типы контракта хоткеев. Менять только вместе с docs/contracts.md и src/lib/api/types.ts.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HotkeyAction {
    PlaySound { sound_id: String },
    StopAll,
    ToggleMicMute,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBinding {
    /// Формат tauri-plugin-global-shortcut: `Ctrl+Shift+F1`, `Alt+Numpad3`.
    pub accelerator: String,
    pub action: HotkeyAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyTriggered {
    pub accelerator: String,
    pub action: HotkeyAction,
}
