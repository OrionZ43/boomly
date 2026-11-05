//! Типы контракта аудио. Менять только вместе с docs/contracts.md и src/lib/api/types.ts.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    /// Стабильный id (для WASAPI — имя устройства; движок решает сам, UI считает непрозрачным).
    pub id: String,
    pub name: String,
    pub is_default: bool,
    /// Это VB-CABLE (`CABLE Input` для выходов, `CABLE Output` для входов).
    pub is_cable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceList {
    pub outputs: Vec<AudioDevice>,
    pub inputs: Vec<AudioDevice>,
}

/// Регулятор громкости.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VolumeChannel {
    /// «Я слышу» — наушники/колонки.
    Monitor,
    /// «В микрофон» — звуки в VB-CABLE.
    Cable,
    /// «Голос» — сквозной микрофон в VB-CABLE.
    Mic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSettings {
    /// `None` — системное устройство по умолчанию.
    pub monitor_device_id: Option<String>,
    /// `None` — авто: первый найденный VB-CABLE.
    pub cable_device_id: Option<String>,
    /// Реальный микрофон для сквозного режима; `None` — по умолчанию.
    pub mic_device_id: Option<String>,
    /// 0.0..=1.0
    pub monitor_volume: f32,
    pub cable_volume: f32,
    pub mic_volume: f32,
    pub mic_muted: bool,
    /// Сквозной микрофон включён (голос подмешивается в VB-CABLE).
    pub mic_passthrough: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            monitor_device_id: None,
            cable_device_id: None,
            mic_device_id: None,
            monitor_volume: 0.8,
            cable_volume: 0.8,
            mic_volume: 1.0,
            mic_muted: false,
            mic_passthrough: true,
        }
    }
}

/// Частичное обновление выбора устройств: поле отсутствует — не менять;
/// `Some(None)` в JSON задаётся как `null` — «по умолчанию».
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSelection {
    #[serde(default, with = "serde_double_option")]
    pub monitor_device_id: Option<Option<String>>,
    #[serde(default, with = "serde_double_option")]
    pub cable_device_id: Option<Option<String>>,
    #[serde(default, with = "serde_double_option")]
    pub mic_device_id: Option<Option<String>>,
}

mod serde_double_option {
    use serde::{Deserialize, Deserializer};
    pub fn deserialize<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Option::<T>::deserialize(d).map(Some)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CableStatus {
    /// Найдены и `CABLE Input`, и `CABLE Output`.
    pub installed: bool,
    pub output_device_name: Option<String>,
    pub input_device_name: Option<String>,
    pub download_url: String,
}

/// Куда играть тестовый звук в мастере.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TestTarget {
    Monitor,
    Cable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackState {
    Started,
    /// Остановлен пользователем/«Стоп всё».
    Stopped,
    /// Доиграл до конца.
    Finished,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackEvent {
    pub sound_id: String,
    pub state: PlaybackState,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub sound_id: String,
    pub position_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MicLevel {
    /// Пиковый уровень 0.0..=1.0 после громкости «Голос».
    pub peak: f32,
}

pub const VB_CABLE_URL: &str = "https://vb-audio.com/Cable/";
