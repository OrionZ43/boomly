//! Tauri-команды аудио (участок B). Имена и сигнатуры — контракт (docs/contracts.md).
use tauri::State;

use super::types::*;
use super::AudioEngine;
use crate::error::{AppError, AppResult};
use crate::library::Library;

#[tauri::command]
pub fn audio_list_devices(engine: State<'_, AudioEngine>) -> AppResult<DeviceList> {
    engine.list_devices()
}

#[tauri::command]
pub fn audio_get_settings(engine: State<'_, AudioEngine>) -> AudioSettings {
    engine.settings()
}

#[tauri::command]
pub fn audio_set_devices(
    engine: State<'_, AudioEngine>,
    selection: DeviceSelection,
) -> AppResult<AudioSettings> {
    engine.set_devices(selection)
}

#[tauri::command]
pub fn audio_set_volume(
    engine: State<'_, AudioEngine>,
    channel: VolumeChannel,
    value: f32,
) -> AppResult<AudioSettings> {
    engine.set_volume(channel, value)
}

#[tauri::command]
pub fn audio_set_mic_muted(engine: State<'_, AudioEngine>, muted: bool) -> AppResult<AudioSettings> {
    engine.set_mic_muted(muted)
}

#[tauri::command]
pub fn audio_set_mic_passthrough(
    engine: State<'_, AudioEngine>,
    enabled: bool,
) -> AppResult<AudioSettings> {
    engine.set_mic_passthrough(enabled)
}

#[tauri::command]
pub fn audio_cable_status(engine: State<'_, AudioEngine>) -> CableStatus {
    engine.cable_status()
}

#[tauri::command]
pub fn audio_play(
    engine: State<'_, AudioEngine>,
    library: State<'_, Library>,
    sound_id: String,
) -> AppResult<()> {
    let sound = library
        .sound(&sound_id)
        .ok_or_else(|| AppError::new("not_found", format!("звук {sound_id} не найден")))?;
    engine.play(&sound)
}

#[tauri::command]
pub fn audio_stop(engine: State<'_, AudioEngine>, sound_id: String) -> AppResult<()> {
    engine.stop(&sound_id)
}

#[tauri::command]
pub fn audio_stop_all(engine: State<'_, AudioEngine>) -> AppResult<()> {
    engine.stop_all()
}

#[tauri::command]
pub fn audio_play_test(engine: State<'_, AudioEngine>, target: TestTarget) -> AppResult<()> {
    engine.play_test(target)
}
