//! Участок B — аудиодвижок: декодирование (symphonia) и кэш, микшер, два выхода cpal/WASAPI
//! («себе» и VB-CABLE), сквозной микрофон, детект VB-CABLE, хот-плаг.
//! Устройство и решения описаны в `docs/audio.md`.
//!
//! Потоки:
//! - **аудиоколбэки cpal** (по одному на выход и на микрофон) — только `mixer::Renderer` /
//!   `mic::MicWriter`, без блокировок и аллокаций;
//! - **поток устройств** — владеет стримами, раз в 2 с опрашивает список устройств
//!   (хот-плаг, смена устройства по умолчанию), пересобирает стримы по ошибке устройства;
//! - **тикер** (~15 Гц) — освобождает память из аудиопотоков, ловит доигравшие звуки,
//!   шлёт `audio://progress` и `audio://mic-level`;
//! - **декодер** — декодирует при промахе кэша (play не ждёт) и предзагружает библиотеку.
//!
//! Публичные методы `AudioEngine` — контракт (их зовут команды и хоткеи B2): все быстрые,
//! тяжёлая работа уходит в потоки движка.

pub mod cache;
pub mod commands;
pub mod decode;
pub mod devices;
pub mod dsp;
pub mod mic;
pub mod mixer;
pub mod types;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod hw_tests;

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::Duration;

use cpal::traits::DeviceTrait;
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::events;
use crate::library::types::{PlayMode, Sound};
use cache::ClipCache;
use decode::Clip;
use devices::{OpenStream, Resolved, Snapshot};
use mic::MicStats;
use mixer::{CABLE, MONITOR, OutputControl, OutputParams, RtCmd, RtVoice, VoiceShared};
use types::*;

pub use decode::probe_duration_ms;

/// Куда движок отправляет события (в приложении — `AppHandle::emit`, в тестах — вектор).
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: &str, payload: serde_json::Value);
}

struct TauriSink<R: tauri::Runtime>(tauri::AppHandle<R>);

impl<R: tauri::Runtime> EventSink for TauriSink<R> {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        use tauri::Emitter;
        let _ = self.0.emit(event, payload);
    }
}

/// Регистрация движка в приложении: маленький плагин со своим `setup`
/// (не трогаем общий `Builder::setup`). Порядок относительно B2 не важен:
/// библиотека подхватывается по событию `library://changed`.
pub fn init<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.plugin(
        tauri::plugin::Builder::<R, ()>::new("boomly-audio")
            .setup(|app, _api| {
                use tauri::Manager;
                if let Some(engine) = app.try_state::<AudioEngine>() {
                    engine.start(app.clone());
                }
                Ok(())
            })
            .build(),
    )
}

const TICK: Duration = Duration::from_millis(66);
const POLL: Duration = Duration::from_secs(2);
/// Служебный id для тестового звука мастера (события по нему не шлём).
const TEST_SOUND_ID: &str = "__boomly_test__";

struct Active {
    instance: u64,
    sound_id: String,
    duration_ms: u64,
    shared: Arc<VoiceShared>,
    /// Остановлен пользователем (событие уже отправлено), ждём освобождения.
    stopped: bool,
    /// Служебный звук (тест мастера) — без событий.
    silent: bool,
}

enum Job {
    Play { sound: Sound, token: u64 },
    Preload { path: PathBuf },
}

#[derive(Default)]
struct JobQueue {
    q: Mutex<VecDeque<Job>>,
    cv: Condvar,
}

impl JobQueue {
    fn push_front(&self, j: Job) {
        self.q.lock().unwrap().push_front(j);
        self.cv.notify_one();
    }
    fn push_back(&self, j: Job) {
        self.q.lock().unwrap().push_back(j);
        self.cv.notify_one();
    }
    fn pop(&self) -> Job {
        let mut q = self.q.lock().unwrap();
        loop {
            if let Some(j) = q.pop_front() {
                return j;
            }
            q = self.cv.wait(q).unwrap();
        }
    }
}

enum DevMsg {
    /// Пересобрать всё (смена устройств в настройках).
    Rebuild,
    /// Включить/выключить сквозной микрофон.
    RebuildMic,
    /// Ошибка потока (устройство пропало / сменилось).
    StreamError(String),
}

