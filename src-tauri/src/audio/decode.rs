//! Декодирование файлов (symphonia) в стерео f32 и ресемплинг клипов (rubato, sinc).
//!
//! Клип — целиком в памяти, interleaved L,R, на частоте конкретного выхода: в аудиоколбэке
//! никакого ресемплинга звуков нет, только сложение — поэтому старт мгновенный.

use std::fs::File;
use std::path::Path;

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::error::{AppError, AppResult};

/// Звук в памяти: стерео, interleaved.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    pub rate: u32,
    pub samples: Vec<f32>,
}

impl Clip {
    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }
    pub fn duration_ms(&self) -> u64 {
        if self.rate == 0 {
            return 0;
        }
        self.frames() as u64 * 1000 / self.rate as u64
    }
    pub fn bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<f32>()
    }
}

fn decode_err(path: &Path, e: impl std::fmt::Display) -> AppError {
    AppError::new("decode", format!("{}: {e}", path.display()))
}

/// Декодировать файл в стерео на его родной частоте.
pub fn decode_file(path: &Path) -> AppResult<Clip> {
    let file = File::open(path).map_err(|e| {
        let code = if e.kind() == std::io::ErrorKind::NotFound {
            "not_found"
        } else {
            "io"
        };
        AppError::new(code, format!("{}: {e}", path.display()))
    })?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| decode_err(path, e))?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| decode_err(path, "нет аудиодорожки"))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| decode_err(path, "нет параметров кодека"))?
        .clone();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|e| decode_err(path, e))?;

    let mut rate = params.sample_rate.unwrap_or(0);
    let mut out: Vec<f32> = Vec::new();
    let mut tmp: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::ResetRequired) => break,
            Err(e) => return Err(decode_err(path, e)),
        };
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buf) => {
                let channels = buf.spec().channels().count().max(1);
                if rate == 0 {
                    rate = buf.spec().rate();
                }
                tmp.resize(buf.samples_interleaved(), 0.0);
                buf.copy_to_slice_interleaved(&mut tmp);
                append_stereo(&mut out, &tmp, channels);
            }
            // битый пакет — пропускаем, как делают плееры
            Err(SymError::DecodeError(_)) => continue,
            Err(e) => return Err(decode_err(path, e)),
        }
    }
    if rate == 0 || out.is_empty() {
        return Err(decode_err(path, "пустой или неподдерживаемый файл"));
    }
    Ok(Clip { rate, samples: out })
}

/// Длительность по заголовку, без полного декодирования (для импорта в библиотеку, B2).
/// `None` — формат не распознан или контейнер не хранит длину.
pub fn probe_duration_ms(path: &Path) -> Option<u64> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .ok()?;
    let track = format.default_track(TrackType::Audio)?;
    let frames = track.num_frames?;
    let rate = track.codec_params.as_ref()?.audio()?.sample_rate?;
    (rate > 0).then(|| frames * 1000 / rate as u64)
}

/// Добавить interleaved-блок с `channels` каналами как стерео.
/// Моно → в оба канала; больше двух → берём фронтальные L,R (остальное подмешиваем с −6 дБ).
pub fn append_stereo(out: &mut Vec<f32>, data: &[f32], channels: usize) {
    match channels {
        1 => {
            out.reserve(data.len() * 2);
            for &s in data {
                out.push(s);
                out.push(s);
            }
        }
        2 => out.extend_from_slice(data),
        n => {
            out.reserve(data.len() / n * 2);
            for f in data.chunks_exact(n) {
                let rest: f32 = f[2..].iter().sum::<f32>() * 0.5 / (n - 2) as f32;
                out.push(f[0] + rest);
                out.push(f[1] + rest);
            }
        }
    }
}

/// Пересчитать клип на другую частоту (качественный sinc-ресемплер, вне аудиопотока).
pub fn resample(clip: &Clip, rate: u32) -> AppResult<Clip> {
    if clip.rate == rate || clip.samples.is_empty() {
        return Ok(Clip {
            rate,
            samples: clip.samples.clone(),
        });
    }
    let ratio = rate as f64 / clip.rate as f64;
    let params = SincInterpolationParameters::new(128, WindowFunction::Blackman2)
        .oversampling_factor(256)
        .interpolation(SincInterpolationType::Linear);
    let err = |e: &dyn std::fmt::Display| AppError::new("internal", format!("resample: {e}"));
    let mut rs = Async::<f32>::new_sinc(ratio, 1.0, &params, 1024, 2, FixedAsync::Input)
        .map_err(|e| err(&e))?;
    let in_frames = clip.frames();
    let input = InterleavedSlice::new(&clip.samples, 2, in_frames).map_err(|e| err(&e))?;
    let out_frames = rs.process_all_needed_output_len(in_frames);
    let mut buf = vec![0.0f32; out_frames * 2];
    let mut output = InterleavedSlice::new_mut(&mut buf, 2, out_frames).map_err(|e| err(&e))?;
    let (_, produced) = rs
        .process_all_into_buffer(&input, &mut output, in_frames, None)
        .map_err(|e| err(&e))?;
    buf.truncate(produced * 2);
    Ok(Clip { rate, samples: buf })
}

