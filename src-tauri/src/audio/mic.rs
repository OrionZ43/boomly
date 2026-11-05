//! Сквозной микрофон: мост «вход микрофона → выход VB-CABLE» между двумя устройствами
//! с независимыми часами.
//!
//! Микрофон и кабель тактируются разными генераторами: даже при одинаковой номинальной
//! частоте реальные частоты отличаются на десятки ppm, и простой буфер между ними со
//! временем либо пустеет (щелчки), либо копит задержку. Поэтому:
//!
//! 1. колбэк микрофона кладёт моно-сэмплы в lock-free кольцо (`rtrb`);
//! 2. колбэк кабеля забирает их через **адаптивный ресемплер** (кубический Эрмит) с шагом
//!    `in_rate / out_rate · (1 + коррекция)`; коррекцию задаёт пропорциональный регулятор по
//!    сглаженному отклонению заполнения кольца от цели;
//! 3. цель заполнения (замер перед чтением) = блок микрофона + блок кабеля + 2 мс. Меньше нельзя:
//!    из-за дрейфа фаза колбэков медленно «проезжает» целый блок, и уровень перед чтением
//!    скачком падает на блок микрофона (проверено тестом с ±300 ppm). При опустошении —
//!    тишина и повторная предбуферизация,
//!    при переполнении (система «подвисла») — сброс лишнего до цели.
//!
//! Коррекция ограничена ±0,5 % (≈ 8,6 цента) — на голосе не слышна, а дрейф часов
//! покрывает с запасом (реальный — 0,01–0,03 %).

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use rtrb::{Consumer, Producer, RingBuffer};

use super::dsp::{AtomicF32, SmoothedGain, hermite};

/// Максимальная коррекция шага ресемплера.
const MAX_CORRECTION: f64 = 0.005;
/// Коэффициент регулятора: коррекция на единицу относительной ошибки заполнения.
const KP: f64 = 0.004;
/// Запас на дрожание колбэков, секунды.
const MARGIN_S: f64 = 0.002;

/// Общие счётчики моста (для отчётов и тестов).
#[derive(Debug, Default)]
pub struct MicStats {
    /// Размер последнего блока микрофона, сэмплов.
    pub in_block: AtomicU32,
    /// Сколько раз кольцо опустело.
    pub underruns: AtomicU64,
    /// Сколько раз сбрасывали лишнее.
    pub overflows: AtomicU64,
    /// Средняя задержка кольца, мкс (для отчёта).
    pub buffer_us: AtomicU64,
    /// Пик голоса после громкости «Голос» (до mute) — для `audio://mic-level`.
    pub peak: AtomicF32,
}

/// Создать мост. Кольцо — 0,5 с (с запасом на «подвисания» системы).
pub fn bridge(in_rate: u32, out_rate: u32) -> (MicWriter, MicReader) {
    let cap = (in_rate as usize).max(8_000) / 2; // 0,5 с
    let (prod, cons) = RingBuffer::new(cap);
    let stats = Arc::new(MicStats::default());
    stats.in_block.store(in_rate / 100, Ordering::Relaxed); // догадка до первого колбэка
    let nominal = in_rate as f64 / out_rate as f64;
    (
        MicWriter {
            prod,
            stats: stats.clone(),
        },
        MicReader {
            cons,
            stats,
            in_rate,
            out_rate,
            nominal,
            step: nominal,
            frac: 0.0,
            hist: [0.0; 4],
            prebuffering: true,
            err_smooth: 0.0,
            gain: SmoothedGain::new(1.0, out_rate),
            mute: SmoothedGain::new(1.0, out_rate),
        },
    )
}

/// Сторона микрофона (живёт в колбэке входного потока).
pub struct MicWriter {
    prod: Producer<f32>,
    stats: Arc<MicStats>,
}

impl MicWriter {
    /// Положить блок, сведённый в моно. Лишнее при переполнении отбрасывается.
    pub fn write_interleaved(&mut self, data: &[f32], channels: usize) {
        let channels = channels.max(1);
        let frames = data.len() / channels;
        self.stats.in_block.store(frames as u32, Ordering::Relaxed);
        let inv = 1.0 / channels as f32;
        for frame in data.chunks_exact(channels) {
            let s: f32 = frame.iter().sum::<f32>() * inv;
            if self.prod.push(s).is_err() {
                break;
            }
        }
    }

    pub fn stats(&self) -> Arc<MicStats> {
        self.stats.clone()
    }
}