/// Диагностика открытых потоков: для отчёта о задержке.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamReport {
    pub device: String,
    pub rate: u32,
    pub channels: u16,
    pub period_frames: Option<u32>,
    pub callback_frames: u64,
    pub latency_us: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyReport {
    pub monitor: Option<StreamReport>,
    pub cable: Option<StreamReport>,
    pub mic: Option<StreamReport>,
    /// Средняя задержка кольца сквозного микрофона, мкс.
    pub mic_buffer_us: Option<u64>,
    pub mic_underruns: Option<u64>,
}

/// Открытый поток для отчёта: имя устройства, счётчики, частота, каналы, период.
type StreamInfo = (String, Arc<devices::StreamStats>, u32, u16, Option<u32>);

struct Inner {
    settings: Mutex<AudioSettings>,
    settings_path: Mutex<Option<PathBuf>>,
    params: [Arc<OutputParams>; 2],
    outputs: Mutex<[Option<OutputControl>; 2]>,
    active: Mutex<Vec<Active>>,
    /// Звуки, ждущие декодирования: sound_id → токен (отмена = удаление).
    pending: Mutex<HashMap<String, u64>>,
    cache: ClipCache,
    sink: RwLock<Option<Arc<dyn EventSink>>>,
    devices: Mutex<DeviceList>,
    devices_ready: AtomicBool,
    cable: Mutex<CableStatus>,
    mic_stats: Mutex<Option<Arc<MicStats>>>,
    stream_reports: Mutex<[Option<StreamInfo>; 3]>,
    dev_tx: Mutex<Option<mpsc::Sender<DevMsg>>>,
    jobs: JobQueue,
    library_paths: Mutex<Vec<PathBuf>>,
    next_id: AtomicU64,
    started: AtomicBool,
}

pub struct AudioEngine {
    inner: Arc<Inner>,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn to_json<T: Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

impl AudioEngine {
    pub fn new() -> Self {
        let s = AudioSettings::default();
        let params = [
            OutputParams::new(s.monitor_volume),
            OutputParams::new(s.cable_volume),
        ];
        let inner = Inner {
            params,
            settings: Mutex::new(s),
            settings_path: Mutex::new(None),
            outputs: Mutex::new([None, None]),
            active: Mutex::new(Vec::new()),
            pending: Mutex::new(HashMap::new()),
            cache: ClipCache::default(),
            sink: RwLock::new(None),
            devices: Mutex::new(DeviceList::default()),
            devices_ready: AtomicBool::new(false),
            cable: Mutex::new(devices::cable_status(&DeviceList::default())),
            mic_stats: Mutex::new(None),
            stream_reports: Mutex::new([None, None, None]),
            dev_tx: Mutex::new(None),
            jobs: JobQueue::default(),
            library_paths: Mutex::new(Vec::new()),
            next_id: AtomicU64::new(1),
            started: AtomicBool::new(false),
        };
        let e = Self {
            inner: Arc::new(inner),
        };
        e.inner.apply_params();
        e
    }

    /// Запустить движок: загрузить настройки, открыть устройства, запустить потоки.
    /// Повторный вызов ничего не делает (его зовут и `init`, и первая команда).
    pub fn start<R: tauri::Runtime>(&self, app: tauri::AppHandle<R>) {
        if self.inner.started.swap(true, Ordering::SeqCst) {
            return;
        }
        use tauri::{Listener, Manager};
        *self.inner.sink.write().unwrap() = Some(Arc::new(TauriSink(app.clone())));
        if let Ok(dir) = crate::paths::data_dir(&app) {
            let path = dir.join("audio.json");
            self.inner.load_settings(&path);
            *self.inner.settings_path.lock().unwrap() = Some(path);
        }
        // библиотека: что уже есть + все изменения
        if let Some(lib) = app.try_state::<crate::library::Library>() {
            self.inner.set_library(&lib.snapshot().sounds);
        }
        let inner = self.inner.clone();
        app.listen(events::LIBRARY_CHANGED, move |ev| {
            if let Ok(data) =
                serde_json::from_str::<crate::library::types::LibraryData>(ev.payload())
            {
                inner.set_library(&data.sounds);
            }
        });
        self.spawn_threads(true);
    }