/// Тестовый звук для мастера: два мягких «динь» (A5 → E6), ~0,6 с.
pub fn test_chime(rate: u32) -> Clip {
    let tone = |freq: f32, start: f32, len: f32, t: f32| -> f32 {
        let x = t - start;
        if !(0.0..len).contains(&x) {
            return 0.0;
        }
        let env = (x * 200.0).min(1.0) * (-x * 6.0).exp();
        (2.0 * std::f32::consts::PI * freq * x).sin() * env * 0.35
    };
    let frames = (rate as f32 * 0.65) as usize;
    let mut samples = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / rate as f32;
        let s = tone(880.0, 0.0, 0.3, t) + tone(1318.5, 0.18, 0.47, t);
        samples.push(s);
        samples.push(s);
    }
    Clip { rate, samples }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, freq: f32, seconds: f32) -> Clip {
        let n = (rate as f32 * seconds) as usize;
        let mut samples = Vec::with_capacity(n * 2);
        for i in 0..n {
            let s = (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin() * 0.5;
            samples.push(s);
            samples.push(s);
        }
        Clip { rate, samples }
    }

    fn freq_of(clip: &Clip) -> f32 {
        let l: Vec<f32> = clip.samples.iter().step_by(2).copied().collect();
        let skip = clip.rate as usize / 10;
        let tail = &l[skip..l.len() - skip];
        let crossings = tail
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count();
        crossings as f32 / (tail.len() as f32 / clip.rate as f32)
    }

    #[test]
    fn resample_44k1_to_48k_keeps_length_and_pitch() {
        let src = sine(44_100, 1000.0, 1.0);
        let dst = resample(&src, 48_000).unwrap();
        assert_eq!(dst.rate, 48_000);
        assert!(
            (dst.frames() as i64 - 48_000).abs() <= 2,
            "кадров {}",
            dst.frames()
        );
        assert!(
            (freq_of(&dst) - 1000.0).abs() < 2.0,
            "частота {}",
            freq_of(&dst)
        );
        // амплитуда сохраняется (sinc без заметных потерь на 1 кГц)
        let peak = dst.samples[4800..dst.samples.len() - 4800]
            .iter()
            .fold(0.0f32, |m, x| m.max(x.abs()));
        assert!((peak - 0.5).abs() < 0.01, "пик {peak}");
    }

    #[test]
    fn resample_48k_to_44k1() {
        let dst = resample(&sine(48_000, 440.0, 2.0), 44_100).unwrap();
        assert!((dst.frames() as i64 - 88_200).abs() <= 2);
        assert!((freq_of(&dst) - 440.0).abs() < 3.0);
    }

    #[test]
    fn same_rate_is_copy() {
        let src = sine(48_000, 440.0, 0.1);
        assert_eq!(resample(&src, 48_000).unwrap(), src);
    }

    #[test]
    fn channel_layouts_to_stereo() {
        let mut out = vec![];
        append_stereo(&mut out, &[0.1, 0.2], 1);
        assert_eq!(out, vec![0.1, 0.1, 0.2, 0.2]);
        let mut out = vec![];
        append_stereo(&mut out, &[0.1, 0.2, 0.3, 0.4], 2);
        assert_eq!(out, vec![0.1, 0.2, 0.3, 0.4]);
        let mut out = vec![];
        // 5.1 → центр и прочее делится поровну в оба канала
        append_stereo(&mut out, &[1.0, 0.0, 0.4, 0.4, 0.4, 0.4], 6);
        assert!((out[0] - 1.2).abs() < 1e-6 && (out[1] - 0.2).abs() < 1e-6);
    }

    /// Минимальный WAV PCM16 — проверяем полный путь через symphonia без внешних файлов.
    fn write_wav(path: &Path, rate: u32, channels: u16, samples: &[i16]) {
        let data_len = (samples.len() * 2) as u32;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data_len).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&channels.to_le_bytes());
        b.extend_from_slice(&rate.to_le_bytes());
        b.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
        b.extend_from_slice(&(channels * 2).to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            b.extend_from_slice(&s.to_le_bytes());
        }
        std::fs::write(path, b).unwrap();
    }

    #[test]
    fn decode_wav_mono() {
        let dir = std::env::temp_dir().join(format!("boomly-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mono.wav");
        let samples: Vec<i16> = (0..22_050)
            .map(|i| if i % 2 == 0 { 16384 } else { -16384 })
            .collect();
        write_wav(&path, 22_050, 1, &samples);
        let clip = decode_file(&path).unwrap();
        assert_eq!(probe_duration_ms(&path), Some(1000));
        assert_eq!(clip.rate, 22_050);
        assert_eq!(clip.frames(), 22_050);
        assert_eq!(clip.duration_ms(), 1000);
        assert!((clip.samples[0] - 0.5).abs() < 1e-3 && clip.samples[0] == clip.samples[1]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn decode_missing_and_garbage() {
        let err = decode_file(Path::new("Z:/нет/такого.mp3")).unwrap_err();
        assert_eq!(err.code, "not_found");
        let dir = std::env::temp_dir().join(format!("boomly-test-g-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("junk.mp3");
        std::fs::write(&path, b"definitely not audio").unwrap();
        assert_eq!(decode_file(&path).unwrap_err().code, "decode");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn chime_is_sane() {
        let c = test_chime(48_000);
        assert!(c.duration_ms() > 500);
        assert!(c.samples.iter().all(|x| x.abs() < 0.8));
    }
}