/// Сторона кабеля (живёт в рендерере выхода «в микрофон»).
pub struct MicReader {
    cons: Consumer<f32>,
    stats: Arc<MicStats>,
    in_rate: u32,
    out_rate: u32,
    nominal: f64,
    step: f64,
    frac: f64,
    hist: [f32; 4],
    prebuffering: bool,
    err_smooth: f64,
    gain: SmoothedGain,
    mute: SmoothedGain,
}

impl MicReader {
    pub fn stats(&self) -> Arc<MicStats> {
        self.stats.clone()
    }

    /// Цель заполнения кольца, сэмплов входа.
    fn target(&self, out_frames: usize) -> f64 {
        let in_block = self.stats.in_block.load(Ordering::Relaxed) as f64;
        in_block + out_frames as f64 * self.nominal + MARGIN_S * self.in_rate as f64
    }

    /// Подмешать голос в стерео-буфер `out` (interleaved L,R) длиной `frames` кадров.
    /// `volume` — «Голос», `muted` — mute; пик пишется в статистику.
    pub fn mix_into(&mut self, out: &mut [f32], frames: usize, volume: f32, muted: bool) {
        self.begin_callback(frames, volume, muted);
        self.mix_chunk(out, frames);
    }

    /// Начало колбэка кабеля: замер кольца, предбуферизация, регулятор. `frames` — размер
    /// **всего** колбэка (рендерер может потом подмешивать его кусками через `mix_chunk`).
    pub fn begin_callback(&mut self, frames: usize, volume: f32, muted: bool) {
        self.gain.set_target(volume);
        self.mute.set_target(if muted { 0.0 } else { 1.0 });

        let avail = self.cons.slots() as f64;
        let target = self.target(frames);
        self.stats.buffer_us.store(
            (avail / self.in_rate as f64 * 1e6) as u64,
            Ordering::Relaxed,
        );

        if self.prebuffering {
            if avail < target {
                return;
            }
            self.prebuffering = false;
            self.err_smooth = 0.0;
        }

        // «подвисание»: в кольце сильно больше цели — выбрасываем лишнее, а не копим задержку
        if avail > target * 3.0 + self.in_rate as f64 * 0.02 {
            let drop = (avail - target) as usize;
            if let Ok(chunk) = self.cons.read_chunk(drop) {
                chunk.commit_all();
            }
            self.stats.overflows.fetch_add(1, Ordering::Relaxed);
        }

        // регулятор: сглаживаем ошибку за ~0,5 с, чтобы не реагировать на пилу от блоков
        let avail = self.cons.slots() as f64;
        let err = (avail - target) / target.max(1.0);
        let alpha = (frames as f64 / (self.out_rate as f64 * 0.5)).min(1.0);
        self.err_smooth += alpha * (err - self.err_smooth);
        let corr = (KP * self.err_smooth).clamp(-MAX_CORRECTION, MAX_CORRECTION);
        self.step = self.nominal * (1.0 + corr);
    }

    /// Подмешать кусок колбэка (после `begin_callback`).
    pub fn mix_chunk(&mut self, out: &mut [f32], frames: usize) {
        if self.prebuffering {
            return;
        }
        let mut peak = 0.0f32;
        for i in 0..frames {
            self.frac += self.step;
            while self.frac >= 1.0 {
                self.frac -= 1.0;
                match self.cons.pop() {
                    Ok(s) => {
                        self.hist = [self.hist[1], self.hist[2], self.hist[3], s];
                    }
                    Err(_) => {
                        // опустели посреди блока — тишина до новой предбуферизации
                        self.prebuffering = true;
                        self.stats.underruns.fetch_add(1, Ordering::Relaxed);
                        self.hist = [0.0; 4];
                        self.frac = 0.0;
                        self.stats.peak.fetch_max(peak);
                        return;
                    }
                }
            }
            let [y0, y1, y2, y3] = self.hist;
            let v = hermite(y0, y1, y2, y3, self.frac as f32) * self.gain.next_gain();
            peak = peak.max(v.abs());
            let v = v * self.mute.next_gain();
            out[2 * i] += v;
            out[2 * i + 1] += v;
        }
        self.stats.peak.fetch_max(peak.min(1.0));
    }

