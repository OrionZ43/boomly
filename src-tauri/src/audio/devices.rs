//! Устройства: перечисление WASAPI (cpal), детект VB-CABLE, выбор устройств по настройкам,
//! открытие потоков cpal для рендерера/микрофона.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, FromSample, SampleFormat, SizedSample, StreamConfig, SupportedBufferSize};

use super::mic::MicWriter;
use super::mixer::{Renderer, write_frames};
use super::types::{AudioDevice, CableStatus, DeviceList, VB_CABLE_URL};
use crate::error::{AppError, AppResult};

/// Устройство из перечисления вместе с хэндлом cpal.
pub struct DevInfo {
    pub info: AudioDevice,
    pub device: cpal::Device,
}

#[derive(Default)]
pub struct Snapshot {
    pub outputs: Vec<DevInfo>,
    pub inputs: Vec<DevInfo>,
}

impl Snapshot {
    pub fn list(&self) -> DeviceList {
        DeviceList {
            outputs: self.outputs.iter().map(|d| d.info.clone()).collect(),
            inputs: self.inputs.iter().map(|d| d.info.clone()).collect(),
        }
    }
    pub fn output(&self, id: &str) -> Option<&cpal::Device> {
        self.outputs
            .iter()
            .find(|d| d.info.id == id)
            .map(|d| &d.device)
    }
    pub fn input(&self, id: &str) -> Option<&cpal::Device> {
        self.inputs
            .iter()
            .find(|d| d.info.id == id)
            .map(|d| &d.device)
    }
}

fn norm(name: &str) -> String {
    name.to_lowercase()
}

/// Выход VB-CABLE (`CABLE Input (VB-Audio Virtual Cable)`, а также `CABLE-A Input` и т.п.).
pub fn is_cable_output_name(name: &str) -> bool {
    let n = norm(name);
    n.starts_with("cable") && n.contains("input") && (n.contains("vb-audio") || !n.contains('('))
}

/// Вход VB-CABLE — то, что Discord видит как микрофон (`CABLE Output (...)`).
pub fn is_cable_input_name(name: &str) -> bool {
    let n = norm(name);
    n.starts_with("cable") && n.contains("output") && (n.contains("vb-audio") || !n.contains('('))
}

fn describe(d: &cpal::Device) -> Option<(String, String)> {
    let id = d.id().ok()?.to_string();
    let name = d
        .description()
        .map(|x| x.name().to_string())
        .unwrap_or_else(|_| id.clone());
    Some((id, name))
}

/// Перечислить устройства (десятки мс; не из аудиопотока и не из главного потока).
pub fn enumerate(host: &cpal::Host) -> Snapshot {
    let def_out = host
        .default_output_device()
        .and_then(|d| d.id().ok())
        .map(|i| i.to_string());
    let def_in = host
        .default_input_device()
        .and_then(|d| d.id().ok())
        .map(|i| i.to_string());
    let mut snap = Snapshot::default();
    if let Ok(devs) = host.output_devices() {
        for d in devs {
            if let Some((id, name)) = describe(&d) {
                let info = AudioDevice {
                    is_default: def_out.as_deref() == Some(id.as_str()),
                    is_cable: is_cable_output_name(&name),
                    id,
                    name,
                };
                snap.outputs.push(DevInfo { info, device: d });
            }
        }
    }
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            if let Some((id, name)) = describe(&d) {
                let info = AudioDevice {
                    is_default: def_in.as_deref() == Some(id.as_str()),
                    is_cable: is_cable_input_name(&name),
                    id,
                    name,
                };
                snap.inputs.push(DevInfo { info, device: d });
            }
        }
    }
    snap
}

pub fn cable_status(list: &DeviceList) -> CableStatus {
    let out = list.outputs.iter().find(|d| d.is_cable);
    let inp = list.inputs.iter().find(|d| d.is_cable);
    CableStatus {
        installed: out.is_some() && inp.is_some(),
        output_device_name: out.map(|d| d.name.clone()),
        input_device_name: inp.map(|d| d.name.clone()),
        download_url: VB_CABLE_URL.into(),
    }
}