    fn spawn_threads(&self, with_devices: bool) {
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("boomly-audio-tick".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(TICK);
                    inner.tick();
                }
            })
            .ok();
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("boomly-audio-decode".into())
            .spawn(move || {
                loop {
                    let job = inner.jobs.pop();
                    inner.run_job(job);
                }
            })
            .ok();
        if with_devices {
            let (tx, rx) = mpsc::channel();
            *self.inner.dev_tx.lock().unwrap() = Some(tx.clone());
            let inner = self.inner.clone();
            std::thread::Builder::new()
                .name("boomly-audio-devices".into())
                .spawn(move || DeviceThread::new(inner, tx).run(rx))
                .ok();
        }
    }

    pub fn list_devices(&self) -> AppResult<DeviceList> {
        if self.inner.devices_ready.load(Ordering::SeqCst) {
            return Ok(self.inner.devices.lock().unwrap().clone());
        }
        // движок ещё не опросил устройства — перечислим сами
        Ok(devices::enumerate(&cpal::default_host()).list())
    }

    pub fn settings(&self) -> AudioSettings {
        self.inner.settings.lock().unwrap().clone()
    }

    pub fn set_devices(&self, sel: DeviceSelection) -> AppResult<AudioSettings> {
        let s = self.inner.update_settings(|s| {
            if let Some(v) = sel.monitor_device_id {
                s.monitor_device_id = v;
            }
            if let Some(v) = sel.cable_device_id {
                s.cable_device_id = v;
            }
            if let Some(v) = sel.mic_device_id {
                s.mic_device_id = v;
            }
        });
        self.inner.send_dev(DevMsg::Rebuild);
        Ok(s)
    }

    pub fn set_volume(&self, channel: VolumeChannel, value: f32) -> AppResult<AudioSettings> {
        if !(0.0..=1.0).contains(&value) {
            return Err(AppError::new("invalid_argument", "громкость вне 0..1"));
        }
        Ok(self.inner.update_settings(|s| match channel {
            VolumeChannel::Monitor => s.monitor_volume = value,
            VolumeChannel::Cable => s.cable_volume = value,
            VolumeChannel::Mic => s.mic_volume = value,
        }))
    }

    pub fn set_mic_muted(&self, muted: bool) -> AppResult<AudioSettings> {
        Ok(self.inner.update_settings(|s| s.mic_muted = muted))
    }

    /// Переключить mute голоса (для хоткея `toggleMicMute`).
    pub fn toggle_mic_muted(&self) -> AppResult<AudioSettings> {
        Ok(self.inner.update_settings(|s| s.mic_muted = !s.mic_muted))
    }

    pub fn set_mic_passthrough(&self, enabled: bool) -> AppResult<AudioSettings> {
        let s = self.inner.update_settings(|s| s.mic_passthrough = enabled);
        self.inner.send_dev(DevMsg::RebuildMic);
        Ok(s)
    }

    pub fn cable_status(&self) -> CableStatus {
        if !self.inner.devices_ready.load(Ordering::SeqCst) {
            return devices::cable_status(&self.list_devices().unwrap_or_default());
        }
        self.inner.cable.lock().unwrap().clone()
    }

    /// Играть звук из библиотеки. Режим (toggle/restart/overlay) и громкость берутся из `Sound`.
    /// Возвращается сразу: из кэша звук стартует в этом же вызове, иначе — после декодирования
    /// в потоке движка (событие `started` или `error`).
    pub fn play(&self, sound: &Sound) -> AppResult<()> {
        self.inner.play(sound)
    }

    pub fn stop(&self, sound_id: &str) -> AppResult<()> {
        self.inner
            .stop_where(|a| a.sound_id == sound_id && !a.silent);
        if self
            .inner
            .pending
            .lock()
            .unwrap()
            .remove(sound_id)
            .is_some()
        {
            self.inner
                .emit_playback(sound_id, PlaybackState::Stopped, None);
        }
        Ok(())
    }

    pub fn stop_all(&self) -> AppResult<()> {
        self.inner.stop_where(|_| true);
        let pending: Vec<String> = self
            .inner
            .pending
            .lock()
            .unwrap()
            .drain()
            .map(|(k, _)| k)
            .collect();
        for id in pending {
            self.inner.emit_playback(&id, PlaybackState::Stopped, None);
        }
        Ok(())
    }

    pub fn play_test(&self, target: TestTarget) -> AppResult<()> {
        let idx = match target {
            TestTarget::Monitor => MONITOR,
            TestTarget::Cable => CABLE,
        };
        let rate = self.inner.outputs.lock().unwrap()[idx]
            .as_ref()
            .map(|c| c.rate);
        let Some(rate) = rate else {
            return Err(match target {
                TestTarget::Cable if !self.cable_status().installed => {
                    AppError::new("cable_missing", "VB-CABLE не найден")
                }
                _ => AppError::new("device_unavailable", "устройство вывода не открыто"),
            });
        };
        self.inner.stop_where(|a| a.silent);
        let clip = Arc::new(decode::test_chime(rate));
        self.inner
            .start_voice(TEST_SOUND_ID, vec![(idx, clip)], 1.0, true)
    }

    /// Задержки открытых потоков (для диагностики и отчёта).
    pub fn latency_report(&self) -> LatencyReport {
        let reps = self.inner.stream_reports.lock().unwrap();
        let mk = |i: usize| {
            reps[i]
                .as_ref()
                .map(|(dev, st, rate, ch, period)| StreamReport {
                    device: dev.clone(),
                    rate: *rate,
                    channels: *ch,
                    period_frames: *period,
                    callback_frames: st.callback_frames.load(Ordering::Relaxed),
                    latency_us: st.latency_us.load(Ordering::Relaxed),
                })
        };
        let ms = self.inner.mic_stats.lock().unwrap().clone();
        LatencyReport {
            monitor: mk(0),
            cable: mk(1),
            mic: mk(2),
            mic_buffer_us: ms.as_ref().map(|s| s.buffer_us.load(Ordering::Relaxed)),
            mic_underruns: ms.as_ref().map(|s| s.underruns.load(Ordering::Relaxed)),
        }
    }
}

