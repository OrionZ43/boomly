//! Чистые DSP-кирпичики без устройств: сглаженная громкость, лимитер, атомарный f32.
//! Всё здесь realtime-безопасно: без аллокаций и блокировок.

use std::sync::atomic::{AtomicU32, Ordering};

/// `f32`, который можно менять из потока команд и читать из аудиоколбэка без блокировок.
#[derive(Debug, Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    pub fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed)
    }
    /// Запомнить максимум (для пиковых измерителей).
    pub fn fetch_max(&self, v: f32) {
        let mut cur = self.0.load(Ordering::Relaxed);
        while v > f32::from_bits(cur) {
            match self.0.compare_exchange_weak(
                cur,
                v.to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => cur = x,
            }
        }
    }
    /// Прочитать и обнулить.
    pub fn take(&self) -> f32 {
        f32::from_bits(self.0.swap(0f32.to_bits(), Ordering::Relaxed))
    }
}

/// Громкость с линейным подходом к цели за `ramp_frames` кадров — без щелчков («zipper noise»).
#[derive(Debug, Clone)]
pub struct SmoothedGain {
    current: f32,
    target: f32,
    step: f32,
    ramp_frames: u32,
}

impl SmoothedGain {
    pub fn new(value: f32, sample_rate: u32) -> Self {
        // ~10 мс на смену громкости
        let ramp_frames = (sample_rate / 100).max(1);
        Self {
            current: value,
            target: value,
            step: 0.0,
            ramp_frames,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        if (target - self.target).abs() > f32::EPSILON {
            self.target = target;
            self.step = (target - self.current) / self.ramp_frames as f32;
        }
    }

    #[inline]
    pub fn next_gain(&mut self) -> f32 {
        if self.current != self.target {
            self.current += self.step;
            if (self.step > 0.0 && self.current >= self.target)
                || (self.step < 0.0 && self.current <= self.target)
                || self.step == 0.0
            {
                self.current = self.target;
            }
        }
        self.current
    }

    pub fn current(&self) -> f32 {
        self.current
    }
}

/// Пиковый лимитер с мгновенной атакой и экспоненциальным отпуском.
/// Гарантирует |выход| ≤ `threshold` для любого входа (без «перелёта»), звучит мягче,
/// чем жёсткое обрезание: при наложении нескольких громких звуков просто приглушает микс.
#[derive(Debug, Clone)]
pub struct Limiter {
    threshold: f32,
    gain: f32,
    release_coef: f32,
}

impl Limiter {
    pub fn new(sample_rate: u32) -> Self {
        // отпуск ~150 мс
        let release_s = 0.150;
        let release_coef = (-1.0 / (release_s * sample_rate as f32)).exp();
        Self {
            threshold: 0.98,
            gain: 1.0,
            release_coef,
        }
    }

    /// Обработать стерео-кадр на месте.
    #[inline]
    pub fn process_frame(&mut self, l: &mut f32, r: &mut f32) {
        let peak = l.abs().max(r.abs());
        // отпуск: тянем усиление обратно к 1
        self.gain = 1.0 - (1.0 - self.gain) * self.release_coef;
        if peak * self.gain > self.threshold {
            self.gain = self.threshold / peak;
        }
        *l *= self.gain;
        *r *= self.gain;
        // защита от NaN/inf из битого файла
        if !l.is_finite() {
            *l = 0.0;
        }
        if !r.is_finite() {
            *r = 0.0;
        }
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }
}

/// Кубическая интерполяция Эрмита (Catmull-Rom) между `y1` и `y2`, `t` в `0..1`.
#[inline]
pub fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let c0 = y1;
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * t + c2) * t + c1) * t + c0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothed_gain_reaches_target_without_jumps() {
        let mut g = SmoothedGain::new(0.0, 48_000);
        g.set_target(1.0);
        let mut prev = 0.0;
        for _ in 0..480 {
            let v = g.next_gain();
            assert!(v - prev <= 1.0 / 480.0 + 1e-6, "скачок громкости");
            prev = v;
        }
        assert_eq!(g.next_gain(), 1.0);
        g.set_target(0.25);
        for _ in 0..1000 {
            g.next_gain();
        }
        assert_eq!(g.current(), 0.25);
    }

    #[test]
    fn limiter_never_exceeds_threshold() {
        let mut lim = Limiter::new(48_000);
        for i in 0..48_000 {
            let x = 4.0 * ((i as f32) * 0.05).sin();
            let (mut l, mut r) = (x, -x * 0.5);
            lim.process_frame(&mut l, &mut r);
            assert!(l.abs() <= 0.98 + 1e-6 && r.abs() <= 0.98 + 1e-6);
        }
    }

    #[test]
    fn limiter_transparent_for_quiet_signal_and_recovers() {
        let mut lim = Limiter::new(48_000);
        let (mut l, mut r) = (0.5, 0.5);
        lim.process_frame(&mut l, &mut r);
        assert_eq!((l, r), (0.5, 0.5));
        // громкий всплеск, затем тишина 1 с — усиление возвращается почти к 1
        let (mut l, mut r) = (3.0, 3.0);
        lim.process_frame(&mut l, &mut r);
        assert!(lim.gain() < 0.4);
        for _ in 0..48_000 {
            let (mut l, mut r) = (0.0, 0.0);
            lim.process_frame(&mut l, &mut r);
        }
        assert!(lim.gain() > 0.99);
    }

    #[test]
    fn limiter_kills_nan() {
        let mut lim = Limiter::new(48_000);
        let (mut l, mut r) = (f32::NAN, 0.1);
        lim.process_frame(&mut l, &mut r);
        assert_eq!(l, 0.0);
        assert!(r.is_finite());
    }

    #[test]
    fn atomic_f32_peak() {
        let a = AtomicF32::new(0.0);
        a.fetch_max(0.3);
        a.fetch_max(0.1);
        assert_eq!(a.take(), 0.3);
        assert_eq!(a.get(), 0.0);
    }

    #[test]
    fn hermite_interpolates_line_exactly() {
        assert!((hermite(0.0, 1.0, 2.0, 3.0, 0.5) - 1.5).abs() < 1e-6);
        assert_eq!(hermite(0.0, 1.0, 2.0, 3.0, 0.0), 1.0);
    }
}
