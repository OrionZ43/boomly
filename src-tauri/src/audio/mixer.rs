//! Рендерер одного выхода — то, что крутится в аудиоколбэке cpal.
//!
//! Правила реального времени: в `render` нет аллокаций, блокировок и освобождения памяти.
//! Команды приходят через lock-free кольцо (`rtrb`), а всё, что надо освободить (доигравшие
//! голоса, старый мост микрофона), уходит обратно в «мусорное» кольцо и освобождается
//! потоком движка.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rtrb::{Consumer, Producer, RingBuffer};

use super::decode::Clip;
use super::dsp::{AtomicF32, Limiter, SmoothedGain};
use super::mic::MicReader;

/// Индексы выходов.
pub const MONITOR: usize = 0;
pub const CABLE: usize = 1;

/// Блок микса (кадров). Колбэк длиннее — обрабатывается кусками.
const MIX_BLOCK: usize = 2048;
/// Затухание при остановке, чтобы не щёлкало.
const STOP_FADE_S: f32 = 0.006;
const CMD_CAPACITY: usize = 256;
const GARBAGE_CAPACITY: usize = 512;
const MAX_VOICES: usize = 128;

/// Общее состояние экземпляра звука: видят и движок, и рендереры.
#[derive(Debug, Default)]
pub struct VoiceShared {
    /// Позиция на каждом выходе, мс.
    pub pos_ms: [AtomicU64; 2],
}

/// Голос внутри рендерера.
pub struct RtVoice {
    pub instance: u64,
    pub shared: Arc<VoiceShared>,
    pub clip: Arc<Clip>,
    pub gain: f32,
    pos: usize,
    /// Оставшиеся кадры затухания (`Some` — останавливается).
    fade: Option<u32>,
}

impl RtVoice {
    pub fn new(instance: u64, shared: Arc<VoiceShared>, clip: Arc<Clip>, gain: f32) -> Self {
        Self {
            instance,
            shared,
            clip,
            gain,
            pos: 0,
            fade: None,
        }
    }
}

pub enum RtCmd {
    Add(RtVoice),
    Stop(u64),
    StopAll,
    SetMic(Option<Box<MicReader>>),
}