/// Какие устройства открыть.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolved {
    pub monitor: Option<String>,
    pub cable: Option<String>,
    pub mic: Option<String>,
}

/// Выбрать устройства по настройкам.
///
/// - «себе»: выбранное, иначе системное по умолчанию; кабель «себе» не берём никогда
///   (иначе звук задвоится в микрофон) — тогда первое не-кабельное устройство;
/// - «в микрофон»: выбранное, иначе первый найденный VB-CABLE; выбранное, но пропавшее — нет;
/// - микрофон: выбранный, иначе по умолчанию; `CABLE Output` никогда (петля самого себя).
pub fn resolve(
    list: &DeviceList,
    monitor: Option<&str>,
    cable: Option<&str>,
    mic: Option<&str>,
) -> Resolved {
    let pick = |devs: &[AudioDevice], want: Option<&str>| -> Option<String> {
        let ok = |d: &&AudioDevice| !d.is_cable;
        want.and_then(|w| devs.iter().filter(ok).find(|d| d.id == w))
            .or_else(|| devs.iter().filter(ok).find(|d| d.is_default))
            .or_else(|| devs.iter().find(ok))
            .map(|d| d.id.clone())
    };
    let cable = match cable {
        Some(id) => list
            .outputs
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.id.clone()),
        None => list
            .outputs
            .iter()
            .find(|d| d.is_cable)
            .map(|d| d.id.clone()),
    };
    let mut monitor = pick(&list.outputs, monitor);
    if monitor.is_some() && monitor == cable {
        monitor = None;
    }
    Resolved {
        monitor,
        cable,
        mic: pick(&list.inputs, mic),
    }
}

/// Задержки и размеры буферов открытого потока (для отчёта/диагностики).
#[derive(Debug, Default)]
pub struct StreamStats {
    /// Последний размер колбэка, кадров.
    pub callback_frames: AtomicU64,
    /// Задержка «колбэк → ЦАП» (выход) или «АЦП → колбэк» (вход), мкс.
    pub latency_us: AtomicU64,
}

pub struct OpenStream {
    pub stream: cpal::Stream,
    pub rate: u32,
    pub channels: u16,
    pub stats: Arc<StreamStats>,
}

fn dev_err(e: impl std::fmt::Display) -> AppError {
    AppError::new("device_unavailable", e.to_string())
}

/// Период, который просим у WASAPI: 10 мс — минимум shared-режима на большинстве драйверов.
pub const LOW_LATENCY_S: f64 = 0.010;

/// Конфиг с малым буфером (если драйвер сообщает допустимый диапазон — в его пределах).
fn low_latency(c: &cpal::SupportedStreamConfig) -> StreamConfig {
    let mut cfg = c.config();
    let want = (c.sample_rate() as f64 * LOW_LATENCY_S).round() as u32;
    cfg.buffer_size = match *c.buffer_size() {
        SupportedBufferSize::Range { min, max } => BufferSize::Fixed(want.clamp(min, max)),
        SupportedBufferSize::Unknown => BufferSize::Fixed(want),
    };
    cfg
}

/// Формат выхода: системный микс-формат WASAPI (shared mode), частоту не навязываем;
/// буфер — малый. Если драйвер его не примет, вызывающий повторит с `BufferSize::Default`.
pub fn output_config(device: &cpal::Device) -> AppResult<(StreamConfig, SampleFormat)> {
    let c = device.default_output_config().map_err(dev_err)?;
    Ok((low_latency(&c), c.sample_format()))
}

pub fn input_config(device: &cpal::Device) -> AppResult<(StreamConfig, SampleFormat)> {
    let c = device.default_input_config().map_err(dev_err)?;
    Ok((low_latency(&c), c.sample_format()))
}