impl Inner {
    fn sink(&self) -> Option<Arc<dyn EventSink>> {
        self.sink.read().unwrap().clone()
    }

    fn emit<T: Serialize>(&self, event: &str, payload: &T) {
        if let Some(s) = self.sink() {
            s.emit(event, to_json(payload));
        }
    }

    fn emit_playback(&self, sound_id: &str, state: PlaybackState, duration_ms: Option<u64>) {
        if sound_id == TEST_SOUND_ID {
            return;
        }
        self.emit(
            events::AUDIO_PLAYBACK,
            &PlaybackEvent {
                sound_id: sound_id.into(),
                state,
                duration_ms,
            },
        );
    }

    fn emit_error(&self, err: &AppError) {
        self.emit(events::AUDIO_ERROR, err);
    }

    fn send_dev(&self, msg: DevMsg) {
        if let Some(tx) = self.dev_tx.lock().unwrap().as_ref() {
            let _ = tx.send(msg);
        }
    }

    // ---------- настройки ----------

    fn apply_params(&self) {
        let s = self.settings.lock().unwrap().clone();
        self.params[MONITOR].volume.set(s.monitor_volume);
        self.params[CABLE].volume.set(s.cable_volume);
        self.params[CABLE].mic_volume.set(s.mic_volume);
        self.params[CABLE]
            .mic_muted
            .store(s.mic_muted, Ordering::Relaxed);
    }

    fn update_settings(&self, f: impl FnOnce(&mut AudioSettings)) -> AudioSettings {
        let (changed, s) = {
            let mut s = self.settings.lock().unwrap();
            let before = s.clone();
            f(&mut s);
            (*s != before, s.clone())
        };
        self.apply_params();
        if changed {
            self.save_settings(&s);
            self.emit(events::AUDIO_SETTINGS_CHANGED, &s);
        }
        s
    }