/// То, что рендерер отдаёт на освобождение.
pub enum Garbage {
    Voice(#[allow(dead_code)] RtVoice),
    Mic(#[allow(dead_code)] Box<MicReader>),
    Cmd(#[allow(dead_code)] RtCmd),
}

/// Параметры, которые движок меняет на лету.
#[derive(Debug)]
pub struct OutputParams {
    pub volume: AtomicF32,
    pub mic_volume: AtomicF32,
    pub mic_muted: AtomicBool,
}

impl OutputParams {
    pub fn new(volume: f32) -> Arc<Self> {
        Arc::new(Self {
            volume: AtomicF32::new(volume),
            mic_volume: AtomicF32::new(1.0),
            mic_muted: AtomicBool::new(false),
        })
    }
}

/// Сторона движка у выхода: слать команды, забирать мусор.
pub struct OutputControl {
    pub cmds: Producer<RtCmd>,
    pub garbage: Consumer<Garbage>,
    pub rate: u32,
}

impl OutputControl {
    /// Отправить команду; если кольцо забито (колбэк не идёт) — `false`.
    pub fn send(&mut self, cmd: RtCmd) -> bool {
        self.cmds.push(cmd).is_ok()
    }

    /// Освободить всё, что вернул рендерер (вызывать из потока движка).
    pub fn collect_garbage(&mut self) {
        while let Ok(g) = self.garbage.pop() {
            drop(g);
        }
    }
}

pub struct Renderer {
    index: usize,
    rate: u32,
    cmds: Consumer<RtCmd>,
    garbage: Producer<Garbage>,
    voices: Vec<RtVoice>,
    params: Arc<OutputParams>,
    master: SmoothedGain,
    limiter: Limiter,
    mic: Option<Box<MicReader>>,
    mix: Vec<f32>,
    fade_frames: u32,
}

/// Создать пару «рендерер (в колбэк) + управление (движку)».
pub fn output(index: usize, rate: u32, params: Arc<OutputParams>) -> (Renderer, OutputControl) {
    let (cmd_tx, cmd_rx) = RingBuffer::new(CMD_CAPACITY);
    let (g_tx, g_rx) = RingBuffer::new(GARBAGE_CAPACITY);
    let vol = params.volume.get();
    (
        Renderer {
            index,
            rate,
            cmds: cmd_rx,
            garbage: g_tx,
            voices: Vec::with_capacity(MAX_VOICES),
            params,
            master: SmoothedGain::new(vol, rate),
            limiter: Limiter::new(rate),
            mic: None,
            mix: vec![0.0; MIX_BLOCK * 2],
            fade_frames: ((rate as f32 * STOP_FADE_S) as u32).max(1),
        },
        OutputControl {
            cmds: cmd_tx,
            garbage: g_rx,
            rate,
        },
    )
}

impl Renderer {
    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn active_voices(&self) -> usize {
        self.voices.len()
    }

    fn trash(&mut self, g: Garbage) {
        // если мусорное кольцо переполнено (движок завис), придётся освободить здесь
        let _ = self.garbage.push(g);
    }

    fn apply_commands(&mut self) {
        while let Ok(cmd) = self.cmds.pop() {
            match cmd {
                RtCmd::Add(v) => {
                    if self.voices.len() < self.voices.capacity() {
                        self.voices.push(v);
                    } else {
                        self.trash(Garbage::Voice(v));
                    }
                }
                RtCmd::Stop(id) => {
                    let ff = self.fade_frames;
                    for v in self.voices.iter_mut().filter(|v| v.instance == id) {
                        v.fade.get_or_insert(ff);
                    }
                }
                RtCmd::StopAll => {
                    let ff = self.fade_frames;
                    for v in self.voices.iter_mut() {
                        v.fade.get_or_insert(ff);
                    }
                }
                RtCmd::SetMic(m) => {
                    if let Some(old) = std::mem::replace(&mut self.mic, m) {
                        self.trash(Garbage::Mic(old));
                    }
                }
            }
        }
    }

    /// Заполнить стерео-буфер (interleaved L,R).
    pub fn render(&mut self, out: &mut [f32]) {
        self.apply_commands();
        for chunk in out.chunks_mut(MIX_BLOCK * 2) {
            self.render_block(chunk);
        }
    }

    fn render_block(&mut self, out: &mut [f32]) {
        let frames = out.len() / 2;
        let mix = &mut self.mix[..frames * 2];
        mix.iter_mut().for_each(|x| *x = 0.0);

        // звуки
        let fade_frames = self.fade_frames as f32;
        let mut i = 0;
        while i < self.voices.len() {
            let v = &mut self.voices[i];
            let total = v.clip.frames();
            let n = frames.min(total.saturating_sub(v.pos));
            let src = &v.clip.samples[v.pos * 2..(v.pos + n) * 2];
            let mut finished = v.pos + n >= total;
            match v.fade.as_mut() {
                None => {
                    for (d, s) in mix.iter_mut().zip(src) {
                        *d += s * v.gain;
                    }
                }
                Some(left) => {
                    for (k, s) in src.chunks_exact(2).enumerate() {
                        if *left == 0 {
                            break;
                        }
                        let g = v.gain * (*left as f32 / fade_frames);
                        mix[2 * k] += s[0] * g;
                        mix[2 * k + 1] += s[1] * g;
                        *left -= 1;
                    }
                    if *left == 0 {
                        finished = true;
                    }
                }
            }
            v.pos += n;
            v.shared.pos_ms[self.index]
                .store(v.pos as u64 * 1000 / self.rate as u64, Ordering::Relaxed);
            if finished {
                let v = self.voices.swap_remove(i);
                let _ = self.garbage.push(Garbage::Voice(v));
            } else {
                i += 1;
            }
        }

        // громкость выхода — только на звуки; у голоса своя громкость
        self.master.set_target(self.params.volume.get());
        for f in mix.chunks_exact_mut(2) {
            let g = self.master.next_gain();
            f[0] *= g;
            f[1] *= g;
        }

        if let Some(mic) = self.mic.as_mut() {
            mic.mix_into(
                mix,
                frames,
                self.params.mic_volume.get(),
                self.params.mic_muted.load(Ordering::Relaxed),
            );
        }

        for (o, f) in out.chunks_exact_mut(2).zip(mix.chunks_exact_mut(2)) {
            let (mut l, mut r) = (f[0], f[1]);
            self.limiter.process_frame(&mut l, &mut r);
            o[0] = l;
            o[1] = r;
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // стрим закрыт: всё, что не успело попасть в рендерер, тоже на освобождение
        while let Ok(c) = self.cmds.pop() {
            let _ = self.garbage.push(Garbage::Cmd(c));
        }
    }
}

/// Разложить стерео в буфер устройства с `channels` каналами.
#[inline]
pub fn write_frames<T: Copy>(
    dst: &mut [T],
    stereo: &[f32],
    channels: usize,
    conv: impl Fn(f32) -> T,
    zero: T,
) {
    match channels {
        1 => {
            for (d, s) in dst.iter_mut().zip(stereo.chunks_exact(2)) {
                *d = conv((s[0] + s[1]) * 0.5);
            }
        }
        _ => {
            for (d, s) in dst.chunks_exact_mut(channels).zip(stereo.chunks_exact(2)) {
                d[0] = conv(s[0]);
                d[1] = conv(s[1]);
                for x in &mut d[2..] {
                    *x = zero;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::mic;

    fn dc_clip(rate: u32, frames: usize, v: f32) -> Arc<Clip> {
        Arc::new(Clip {
            rate,
            samples: vec![v; frames * 2],
        })
    }

    fn voice(id: u64, clip: Arc<Clip>, gain: f32) -> (RtVoice, Arc<VoiceShared>) {
        let sh = Arc::new(VoiceShared::default());
        (RtVoice::new(id, sh.clone(), clip, gain), sh)
    }

    fn setup(vol: f32) -> (Renderer, OutputControl, Arc<OutputParams>) {
        let p = OutputParams::new(vol);
        let (r, c) = output(MONITOR, 48_000, p.clone());
        (r, c, p)
    }

    #[test]
    fn sums_voices_with_gains() {
        let (mut r, mut c, _) = setup(1.0);
        let (v1, _) = voice(1, dc_clip(48_000, 1000, 0.2), 1.0);
        let (v2, _) = voice(2, dc_clip(48_000, 1000, 0.1), 2.0);
        c.send(RtCmd::Add(v1));
        c.send(RtCmd::Add(v2));
        let mut out = vec![0.0; 256];
        r.render(&mut out);
        assert!(out.iter().all(|x| (x - 0.4).abs() < 1e-6), "{}", out[0]);
    }

    #[test]
    fn output_volume_applies_smoothly() {
        let (mut r, mut c, p) = setup(1.0);
        let (v, _) = voice(1, dc_clip(48_000, 48_000, 0.5), 1.0);
        c.send(RtCmd::Add(v));
        let mut out = vec![0.0; 960];
        r.render(&mut out);
        p.volume.set(0.2);
        r.render(&mut out);
        // через ~10 мс громкость дошла до 0.2, без скачка в первом кадре
        assert!(out[0] > 0.49 * 0.99);
        assert!((out[958] - 0.1).abs() < 1e-3, "{}", out[958]);
    }

    #[test]
    fn voice_finishes_and_goes_to_garbage() {
        let (mut r, mut c, _) = setup(1.0);
        let (v, sh) = voice(1, dc_clip(48_000, 100, 0.5), 1.0);
        c.send(RtCmd::Add(v));
        let mut out = vec![0.0; 512];
        r.render(&mut out);
        assert_eq!(r.active_voices(), 0);
        // после 100 кадров — тишина
        assert!(out[..200].iter().all(|x| (x - 0.5).abs() < 1e-6));
        assert!(out[200..].iter().all(|x| *x == 0.0));
        assert_eq!(
            Arc::strong_count(&sh),
            2,
            "голос ждёт освобождения в мусоре"
        );
        c.collect_garbage();
        assert_eq!(Arc::strong_count(&sh), 1);
        assert_eq!(sh.pos_ms[MONITOR].load(Ordering::Relaxed), 2); // 100 кадров ≈ 2 мс
    }

    #[test]
    fn stop_fades_out_without_click() {
        let (mut r, mut c, _) = setup(1.0);
        let (v, _) = voice(7, dc_clip(48_000, 48_000, 0.5), 1.0);
        c.send(RtCmd::Add(v));
        let mut out = vec![0.0; 256];
        r.render(&mut out);
        c.send(RtCmd::Stop(7));
        let mut out = vec![0.0; 1024];
        r.render(&mut out);
        let l: Vec<f32> = out.iter().step_by(2).copied().collect();
        // монотонно спадает, шаг не больше 0.5/fade
        for w in l.windows(2) {
            assert!(w[1] <= w[0] + 1e-6 && w[0] - w[1] < 0.5 / 288.0 + 1e-4);
        }
        assert_eq!(*l.last().unwrap(), 0.0);
        assert_eq!(r.active_voices(), 0);
    }

    #[test]
    fn stop_all_and_other_ids_untouched() {
        let (mut r, mut c, _) = setup(1.0);
        let (a, _) = voice(1, dc_clip(48_000, 48_000, 0.1), 1.0);
        let (b, _) = voice(2, dc_clip(48_000, 48_000, 0.1), 1.0);
        c.send(RtCmd::Add(a));
        c.send(RtCmd::Add(b));
        c.send(RtCmd::Stop(1));
        let mut out = vec![0.0; 2048];
        r.render(&mut out);
        assert_eq!(r.active_voices(), 1);
        c.send(RtCmd::StopAll);
        r.render(&mut out);
        assert_eq!(r.active_voices(), 0);
    }

    #[test]
    fn limiter_protects_from_clipping_overlay() {
        let (mut r, mut c, _) = setup(1.0);
        for i in 0..10 {
            let (v, _) = voice(i, dc_clip(48_000, 4800, 0.5), 2.0);
            c.send(RtCmd::Add(v));
        }
        let mut out = vec![0.0; 4096];
        r.render(&mut out);
        assert!(out.iter().all(|x| x.abs() <= 0.98 + 1e-6));
    }

    #[test]
    fn long_callback_is_chunked() {
        let (mut r, mut c, _) = setup(1.0);
        let (v, _) = voice(1, dc_clip(48_000, 10_000, 0.25), 1.0);
        c.send(RtCmd::Add(v));
        let mut out = vec![0.0; 5000 * 2];
        r.render(&mut out);
        assert!(out.iter().all(|x| (x - 0.25).abs() < 1e-6));
    }

    #[test]
    fn mic_is_mixed_into_cable_after_output_volume() {
        let p = OutputParams::new(0.0); // звуки на 0 — голос всё равно идёт
        let (mut r, mut c) = output(CABLE, 48_000, p.clone());
        let (mut w, rd) = mic::bridge(48_000, 48_000);
        for _ in 0..10 {
            w.write_interleaved(&[0.3; 480], 1);
        }
        c.send(RtCmd::SetMic(Some(Box::new(rd))));
        let (v, _) = voice(1, dc_clip(48_000, 48_000, 0.5), 1.0);
        c.send(RtCmd::Add(v));
        let mut out = vec![0.0; 960];
        r.render(&mut out);
        r.render(&mut out);
        assert!((out[900] - 0.3).abs() < 1e-3, "{}", out[900]);
        // снять микрофон — старый мост уходит в мусор
        c.send(RtCmd::SetMic(None));
        r.render(&mut out);
        assert!(out.iter().all(|x| x.abs() < 1e-6));
        c.collect_garbage();
    }

    #[test]
    fn drop_returns_pending_commands() {
        let (r, mut c, _) = setup(1.0);
        let (v, sh) = voice(1, dc_clip(48_000, 10, 0.1), 1.0);
        c.send(RtCmd::Add(v));
        drop(r);
        c.collect_garbage();
        assert_eq!(Arc::strong_count(&sh), 1);
    }

    #[test]
    fn channel_mapping() {
        let stereo = [0.5, -0.5, 0.25, 0.75];
        let mut mono = [0.0f32; 2];
        write_frames(&mut mono, &stereo, 1, |x| x, 0.0);
        assert_eq!(mono, [0.0, 0.5]);
        let mut six = [9.0f32; 12];
        write_frames(&mut six, &stereo, 6, |x| x, 0.0);
        assert_eq!(&six[..6], &[0.5, -0.5, 0.0, 0.0, 0.0, 0.0]);
        let mut i16s = [0i16; 4];
        write_frames(&mut i16s, &stereo, 2, |x| (x * 32767.0) as i16, 0);
        assert_eq!(i16s[0], 16383);
    }
}
