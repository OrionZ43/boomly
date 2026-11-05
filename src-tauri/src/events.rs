//! Имена событий Rust → UI. Владелец: A (контракт). Полезная нагрузка — в docs/contracts.md
//! и в типах участков. Зеркало в TS: `src/lib/api/events.ts`.

/// `PlaybackEvent` — звук начал/закончил играть.
pub const AUDIO_PLAYBACK: &str = "audio://playback";
/// `ProgressEvent[]` — позиции играющих звуков, не чаще ~15 раз/с.
pub const AUDIO_PROGRESS: &str = "audio://progress";
/// `DeviceList` — список устройств изменился (подключили/отключили).
pub const AUDIO_DEVICES_CHANGED: &str = "audio://devices-changed";
/// `CableStatus` — изменилось состояние VB-CABLE.
pub const AUDIO_CABLE_STATUS: &str = "audio://cable-status";
/// `MicLevel` — уровень сквозного микрофона, ~15 раз/с, только пока включён.
pub const AUDIO_MIC_LEVEL: &str = "audio://mic-level";
/// `AudioSettings` — настройки звука изменились (в т.ч. из хоткея/трея).
pub const AUDIO_SETTINGS_CHANGED: &str = "audio://settings-changed";
/// `AppError` — фоновая ошибка движка (устройство пропало и т.п.).
pub const AUDIO_ERROR: &str = "audio://error";

/// `Library` — библиотека изменилась (целиком, она небольшая).
pub const LIBRARY_CHANGED: &str = "library://changed";

/// `HotkeyTriggered` — сработал глобальный хоткей (действие уже выполнено в Rust).
pub const HOTKEYS_TRIGGERED: &str = "hotkeys://triggered";

/// `AppSettings` — изменились настройки приложения.
pub const APP_SETTINGS_CHANGED: &str = "app://settings-changed";
