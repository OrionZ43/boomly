//! Ручные тесты на реальных устройствах (не идут в CI):
//! `cargo test hw_ -- --ignored --nocapture --test-threads=1`.
//! Печатают устройства, форматы и замеры задержки. Тестовый «динь» прозвучит в наушниках.

use std::time::Duration;

use super::*;

fn started() -> AudioEngine {
    let e = AudioEngine::new();
    e.spawn_threads(true);
    for _ in 0..100 {
        if e.inner.devices_ready.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    e
}

#[test]
#[ignore = "нужны реальные аудиоустройства"]
fn hw_list_and_play_monitor() {
    let e = started();
    let list = e.list_devices().unwrap();
    println!("outputs:");
    for d in &list.outputs {
        println!(
            "  {:?} default={} cable={}  [{}]",
            d.name, d.is_default, d.is_cable, d.id
        );
    }
    println!("inputs:");
    for d in &list.inputs {
        println!(
            "  {:?} default={} cable={}  [{}]",
            d.name, d.is_default, d.is_cable, d.id
        );
    }
    println!("cable: {:?}", e.cable_status());
    let t = std::time::Instant::now();
    e.play_test(TestTarget::Monitor).unwrap();
    println!("play_test вернулся за {:?}", t.elapsed());
    std::thread::sleep(Duration::from_millis(800));
    println!(
        "{}",
        serde_json::to_string_pretty(&e.latency_report()).unwrap()
    );
}

/// Без VB-CABLE: «кабелем» назначаем второе реальное устройство вывода, чтобы проверить
/// сквозной микрофон и дрейф часов между разными устройствами.
#[test]
#[ignore = "нужны реальные аудиоустройства"]
fn hw_mic_passthrough_drift() {
    let e = started();
    let list = e.list_devices().unwrap();
    let cable = list
        .outputs
        .iter()
        .find(|d| d.is_cable)
        .or_else(|| list.outputs.iter().find(|d| !d.is_default))
        .expect("нужно второе устройство вывода");
    println!("«кабель»: {}", cable.name);
    e.set_devices(DeviceSelection {
        monitor_device_id: None,
        cable_device_id: Some(Some(cable.id.clone())),
        mic_device_id: None,
    })
    .unwrap();
    e.set_mic_passthrough(true).unwrap();
    std::thread::sleep(Duration::from_secs(2));
    let mut max_buf = 0;
    let mut min_buf = u64::MAX;
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(250));
        let r = e.latency_report();
        if let Some(b) = r.mic_buffer_us {
            max_buf = max_buf.max(b);
            min_buf = min_buf.min(b);
        }
    }
    let r = e.latency_report();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
    println!("кольцо микрофона за 10 с: {min_buf}..{max_buf} мкс");
    assert!(r.mic.is_some(), "микрофон не открылся");
}