macro_rules! dispatch_format {
    ($fmt:expr, $f:ident, $($args:expr),*) => {
        match $fmt {
            SampleFormat::F32 => $f::<f32>($($args),*),
            SampleFormat::F64 => $f::<f64>($($args),*),
            SampleFormat::I16 => $f::<i16>($($args),*),
            SampleFormat::I32 => $f::<i32>($($args),*),
            SampleFormat::U16 => $f::<u16>($($args),*),
            SampleFormat::U8 => $f::<u8>($($args),*),
            SampleFormat::I8 => $f::<i8>($($args),*),
            other => Err(AppError::new(
                "device_unavailable",
                format!("неподдерживаемый формат устройства: {other}"),
            )),
        }
    };
}

/// Открыть выход и запустить рендерер в его колбэке.
pub fn open_output(
    device: &cpal::Device,
    config: StreamConfig,
    format: SampleFormat,
    renderer: Renderer,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> AppResult<OpenStream> {
    dispatch_format!(format, build_output, device, config, renderer, on_error)
}

fn build_output<T>(
    device: &cpal::Device,
    config: StreamConfig,
    mut renderer: Renderer,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> AppResult<OpenStream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let stats = Arc::new(StreamStats::default());
    let st = stats.clone();
    // запас под большие колбэки; длиннее — обрабатываем по кускам
    let mut scratch = vec![0.0f32; 8192 * 2];
    let stream = device
        .build_output_stream::<T, _, _>(
            config,
            move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
                let frames = data.len() / channels.max(1);
                st.callback_frames.store(frames as u64, Ordering::Relaxed);
                let ts = info.timestamp();
                st.latency_us.store(
                    ts.playback.duration_since(ts.callback).as_micros() as u64,
                    Ordering::Relaxed,
                );
                let max = scratch.len() / 2;
                for (dst, start) in data.chunks_mut(max * channels).zip((0..).step_by(max)) {
                    let n = (frames - start).min(max);
                    let buf = &mut scratch[..n * 2];
                    renderer.render(buf);
                    write_frames(dst, buf, channels, T::from_sample, T::EQUILIBRIUM);
                }
            },
            on_error,
            None,
        )
        .map_err(dev_err)?;
    stream.play().map_err(dev_err)?;
    Ok(OpenStream {
        stream,
        rate: config.sample_rate,
        channels: config.channels,
        stats,
    })
}