    fn load_settings(&self, path: &std::path::Path) {
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        // поля, которых нет в файле, — по умолчанию
        let def = to_json(&AudioSettings::default());
        let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&text) else {
            return;
        };
        if let (Some(obj), Some(d)) = (v.as_object_mut(), def.as_object()) {
            for (k, val) in d {
                obj.entry(k.clone()).or_insert(val.clone());
            }
        }
        if let Ok(mut s) = serde_json::from_value::<AudioSettings>(v) {
            s.monitor_volume = s.monitor_volume.clamp(0.0, 1.0);
            s.cable_volume = s.cable_volume.clamp(0.0, 1.0);
            s.mic_volume = s.mic_volume.clamp(0.0, 1.0);
            *self.settings.lock().unwrap() = s;
            self.apply_params();
        }
    }

    fn save_settings(&self, s: &AudioSettings) {
        let Some(path) = self.settings_path.lock().unwrap().clone() else {
            return;
        };
        let Ok(text) = serde_json::to_string_pretty(s) else {
            return;
        };
        // атомарно: пишем рядом и переименовываем
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    // ---------- воспроизведение ----------

    fn output_rates(&self) -> Vec<(usize, u32)> {
        let outs = self.outputs.lock().unwrap();
        outs.iter()
            .enumerate()
            .filter_map(|(i, c)| c.as_ref().map(|c| (i, c.rate)))
            .collect()
    }

    fn play(&self, sound: &Sound) -> AppResult<()> {
        let mode = sound.mode;
        // повторный запуск того же звука
        let playing: Vec<u64> = self
            .active
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.sound_id == sound.id && !a.stopped && !a.silent)
            .map(|a| a.instance)
            .collect();
        let was_pending = self.pending.lock().unwrap().contains_key(&sound.id);
        if (!playing.is_empty() || was_pending) && mode != PlayMode::Overlay {
            self.stop_where(|a| playing.contains(&a.instance));
            if was_pending {
                self.pending.lock().unwrap().remove(&sound.id);
                if playing.is_empty() {
                    self.emit_playback(&sound.id, PlaybackState::Stopped, None);
                }
            }
            if mode == PlayMode::Toggle {
                return Ok(());
            }
        }

        let rates = self.output_rates();
        if rates.is_empty() {
            return Err(AppError::new(
                "device_unavailable",
                "нет открытых устройств вывода",
            ));
        }
        let path = PathBuf::from(&sound.path);
        let only_rates: Vec<u32> = rates.iter().map(|r| r.1).collect();
        if let Some(clips) = self.cache.try_get(&path, &only_rates) {
            let routed = rates.iter().map(|r| r.0).zip(clips).collect();
            return self.start_voice(&sound.id, routed, sound.volume, false);
        }
        // промах кэша: декодируем в потоке движка, вызов не ждёт
        let token = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.pending.lock().unwrap().insert(sound.id.clone(), token);
        self.jobs.push_front(Job::Play {
            sound: sound.clone(),
            token,
        });
        Ok(())
    }

    fn start_voice(
        &self,
        sound_id: &str,
        clips: Vec<(usize, Arc<Clip>)>,
        gain: f32,
        silent: bool,
    ) -> AppResult<()> {
        let gain = if gain.is_finite() {
            gain.clamp(0.0, 2.0)
        } else {
            1.0
        };
        let shared = Arc::new(VoiceShared::default());
        let instance = self.next_id.fetch_add(1, Ordering::Relaxed);
        let duration_ms = clips.first().map(|c| c.1.duration_ms()).unwrap_or(0);
        let mut sent = 0;
        {
            let mut outs = self.outputs.lock().unwrap();
            for (idx, clip) in clips {
                if let Some(ctl) = outs[idx].as_mut()
                    && ctl.rate == clip.rate
                    && ctl.send(RtCmd::Add(RtVoice::new(
                        instance,
                        shared.clone(),
                        clip,
                        gain,
                    )))
                {
                    sent += 1;
                }
            }
        }
        if sent == 0 {
            return Err(AppError::new(
                "device_unavailable",
                "устройство вывода недоступно",
            ));
        }
        self.active.lock().unwrap().push(Active {
            instance,
            sound_id: sound_id.into(),
            duration_ms,
            shared,
            stopped: false,
            silent,
        });
        self.emit_playback(sound_id, PlaybackState::Started, Some(duration_ms));
        Ok(())
    }

    /// Остановить экземпляры по условию; событие `stopped` — по одному на звук.
    fn stop_where(&self, pred: impl Fn(&Active) -> bool) {
        let mut ids = Vec::new();
        let mut stopped_sounds: Vec<String> = Vec::new();
        {
            let mut act = self.active.lock().unwrap();
            for a in act.iter_mut().filter(|a| !a.stopped && pred(a)) {
                a.stopped = true;
                ids.push(a.instance);
                if !a.silent && !stopped_sounds.contains(&a.sound_id) {
                    stopped_sounds.push(a.sound_id.clone());
                }
            }
        }
        if ids.is_empty() {
            return;
        }
        {
            let mut outs = self.outputs.lock().unwrap();
            for ctl in outs.iter_mut().flatten() {
                for &id in &ids {
                    ctl.send(RtCmd::Stop(id));
                }
            }
        }
        for s in stopped_sounds {
            self.emit_playback(&s, PlaybackState::Stopped, None);
        }
    }

    fn run_job(&self, job: Job) {
        match job {
            Job::Play { sound, token } => {
                let still = |p: &HashMap<String, u64>| p.get(&sound.id) == Some(&token);
                if !still(&self.pending.lock().unwrap()) {
                    return; // отменили, пока ждали
                }
                let rates = self.output_rates();
                let only: Vec<u32> = rates.iter().map(|r| r.1).collect();
                let res = self
                    .cache
                    .get_or_load(std::path::Path::new(&sound.path), &only);
                {
                    let mut p = self.pending.lock().unwrap();
                    if !still(&p) {
                        return;
                    }
                    p.remove(&sound.id);
                }
                let res = res.and_then(|clips| {
                    let routed = rates.iter().map(|r| r.0).zip(clips).collect();
                    self.start_voice(&sound.id, routed, sound.volume, false)
                });
                if let Err(e) = res {
                    self.emit_playback(&sound.id, PlaybackState::Error, None);
                    self.emit_error(&e);
                }
            }
            Job::Preload { path } => {
                let rates: Vec<u32> = self.output_rates().into_iter().map(|r| r.1).collect();
                if !rates.is_empty() {
                    let _ = self.cache.preload(&path, &rates);
                }
            }
        }
    }

    fn set_library(&self, sounds: &[Sound]) {
        let paths: Vec<PathBuf> = sounds
            .iter()
            .filter(|s| s.duration_ms.is_none_or(|d| d <= cache::MAX_CACHED_MS))
            .map(|s| PathBuf::from(&s.path))
            .collect();
        *self.library_paths.lock().unwrap() = paths;
        self.preload_library();
    }

    fn preload_library(&self) {
        for p in self.library_paths.lock().unwrap().iter() {
            self.jobs.push_back(Job::Preload { path: p.clone() });
        }
    }

    // ---------- тикер ----------

    fn tick(&self) {
        {
            let mut outs = self.outputs.lock().unwrap();
            for c in outs.iter_mut().flatten() {
                c.collect_garbage();
            }
        }
        let mut finished = Vec::new();
        let mut progress = Vec::new();
        {
            let mut act = self.active.lock().unwrap();
            act.retain(|a| {
                // голос больше нигде не живёт — доиграл (или выход закрыт)
                if Arc::strong_count(&a.shared) == 1 {
                    if !a.stopped && !a.silent {
                        finished.push((a.sound_id.clone(), a.duration_ms));
                    }
                    return false;
                }
                if !a.stopped && !a.silent {
                    let pos = a
                        .shared
                        .pos_ms
                        .iter()
                        .map(|p| p.load(Ordering::Relaxed))
                        .max();
                    progress.push(ProgressEvent {
                        sound_id: a.sound_id.clone(),
                        position_ms: pos.unwrap_or(0).min(a.duration_ms),
                        duration_ms: a.duration_ms,
                    });
                }
                true
            });
        }
        for (id, d) in finished {
            self.emit_playback(&id, PlaybackState::Finished, Some(d));
        }
        if !progress.is_empty() {
            self.emit(events::AUDIO_PROGRESS, &progress);
        }
        let passthrough = self.settings.lock().unwrap().mic_passthrough;
        let stats = self.mic_stats.lock().unwrap().clone();
        if let (true, Some(st)) = (passthrough, stats) {
            self.emit(
                events::AUDIO_MIC_LEVEL,
                &MicLevel {
                    peak: st.peak.take().clamp(0.0, 1.0),
                },
            );
        }
    }
}

