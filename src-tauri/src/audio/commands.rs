//! Tauri-команды аудио (участок B). Имена и сигнатуры — контракт (docs/contracts.md).
//! Все команды быстрые (тяжёлое — в потоках движка), поэтому синхронные.
//! `AppHandle` нужен только для ленивого старта движка, если `audio::init` не успел.
use tauri::{AppHandle, State};

use super::AudioEngine;
use super::types::*;
use crate::error::{AppError, AppResult};
use crate::library::Library;

fn engine<'a>(app: &AppHandle, engine: State<'a, AudioEngine>) -> State<'a, AudioEngine> {
    engine.start(app.clone());
    engine
}

/// async: до первого опроса движок перечисляет устройства сам (десятки мс) — не на главном потоке.
#[tauri::command]
pub async fn audio_list_devices(
    app: AppHandle,
    engine: State<'_, AudioEngine>,
) -> AppResult<DeviceList> {
    self::engine(&app, engine).list_devices()
}

#[tauri::command]
pub fn audio_get_settings(app: AppHandle, engine: State<'_, AudioEngine>) -> AudioSettings {
    self::engine(&app, engine).settings()
}

#[tauri::command]
pub fn audio_set_devices(
    app: AppHandle,
    engine: State<'_, AudioEngine>,
    selection: DeviceSelection,
) -> AppResult<AudioSettings> {
    self::engine(&app, engine).set_devices(selection)
}

#[tauri::command]
pub fn audio_set_volume(
    app: AppHandle,
    engine: State<'_, AudioEngine>,
    channel: VolumeChannel,
    value: f32,
) -> AppResult<AudioSettings> {
    self::engine(&app, engine).set_volume(channel, value)
}

#[tauri::command]
pub fn audio_set_mic_muted(
    app: AppHandle,
    engine: State<'_, AudioEngine>,
    muted: bool,
) -> AppResult<AudioSettings> {
    self::engine(&app, engine).set_mic_muted(muted)
}

#[tauri::command]
pub fn audio_set_mic_passthrough(
    app: AppHandle,
    engine: State<'_, AudioEngine>,
    enabled: bool,
) -> AppResult<AudioSettings> {
    self::engine(&app, engine).set_mic_passthrough(enabled)
}

#[tauri::command]
pub async fn audio_cable_status(
    app: AppHandle,
    engine: State<'_, AudioEngine>,
) -> AppResult<CableStatus> {
    Ok(self::engine(&app, engine).cable_status())
}

#[tauri::command]
pub fn audio_play(
    app: AppHandle,
    engine: State<'_, AudioEngine>,
    library: State<'_, Library>,
    sound_id: String,
) -> AppResult<()> {
    let sound = library
        .sound(&sound_id)
        .ok_or_else(|| AppError::new("not_found", format!("звук {sound_id} не найден")))?;
    self::engine(&app, engine).play(&sound)
}

#[tauri::command]
pub fn audio_stop(
    app: AppHandle,
    engine: State<'_, AudioEngine>,
    sound_id: String,
) -> AppResult<()> {
    self::engine(&app, engine).stop(&sound_id)
}

#[tauri::command]
pub fn audio_stop_all(app: AppHandle, engine: State<'_, AudioEngine>) -> AppResult<()> {
    self::engine(&app, engine).stop_all()
}

#[tauri::command]
pub fn audio_play_test(
    app: AppHandle,
    engine: State<'_, AudioEngine>,
    target: TestTarget,
) -> AppResult<()> {
    self::engine(&app, engine).play_test(target)
}