    /// Текущий шаг относительно номинала (для тестов): 1.0 — без коррекции.
    pub fn relative_step(&self) -> f64 {
        self.step / self.nominal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Имитация двух устройств с разными часами: микрофон пишет блоками по 10 мс
    /// с частотой `in_rate·(1+ppm)`, кабель читает блоками по 10 мс с `out_rate`.
    /// Возвращает (reader, выход моно, номера блоков с недобором).
    fn simulate(
        in_rate: u32,
        out_rate: u32,
        ppm: f64,
        seconds: f64,
        freq: f64,
    ) -> (MicReader, Vec<f32>) {
        let (mut w, mut r) = bridge(in_rate, out_rate);
        let real_in = in_rate as f64 * (1.0 + ppm * 1e-6);
        let in_block = (in_rate / 100) as usize;
        let out_block = (out_rate / 100) as usize;
        let mut t_in = 0.0f64; // время следующего блока микрофона
        let mut t_out = 0.0f64;
        let mut n_in = 0u64;
        let mut out = Vec::new();
        let mut buf = vec![0.0f32; out_block * 2];
        while t_out < seconds {
            if t_in <= t_out {
                let block: Vec<f32> = (0..in_block)
                    .map(|k| {
                        let n = n_in + k as u64;
                        (2.0 * std::f64::consts::PI * freq * n as f64 / real_in).sin() as f32 * 0.5
                    })
                    .collect();
                n_in += in_block as u64;
                w.write_interleaved(&block, 1);
                t_in += in_block as f64 / real_in;
            } else {
                buf.iter_mut().for_each(|x| *x = 0.0);
                r.mix_into(&mut buf, out_block, 1.0, false);
                out.extend(buf.as_chunks::<2>().0.iter().map(|f| f[0]));
                t_out += out_block as f64 / out_rate as f64;
            }
        }
        (r, out)
    }

    #[test]
    fn drift_is_absorbed_without_underruns() {
        for ppm in [-300.0, 0.0, 300.0] {
            let (r, _) = simulate(48_000, 48_000, ppm, 120.0, 440.0);
            let st = r.stats();
            // одно опустошение допускаем — на старте, пока фазы блоков не устоялись
            assert!(
                st.underruns.load(Ordering::Relaxed) <= 1,
                "ppm={ppm}: опустошения"
            );
            assert_eq!(
                st.overflows.load(Ordering::Relaxed),
                0,
                "ppm={ppm}: переполнения"
            );
            // буфер держится около цели (≈ 10 + 10 + 2 мс), а не уползает
            let us = st.buffer_us.load(Ordering::Relaxed);
            assert!(us < 40_000, "ppm={ppm}: задержка кольца {us} мкс");
            // регулятор отработал дрейф в нужную сторону
            let rel = r.relative_step();
            if ppm > 0.0 {
                assert!(rel > 1.0, "ppm={ppm}: rel={rel}");
            }
            if ppm < 0.0 {
                assert!(rel < 1.0, "ppm={ppm}: rel={rel}");
            }
        }
    }

    #[test]
    fn rate_conversion_keeps_pitch() {
        // микрофон 44,1 кГц → кабель 48 кГц: частота тона на выходе должна остаться 1 кГц
        let (_, out) = simulate(44_100, 48_000, 0.0, 3.0, 1000.0);
        let tail = &out[48_000..]; // последние 2 с
        let crossings = tail
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count();
        let freq = crossings as f64 / (tail.len() as f64 / 48_000.0);
        assert!((freq - 1000.0).abs() < 5.0, "частота {freq}");
    }

    #[test]
    fn mute_and_volume() {
        let (mut w, mut r) = bridge(48_000, 48_000);
        for _ in 0..3 {
            w.write_interleaved(&[0.5f32; 480], 1);
        }
        let st = r.stats();
        let mut buf = vec![0.0f32; 960];
        // как в жизни: на каждый блок кабеля приходит блок микрофона
        let mut step = |buf: &mut [f32], muted: bool| {
            w.write_interleaved(&[0.5f32; 480], 1);
            r.mix_into(buf, 480, 0.5, muted);
        };
        step(&mut buf, false);
        // после разгона сглаживания громкости голос ≈ 0.25
        step(&mut buf, false);
        buf.iter_mut().for_each(|x| *x = 0.0);
        step(&mut buf, false);
        assert!((buf[900] - 0.25).abs() < 1e-3, "{}", buf[900]);
        // mute: в кабель ничего, а уровень для индикатора всё равно меряется
        st.peak.take();
        step(&mut buf, true);
        buf.iter_mut().for_each(|x| *x = 0.0);
        step(&mut buf, true);
        assert!(buf.iter().all(|x| x.abs() < 1e-6));
        assert!(st.peak.take() > 0.2);
    }

    #[test]
    fn stereo_mic_is_downmixed() {
        let (mut w, r) = bridge(48_000, 48_000);
        w.write_interleaved(&[1.0, 0.0, 0.0, 1.0], 2);
        assert_eq!(r.cons.slots(), 2);
        assert_eq!(r.stats().in_block.load(Ordering::Relaxed), 2);
    }
}