/// Поток устройств: владеет стримами cpal.
struct DeviceThread {
    inner: Arc<Inner>,
    tx: mpsc::Sender<DevMsg>,
    host: cpal::Host,
    snap: Snapshot,
    resolved: Resolved,
    streams: [Option<OpenStream>; 3],
}

const MIC: usize = 2;

impl DeviceThread {
    fn new(inner: Arc<Inner>, tx: mpsc::Sender<DevMsg>) -> Self {
        Self {
            inner,
            tx,
            host: cpal::default_host(),
            snap: Snapshot::default(),
            resolved: Resolved::default(),
            streams: [None, None, None],
        }
    }

    fn run(mut self, rx: mpsc::Receiver<DevMsg>) {
        self.refresh_list(true);
        self.rebuild_all();
        self.inner.devices_ready.store(true, Ordering::SeqCst);
        loop {
            match rx.recv_timeout(POLL) {
                Ok(DevMsg::Rebuild) => {
                    self.refresh_list(false);
                    self.rebuild_all();
                }
                Ok(DevMsg::RebuildMic) => self.rebuild_mic(),
                Ok(DevMsg::StreamError(msg)) => {
                    // склеиваем пачку ошибок и даём системе устояться
                    std::thread::sleep(Duration::from_millis(300));
                    while rx.try_recv().is_ok() {}
                    self.inner
                        .emit_error(&AppError::new("device_unavailable", msg));
                    self.refresh_list(false);
                    self.rebuild_all();
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if self.refresh_list(false) {
                        let want = self.want();
                        if want != self.resolved {
                            self.rebuild_all();
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn want(&self) -> Resolved {
        let s = self.inner.settings.lock().unwrap().clone();
        devices::resolve(
            &self.snap.list(),
            s.monitor_device_id.as_deref(),
            s.cable_device_id.as_deref(),
            s.mic_device_id.as_deref(),
        )
    }

    /// Перечислить устройства; `true`, если список изменился.
    fn refresh_list(&mut self, force_emit: bool) -> bool {
        self.snap = devices::enumerate(&self.host);
        let list = self.snap.list();
        let changed = {
            let mut cur = self.inner.devices.lock().unwrap();
            let changed = *cur != list;
            *cur = list.clone();
            changed
        };
        if changed || force_emit {
            if changed {
                self.inner.emit(events::AUDIO_DEVICES_CHANGED, &list);
            }
            let cs = devices::cable_status(&list);
            let cs_changed = {
                let mut cur = self.inner.cable.lock().unwrap();
                let c = *cur != cs;
                *cur = cs.clone();
                c
            };
            if cs_changed {
                self.inner.emit(events::AUDIO_CABLE_STATUS, &cs);
            }
        }
        changed
    }

    fn error_cb(&self, what: &'static str) -> impl FnMut(cpal::Error) + Send + 'static {
        let tx = self.tx.clone();
        move |e: cpal::Error| match e.kind() {
            cpal::ErrorKind::Xrun | cpal::ErrorKind::RealtimeDenied => {}
            _ => {
                let _ = tx.send(DevMsg::StreamError(format!("{what}: {e}")));
            }
        }
    }

    fn close_all(&mut self) {
        // голоса на закрываемых выходах — «остановлены»
        self.inner.stop_where(|a| !a.silent);
        let taken: Vec<OutputControl> = {
            let mut outs = self.inner.outputs.lock().unwrap();
            outs.iter_mut().filter_map(Option::take).collect()
        };
        *self.inner.mic_stats.lock().unwrap() = None;
        for s in self.streams.iter_mut() {
            drop(s.take());
        }
        *self.inner.stream_reports.lock().unwrap() = [None, None, None];
        for mut c in taken {
            c.collect_garbage();
        }
    }

    fn open_out(&mut self, idx: usize, id: &str) -> AppResult<()> {
        let dev = self
            .snap
            .output(id)
            .ok_or_else(|| AppError::new("device_unavailable", format!("нет устройства {id}")))?
            .clone();
        let (cfg, fmt) = devices::output_config(&dev)?;
        let what = if idx == MONITOR { "monitor" } else { "cable" };
        let attempt = |cfg: cpal::StreamConfig| {
            let (renderer, ctl) =
                mixer::output(idx, cfg.sample_rate, self.inner.params[idx].clone());
            devices::open_output(&dev, cfg, fmt, renderer, self.error_cb(what)).map(|s| (s, ctl))
        };
        // малый буфер не принят драйвером — системный по умолчанию
        let (s, ctl) = attempt(cfg).or_else(|_| {
            attempt(cpal::StreamConfig {
                buffer_size: cpal::BufferSize::Default,
                ..cfg
            })
        })?;
        let name = dev
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_default();
        self.inner.stream_reports.lock().unwrap()[idx] = Some((
            name,
            s.stats.clone(),
            s.rate,
            s.channels,
            devices::period_frames(&s),
        ));
        self.inner.outputs.lock().unwrap()[idx] = Some(ctl);
        self.streams[idx] = Some(s);
        Ok(())
    }

    fn rebuild_all(&mut self) {
        self.close_all();
        self.resolved = self.want();
        let r = self.resolved.clone();
        if let Some(id) = &r.monitor
            && let Err(e) = self.open_out(MONITOR, id)
        {
            self.inner.emit_error(&e);
        }
        if let Some(id) = &r.cable
            && let Err(e) = self.open_out(CABLE, id)
        {
            self.inner.emit_error(&e);
        }
        let rates: Vec<u32> = self.inner.output_rates().into_iter().map(|r| r.1).collect();
        self.inner.cache.retain_rates(&rates);
        self.rebuild_mic();
        self.inner.preload_library();
    }

    fn rebuild_mic(&mut self) {
        if let Some(c) = self.inner.outputs.lock().unwrap()[CABLE].as_mut() {
            c.send(RtCmd::SetMic(None));
        }
        drop(self.streams[MIC].take());
        *self.inner.mic_stats.lock().unwrap() = None;
        self.inner.stream_reports.lock().unwrap()[MIC] = None;

        let passthrough = self.inner.settings.lock().unwrap().mic_passthrough;
        let cable_rate = self.inner.outputs.lock().unwrap()[CABLE]
            .as_ref()
            .map(|c| c.rate);
        let (true, Some(cable_rate), Some(mic_id)) =
            (passthrough, cable_rate, self.resolved.mic.clone())
        else {
            return;
        };
        let res = (|| -> AppResult<()> {
            let dev = self
                .snap
                .input(&mic_id)
                .ok_or_else(|| AppError::new("device_unavailable", "микрофон не найден"))?
                .clone();
            let (cfg, fmt) = devices::input_config(&dev)?;
            let attempt = |cfg: cpal::StreamConfig| {
                let (writer, reader) = mic::bridge(cfg.sample_rate, cable_rate);
                devices::open_input(&dev, cfg, fmt, writer, self.error_cb("mic"))
                    .map(|s| (s, reader))
            };
            let (s, reader) = attempt(cfg).or_else(|_| {
                attempt(cpal::StreamConfig {
                    buffer_size: cpal::BufferSize::Default,
                    ..cfg
                })
            })?;
            let stats = reader.stats();
            let name = dev
                .description()
                .map(|d| d.name().to_string())
                .unwrap_or_default();
            self.inner.stream_reports.lock().unwrap()[MIC] = Some((
                name,
                s.stats.clone(),
                s.rate,
                s.channels,
                devices::period_frames(&s),
            ));
            if let Some(c) = self.inner.outputs.lock().unwrap()[CABLE].as_mut() {
                c.send(RtCmd::SetMic(Some(Box::new(reader))));
            }
            *self.inner.mic_stats.lock().unwrap() = Some(stats);
            self.streams[MIC] = Some(s);
            Ok(())
        })();
        if let Err(e) = res {
            self.inner.emit_error(&e);
        }
    }
}
