//! Тесты движка без реальных устройств: рендереры крутим вручную.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::mixer::{self, Renderer};
use super::*;

#[derive(Default)]
struct VecSink(Mutex<Vec<(String, Value)>>);

impl EventSink for VecSink {
    fn emit(&self, event: &str, payload: Value) {
        self.0.lock().unwrap().push((event.into(), payload));
    }
}

impl VecSink {
    fn take(&self) -> Vec<(String, Value)> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
    /// Только `audio://playback`: (soundId, state).
    fn playback(&self) -> Vec<(String, String)> {
        self.take()
            .into_iter()
            .filter(|(e, _)| e == events::AUDIO_PLAYBACK)
            .map(|(_, p)| {
                (
                    p["soundId"].as_str().unwrap().to_string(),
                    p["state"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    }
}

struct Rig {
    engine: AudioEngine,
    sink: Arc<VecSink>,
    mon: Renderer,
    cab: Renderer,
    dir: PathBuf,
}

impl Drop for Rig {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn rig(tag: &str, mon_rate: u32, cab_rate: u32) -> Rig {
    let engine = AudioEngine::new();
    let sink = Arc::new(VecSink::default());
    *engine.inner.sink.write().unwrap() = Some(sink.clone());
    let (mon, mc) = mixer::output(MONITOR, mon_rate, engine.inner.params[MONITOR].clone());
    let (cab, cc) = mixer::output(CABLE, cab_rate, engine.inner.params[CABLE].clone());
    *engine.inner.outputs.lock().unwrap() = [Some(mc), Some(cc)];
    engine.inner.devices_ready.store(true, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("boomly-eng-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    Rig {
        engine,
        sink,
        mon,
        cab,
        dir,
    }
}

/// WAV 48 кГц моно с постоянным уровнем `level` (int16).
fn wav(path: &Path, frames: usize, level: i16) {
    let data_len = (frames * 2) as u32;
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&48_000u32.to_le_bytes());
    b.extend_from_slice(&96_000u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for _ in 0..frames {
        b.extend_from_slice(&level.to_le_bytes());
    }
    std::fs::write(path, b).unwrap();
}

impl Rig {
    fn sound(&self, id: &str, frames: usize, mode: PlayMode) -> Sound {
        let path = self.dir.join(format!("{id}.wav"));
        if !path.exists() {
            wav(&path, frames, 16384); // 0.5
        }
        Sound {
            id: id.into(),
            name: id.into(),
            path: path.to_string_lossy().into(),
            color: "cyan".into(),
            emoji: None,
            volume: 1.0,
            mode,
            favorite: false,
            duration_ms: None,
            added_at: 0,
        }
    }

    /// Положить звук в кэш заранее (как делает предзагрузка).
    fn preload(&self, s: &Sound) {
        let rates: Vec<u32> = self
            .engine
            .inner
            .output_rates()
            .into_iter()
            .map(|r| r.1)
            .collect();
        self.engine
            .inner
            .cache
            .preload(Path::new(&s.path), &rates)
            .unwrap();
    }

    /// Выполнить все задания декодера синхронно.
    fn run_jobs(&self) {
        loop {
            let job = self.engine.inner.jobs.q.lock().unwrap().pop_front();
            match job {
                Some(j) => self.engine.inner.run_job(j),
                None => break,
            }
        }
    }

    fn render(&mut self, frames: usize) -> (Vec<f32>, Vec<f32>) {
        let mut a = vec![0.0; frames * 2];
        let mut b = vec![0.0; frames * 2];
        self.mon.render(&mut a);
        self.cab.render(&mut b);
        (a, b)
    }

    fn live(&self) -> usize {
        self.engine
            .inner
            .active
            .lock()
            .unwrap()
            .iter()
            .filter(|a| !a.stopped)
            .count()
    }
}

#[test]
fn plays_to_both_outputs_with_independent_volumes() {
    let mut r = rig("route", 48_000, 48_000);
    let s = r.sound("a", 48_000, PlayMode::Toggle);
    r.preload(&s);
    r.engine.set_volume(VolumeChannel::Monitor, 1.0).unwrap();
    r.engine.set_volume(VolumeChannel::Cable, 0.5).unwrap();
    r.engine.play(&s).unwrap();
    assert_eq!(r.sink.playback(), vec![("a".into(), "started".into())]);
    let (m, c) = r.render(1024);
    assert!((m[1000] - 0.5).abs() < 1e-3, "monitor {}", m[1000]);
    assert!((c[1000] - 0.25).abs() < 1e-3, "cable {}", c[1000]);
}

#[test]
fn sound_volume_scales() {
    let mut r = rig("vol", 48_000, 48_000);
    let mut s = r.sound("a", 48_000, PlayMode::Overlay);
    s.volume = 1.5;
    r.preload(&s);
    r.engine.set_volume(VolumeChannel::Monitor, 1.0).unwrap();
    r.engine.play(&s).unwrap();
    let (m, _) = r.render(1024);
    assert!((m[1000] - 0.75).abs() < 1e-3, "{}", m[1000]);
}

#[test]
fn toggle_mode_stops_on_second_play() {
    let mut r = rig("toggle", 48_000, 48_000);
    let s = r.sound("t", 48_000, PlayMode::Toggle);
    r.preload(&s);
    r.engine.play(&s).unwrap();
    r.engine.play(&s).unwrap();
    assert_eq!(r.live(), 0);
    assert_eq!(
        r.sink.playback(),
        vec![
            ("t".into(), "started".into()),
            ("t".into(), "stopped".into())
        ]
    );
    // после затухания — тишина, и тикер убирает экземпляр без события finished
    r.render(2048);
    let (m, _) = r.render(256);
    assert!(m.iter().all(|x| *x == 0.0));
    r.engine.inner.tick();
    assert!(r.engine.inner.active.lock().unwrap().is_empty());
    assert!(r.sink.playback().is_empty());
}

#[test]
fn restart_mode_starts_over() {
    let mut r = rig("restart", 48_000, 48_000);
    let s = r.sound("r", 48_000, PlayMode::Restart);
    r.preload(&s);
    r.engine.play(&s).unwrap();
    r.render(4800);
    r.engine.play(&s).unwrap();
    assert_eq!(r.live(), 1);
    assert_eq!(
        r.sink.playback(),
        vec![
            ("r".into(), "started".into()),
            ("r".into(), "stopped".into()),
            ("r".into(), "started".into())
        ]
    );
}

#[test]
fn overlay_mode_layers_copies() {
    let mut r = rig("overlay", 48_000, 48_000);
    let s = r.sound("o", 48_000, PlayMode::Overlay);
    r.preload(&s);
    r.engine.set_volume(VolumeChannel::Monitor, 1.0).unwrap();
    r.engine.play(&s).unwrap();
    r.engine.play(&s).unwrap();
    assert_eq!(r.live(), 2);
    let (m, _) = r.render(1024);
    assert!(m[1000] > 0.9, "две копии складываются: {}", m[1000]);
    r.engine.stop("o").unwrap();
    assert_eq!(r.live(), 0);
}

#[test]
fn finished_event_and_progress() {
    let mut r = rig("finish", 48_000, 48_000);
    let s = r.sound("f", 4800, PlayMode::Toggle); // 100 мс
    r.preload(&s);
    r.engine.play(&s).unwrap();
    r.sink.take();
    r.render(2400);
    r.engine.inner.tick();
    let ev = r.sink.take();
    let prog = ev
        .iter()
        .find(|(e, _)| e == events::AUDIO_PROGRESS)
        .expect("progress");
    assert_eq!(prog.1[0]["soundId"], "f");
    assert_eq!(prog.1[0]["positionMs"], 50);
    assert_eq!(prog.1[0]["durationMs"], 100);
    r.render(4800);
    r.engine.inner.tick();
    assert_eq!(r.sink.playback(), vec![("f".into(), "finished".into())]);
    assert!(r.engine.inner.active.lock().unwrap().is_empty());
}

#[test]
fn cache_miss_decodes_in_worker() {
    let mut r = rig("miss", 48_000, 44_100);
    let s = r.sound("m", 48_000, PlayMode::Toggle);
    r.engine.play(&s).unwrap();
    assert!(r.sink.playback().is_empty(), "до декодирования не стартует");
    r.run_jobs();
    assert_eq!(r.sink.playback(), vec![("m".into(), "started".into())]);
    // на кабеле 44,1 кГц — свой клип, ресемплированный
    let (_, c) = r.render(512);
    assert!((c[900] - 0.5 * 0.8).abs() < 0.01, "{}", c[900]);
    // теперь звук в кэше — старт сразу
    r.engine.stop_all().unwrap();
    r.sink.take();
    let mut s2 = s.clone();
    s2.mode = PlayMode::Overlay;
    r.engine.play(&s2).unwrap();
    assert_eq!(r.sink.playback(), vec![("m".into(), "started".into())]);
}

#[test]
fn toggle_while_decoding_cancels() {
    let r = rig("cancel", 48_000, 48_000);
    let s = r.sound("c", 48_000, PlayMode::Toggle);
    r.engine.play(&s).unwrap();
    r.engine.play(&s).unwrap(); // передумал до старта
    r.run_jobs();
    assert_eq!(r.sink.playback(), vec![("c".into(), "stopped".into())]);
    assert_eq!(r.live(), 0);
}

#[test]
fn decode_error_is_reported() {
    let r = rig("err", 48_000, 48_000);
    let mut s = r.sound("x", 10, PlayMode::Toggle);
    s.path = r.dir.join("nope.mp3").to_string_lossy().into();
    r.engine.play(&s).unwrap();
    r.run_jobs();
    let ev = r.sink.take();
    assert!(
        ev.iter()
            .any(|(e, p)| e == events::AUDIO_PLAYBACK && p["state"] == "error")
    );
    assert!(
        ev.iter()
            .any(|(e, p)| e == events::AUDIO_ERROR && p["code"] == "not_found")
    );
}

#[test]
fn no_outputs_is_device_unavailable() {
    let engine = AudioEngine::new();
    let s = Sound {
        id: "z".into(),
        name: "z".into(),
        path: "z.wav".into(),
        color: "cyan".into(),
        emoji: None,
        volume: 1.0,
        mode: PlayMode::Toggle,
        favorite: false,
        duration_ms: None,
        added_at: 0,
    };
    assert_eq!(engine.play(&s).unwrap_err().code, "device_unavailable");
}

#[test]
fn play_test_targets_one_output_silently() {
    let mut r = rig("test", 48_000, 48_000);
    r.engine.play_test(TestTarget::Cable).unwrap();
    let (m, c) = r.render(4800);
    assert!(m.iter().all(|x| *x == 0.0));
    assert!(c.iter().any(|x| x.abs() > 0.05));
    assert!(r.sink.playback().is_empty(), "тест мастера без событий");
    // без кабеля — cable_missing
    r.engine.inner.outputs.lock().unwrap()[CABLE] = None;
    assert_eq!(
        r.engine.play_test(TestTarget::Cable).unwrap_err().code,
        "cable_missing"
    );
}

#[test]
fn settings_validation_and_events() {
    let r = rig("set", 48_000, 48_000);
    assert_eq!(
        r.engine
            .set_volume(VolumeChannel::Mic, 1.5)
            .unwrap_err()
            .code,
        "invalid_argument"
    );
    let s = r.engine.set_mic_muted(true).unwrap();
    assert!(s.mic_muted);
    assert!(
        r.engine.inner.params[CABLE]
            .mic_muted
            .load(Ordering::Relaxed)
    );
    let s = r.engine.toggle_mic_muted().unwrap();
    assert!(!s.mic_muted);
    let ev = r.sink.take();
    assert_eq!(
        ev.iter()
            .filter(|(e, _)| e == events::AUDIO_SETTINGS_CHANGED)
            .count(),
        2
    );
    // то же значение — без события
    r.engine.set_mic_muted(false).unwrap();
    assert!(r.sink.take().is_empty());
    // частичный выбор устройств: null — «по умолчанию», нет поля — не трогать
    r.engine
        .set_devices(DeviceSelection {
            monitor_device_id: Some(Some("x".into())),
            cable_device_id: None,
            mic_device_id: Some(None),
        })
        .unwrap();
    let s = r.engine.settings();
    assert_eq!(s.monitor_device_id.as_deref(), Some("x"));
    assert_eq!(s.mic_device_id, None);
}

#[test]
fn settings_persist_and_load_partial_file() {
    let r = rig("persist", 48_000, 48_000);
    let path = r.dir.join("audio.json");
    *r.engine.inner.settings_path.lock().unwrap() = Some(path.clone());
    r.engine.set_volume(VolumeChannel::Cable, 0.3).unwrap();
    assert!(!path.exists(), "запись на диск — из тикера, не в вызове");
    r.engine.inner.tick();
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!((saved["cableVolume"].as_f64().unwrap() - 0.3).abs() < 1e-6);
    // старый/частичный файл: недостающие поля — по умолчанию, громкость зажимается
    std::fs::write(
        &path,
        "\u{feff}{\"monitorVolume\": 7.0, \"micMuted\": true}",
    )
    .unwrap();
    let e2 = AudioEngine::new();
    e2.inner.load_settings(&path);
    let s = e2.settings();
    assert_eq!(s.monitor_volume, 1.0);
    assert!(s.mic_muted);
    assert!(s.mic_passthrough);
    assert!(e2.inner.params[CABLE].mic_muted.load(Ordering::Relaxed));
}

#[test]
fn mic_level_only_when_passthrough() {
    let r = rig("level", 48_000, 48_000);
    let (_w, rd) = mic::bridge(48_000, 48_000);
    let st = rd.stats();
    st.peak.fetch_max(0.4);
    *r.engine.inner.mic_stats.lock().unwrap() = Some(st.clone());
    r.engine.inner.tick();
    let ev = r.sink.take();
    let lvl = ev
        .iter()
        .find(|(e, _)| e == events::AUDIO_MIC_LEVEL)
        .unwrap();
    assert!((lvl.1["peak"].as_f64().unwrap() - 0.4).abs() < 1e-6);
    r.engine.inner.settings.lock().unwrap().mic_passthrough = false;
    r.engine.inner.tick();
    assert!(
        !r.sink
            .take()
            .iter()
            .any(|(e, _)| e == events::AUDIO_MIC_LEVEL)
    );
}

#[test]
fn overlay_while_decoding_gives_two_copies() {
    let r = rig("ovl-dec", 48_000, 48_000);
    let s = r.sound("d", 48_000, PlayMode::Overlay);
    r.engine.play(&s).unwrap();
    r.engine.play(&s).unwrap();
    r.run_jobs();
    assert_eq!(
        r.sink.playback(),
        vec![
            ("d".into(), "started".into()),
            ("d".into(), "started".into())
        ]
    );
    assert_eq!(r.live(), 2);
}

#[test]
fn stop_playing_and_pending_emits_once() {
    let r = rig("stop-once", 48_000, 48_000);
    let s = r.sound("s", 48_000, PlayMode::Overlay);
    r.preload(&s);
    r.engine.play(&s).unwrap(); // из кэша — играет
    r.engine.inner.cache.retain_rates(&[]); // вторая копия пойдёт через декодер
    r.engine.play(&s).unwrap();
    r.sink.take();
    r.engine.stop("s").unwrap();
    r.run_jobs();
    assert_eq!(r.sink.playback(), vec![("s".into(), "stopped".into())]);
    assert_eq!(r.live(), 0);
}