/// Открыть микрофон: блоки сводятся в моно и уходят в мост.
pub fn open_input(
    device: &cpal::Device,
    config: StreamConfig,
    format: SampleFormat,
    writer: MicWriter,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> AppResult<OpenStream> {
    dispatch_format!(format, build_input, device, config, writer, on_error)
}

fn build_input<T>(
    device: &cpal::Device,
    config: StreamConfig,
    mut writer: MicWriter,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> AppResult<OpenStream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let stats = Arc::new(StreamStats::default());
    let st = stats.clone();
    let mut scratch = vec![0.0f32; 8192 * 8];
    let stream = device
        .build_input_stream::<T, _, _>(
            config,
            move |data: &[T], info: &cpal::InputCallbackInfo| {
                let ts = info.timestamp();
                st.callback_frames
                    .store((data.len() / channels.max(1)) as u64, Ordering::Relaxed);
                st.latency_us.store(
                    ts.callback.duration_since(ts.capture).as_micros() as u64,
                    Ordering::Relaxed,
                );
                let max = scratch.len() / channels.max(1) * channels.max(1);
                for chunk in data.chunks(max) {
                    let buf = &mut scratch[..chunk.len()];
                    for (d, s) in buf.iter_mut().zip(chunk) {
                        *d = s.to_sample::<f32>();
                    }
                    writer.write_interleaved(buf, channels);
                }
            },
            on_error,
            None,
        )
        .map_err(dev_err)?;
    stream.play().map_err(dev_err)?;
    Ok(OpenStream {
        stream,
        rate: config.sample_rate,
        channels: config.channels,
        stats,
    })
}

/// Размер периода потока, если драйвер его сообщает.
pub fn period_frames(s: &OpenStream) -> Option<u32> {
    s.stream.buffer_size().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: &str, name: &str, def: bool, cable: bool) -> AudioDevice {
        AudioDevice {
            id: id.into(),
            name: name.into(),
            is_default: def,
            is_cable: cable,
        }
    }

    #[test]
    fn cable_names() {
        assert!(is_cable_output_name("CABLE Input (VB-Audio Virtual Cable)"));
        assert!(is_cable_output_name("CABLE-A Input (VB-Audio Cable A)"));
        assert!(is_cable_output_name("CABLE Input"));
        assert!(is_cable_input_name("CABLE Output (VB-Audio Virtual Cable)"));
        assert!(!is_cable_output_name(
            "CABLE Output (VB-Audio Virtual Cable)"
        ));
        assert!(!is_cable_output_name("Динамики (Realtek(R) Audio)"));
        assert!(!is_cable_input_name("Микрофон (fifine Microphone)"));
        assert!(!is_cable_output_name(
            "VoiceMeeter Input (VB-Audio VoiceMeeter VAIO)"
        ));
        // HDMI-кабель от производителя — не VB-CABLE
        assert!(!is_cable_output_name(
            "Cable TV Input (NVIDIA High Definition Audio)"
        ));
    }

    fn list() -> DeviceList {
        DeviceList {
            outputs: vec![
                dev("o1", "Speakers", false, false),
                dev("o2", "Headphones", true, false),
                dev("oc", "CABLE Input (VB-Audio Virtual Cable)", false, true),
            ],
            inputs: vec![
                dev("ic", "CABLE Output (VB-Audio Virtual Cable)", true, true),
                dev("i1", "Mic", false, false),
            ],
        }
    }

    #[test]
    fn resolve_defaults_and_auto_cable() {
        let r = resolve(&list(), None, None, None);
        assert_eq!(r.monitor.as_deref(), Some("o2"));
        assert_eq!(r.cable.as_deref(), Some("oc"));
        // по умолчанию стоит CABLE Output — берём настоящий микрофон, а не петлю
        assert_eq!(r.mic.as_deref(), Some("i1"));
    }

    #[test]
    fn resolve_explicit_and_missing() {
        let r = resolve(&list(), Some("o1"), Some("oc"), Some("i1"));
        assert_eq!(
            (r.monitor.as_deref(), r.mic.as_deref()),
            (Some("o1"), Some("i1"))
        );
        // выбранного устройства нет — «себе» уходит на системное, кабель выключается
        let r = resolve(&list(), Some("gone"), Some("gone"), None);
        assert_eq!(r.monitor.as_deref(), Some("o2"));
        assert_eq!(r.cable, None);
        // кабель «себе» выбрать нельзя
        let r = resolve(&list(), Some("oc"), None, None);
        assert_eq!(r.monitor.as_deref(), Some("o2"));
    }

    #[test]
    fn resolve_without_cable() {
        let mut l = list();
        l.outputs.retain(|d| !d.is_cable);
        let r = resolve(&l, None, None, None);
        assert_eq!(r.cable, None);
        assert_eq!(r.monitor.as_deref(), Some("o2"));
    }

    #[test]
    fn cable_status_needs_both_ends() {
        let s = cable_status(&list());
        assert!(s.installed);
        assert_eq!(
            s.output_device_name.as_deref(),
            Some("CABLE Input (VB-Audio Virtual Cable)")
        );
        let mut l = list();
        l.inputs.retain(|d| !d.is_cable);
        assert!(!cable_status(&l).installed);
        assert_eq!(cable_status(&l).download_url, VB_CABLE_URL);
    }
}
