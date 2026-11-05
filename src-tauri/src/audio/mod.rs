//! Участок B — аудиодвижок: декодирование (symphonia), микшер, два выхода cpal
//! («себе» и VB-CABLE), сквозной микрофон, детект VB-CABLE, хот-плаг.
//!
//! Сейчас — заглушка: хранит настройки в памяти, ничего не играет.
//! Внутренний API (`AudioEngine`) вызывают команды и хоткеи (B2) — сигнатуры публичных
//! методов — часть контракта, менять через docs/contracts.md.

pub mod commands;
pub mod types;

use std::sync::Mutex;

use crate::error::{AppError, AppResult};
use crate::library::types::Sound;
use types::*;

#[derive(Default)]
pub struct AudioEngine {
    settings: Mutex<AudioSettings>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list_devices(&self) -> AppResult<DeviceList> {
        Ok(DeviceList::default())
    }

    pub fn settings(&self) -> AudioSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set_devices(&self, sel: DeviceSelection) -> AppResult<AudioSettings> {
        let mut s = self.settings.lock().unwrap();
        if let Some(v) = sel.monitor_device_id {
            s.monitor_device_id = v;
        }
        if let Some(v) = sel.cable_device_id {
            s.cable_device_id = v;
        }
        if let Some(v) = sel.mic_device_id {
            s.mic_device_id = v;
        }
        Ok(s.clone())
    }

    pub fn set_volume(&self, channel: VolumeChannel, value: f32) -> AppResult<AudioSettings> {
        if !(0.0..=1.0).contains(&value) {
            return Err(AppError::new("invalid_argument", "громкость вне 0..1"));
        }
        let mut s = self.settings.lock().unwrap();
        match channel {
            VolumeChannel::Monitor => s.monitor_volume = value,
            VolumeChannel::Cable => s.cable_volume = value,
            VolumeChannel::Mic => s.mic_volume = value,
        }
        Ok(s.clone())
    }

    pub fn set_mic_muted(&self, muted: bool) -> AppResult<AudioSettings> {
        let mut s = self.settings.lock().unwrap();
        s.mic_muted = muted;
        Ok(s.clone())
    }

    pub fn set_mic_passthrough(&self, enabled: bool) -> AppResult<AudioSettings> {
        let mut s = self.settings.lock().unwrap();
        s.mic_passthrough = enabled;
        Ok(s.clone())
    }

    pub fn cable_status(&self) -> CableStatus {
        CableStatus {
            installed: false,
            output_device_name: None,
            input_device_name: None,
            download_url: VB_CABLE_URL.into(),
        }
    }

    /// Играть звук из библиотеки. Режим (toggle/restart/overlay) и громкость берутся из `Sound`.
    pub fn play(&self, _sound: &Sound) -> AppResult<()> {
        Err(AppError::not_implemented("audio.play"))
    }

    pub fn stop(&self, _sound_id: &str) -> AppResult<()> {
        Ok(())
    }

    pub fn stop_all(&self) -> AppResult<()> {
        Ok(())
    }

    pub fn play_test(&self, _target: TestTarget) -> AppResult<()> {
        Err(AppError::not_implemented("audio.play_test"))
    }
}
