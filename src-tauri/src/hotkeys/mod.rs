//! Участок B2 — глобальные хоткеи (tauri-plugin-global-shortcut). Хранятся в
//! `%APPDATA%\Boomly\hotkeys.json`. Действие выполняется в Rust (через `AudioEngine`),
//! затем шлётся `hotkeys://triggered`.
//!
//! Одно действие — одно сочетание. Сочетание, занятое другим действием Boomly или другой
//! программой (ОС не даёт зарегистрировать), — ошибка `hotkey_conflict`.

pub mod commands;
pub mod types;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::audio::AudioEngine;
use crate::error::{AppError, AppResult};
use crate::events;
use crate::library::Library;
use crate::system::store;
use types::{HotkeyAction, HotkeyBinding, HotkeyTriggered};

pub const HOTKEYS_FILE: &str = "hotkeys.json";
const SCHEMA_VERSION: u32 = 1;

/// Регистрирует плагины и setup участка. Вызывается из `lib.rs`.
pub fn init<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let hk = app.state::<Hotkeys>();
                    if hk.debounce(shortcut.id(), event.state() == ShortcutState::Pressed) {
                        hk.trigger(app, shortcut);
                    }
                })
                .build(),
        )
        // Свой мини-плагин, чтобы не занимать единственный `Builder::setup` приложения.
        .plugin(
            tauri::plugin::Builder::<R>::new("boomly-hotkeys")
                .setup(|app, _api| {
                    let dir = crate::paths::data_dir(app)?;
                    if let Err(e) = app.state::<Hotkeys>().open(app, &dir) {
                        eprintln!("[boomly] hotkeys.json: {e}");
                    }
                    Ok(())
                })
                .build(),
        )
}

#[derive(Serialize)]
struct FileOut<'a> {
    version: u32,
    bindings: &'a [HotkeyBinding],
}

#[derive(Deserialize)]
struct FileIn {
    #[serde(default)]
    bindings: Vec<Value>,
}

pub fn parse(accelerator: &str) -> AppResult<Shortcut> {
    Shortcut::from_str(accelerator.trim())
        .map_err(|e| AppError::new("invalid_argument", format!("сочетание «{accelerator}»: {e}")))
}

fn same_keys(a: &str, b: &Shortcut) -> bool {
    parse(a).map(|s| s.id() == b.id()).unwrap_or(false)
}

/// Что сделать с привязками при `hotkeys_set` (чистая логика, без ОС).
#[derive(Debug, PartialEq)]
pub struct SetPlan {
    pub next: Vec<HotkeyBinding>,
    /// Прежнее сочетание того же действия — снять с регистрации.
    pub replaced: Option<HotkeyBinding>,
    /// Ничего не изменилось (то же действие на тех же клавишах).
    pub unchanged: bool,
}

pub fn plan_set(bindings: &[HotkeyBinding], new: HotkeyBinding) -> AppResult<SetPlan> {
    let sc = parse(&new.accelerator)?;
    let new = HotkeyBinding { accelerator: new.accelerator.trim().to_string(), ..new };
    if let Some(other) = bindings.iter().find(|b| b.action != new.action && same_keys(&b.accelerator, &sc)) {
        return Err(AppError::new(
            "hotkey_conflict",
            format!("{} уже назначено на {:?}", other.accelerator, other.action),
        ));
    }
    let replaced = bindings.iter().find(|b| b.action == new.action).cloned();
    let unchanged = replaced.as_ref().is_some_and(|r| same_keys(&r.accelerator, &sc));
    let mut next: Vec<_> = bindings.iter().filter(|b| b.action != new.action).cloned().collect();
    next.push(new);
    Ok(SetPlan { next, replaced, unchanged })
}

#[derive(Default, Clone)]
struct Inner {
    bindings: Vec<HotkeyBinding>,
    file: Option<PathBuf>,
    /// UI записывает новое сочетание — наши хоткеи сняты с регистрации.
    capturing: bool,
}

/// Блокировки: `op` сериализует изменения и держится, пока идут вызовы ОС (они ждут главный
/// поток); `inner` берётся только на мгновение. Обработчик нажатия (главный поток) берёт
/// лишь `inner`, поэтому не может встать в очередь за операцией, которая ждёт главный поток.
/// Изменяющие операции нельзя звать с главного потока — команды для этого асинхронные.
#[derive(Default)]
pub struct Hotkeys {
    op: Mutex<()>,
    inner: Mutex<Inner>,
    /// Зажатые сейчас сочетания: одно нажатие — одно срабатывание.
    pressed: Mutex<HashSet<u32>>,
}

fn conflict_os(acc: &str, e: impl std::fmt::Display) -> AppError {
    AppError::new("hotkey_conflict", format!("{acc} занято другой программой ({e})"))
}

fn persist(file: &Option<PathBuf>, bindings: &[HotkeyBinding]) -> AppResult<()> {
    match file {
        Some(f) => store::write_json_atomic(f, &FileOut { version: SCHEMA_VERSION, bindings }),
        None => Ok(()),
    }
}

impl Hotkeys {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn op(&self) -> MutexGuard<'_, ()> {
        self.op.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn commit(&self, next: Vec<HotkeyBinding>) -> Vec<HotkeyBinding> {
        self.lock().bindings = next.clone();
        next
    }

    pub fn list(&self) -> Vec<HotkeyBinding> {
        self.lock().bindings.clone()
    }

    /// Читает `hotkeys.json` и регистрирует сочетания. Не зарегистрированные (заняты другой
    /// программой) остаются в списке — пользователь увидит и переназначит.
    pub fn open<R: Runtime>(&self, app: &AppHandle<R>, data_dir: &Path) -> AppResult<()> {
        let _op = self.op();
        let file = data_dir.join(HOTKEYS_FILE);
        let bindings = load_bindings(&file)?;
        let gs = app.global_shortcut();
        for b in &bindings {
            if let Err(e) = parse(&b.accelerator).and_then(|s| gs.register(s).map_err(|e| conflict_os(&b.accelerator, e))) {
                eprintln!("[boomly] хоткей {}: {e}", b.accelerator);
            }
        }
        *self.lock() = Inner { bindings, file: Some(file), capturing: false };
        Ok(())
    }

    pub fn set<R: Runtime>(&self, app: &AppHandle<R>, binding: HotkeyBinding) -> AppResult<Vec<HotkeyBinding>> {
        if let HotkeyAction::PlaySound { sound_id } = &binding.action
            && app.state::<Library>().sound(sound_id).is_none()
        {
            return Err(AppError::new("not_found", format!("звук {sound_id} не найден")));
        }
        let _op = self.op();
        let cur = self.lock().clone();
        let plan = plan_set(&cur.bindings, binding.clone())?;
        let gs = app.global_shortcut();
        let new_sc = parse(&binding.accelerator)?;
        if plan.unchanged {
            // Те же клавиши. Если при старте их держала другая программа — пробуем снова.
            if !cur.capturing && !gs.is_registered(new_sc) {
                gs.register(new_sc).map_err(|e| conflict_os(&binding.accelerator, e))?;
            }
            persist(&cur.file, &plan.next)?;
            return Ok(self.commit(plan.next));
        }
        let old_sc = plan.replaced.as_ref().and_then(|r| parse(&r.accelerator).ok());
        if !cur.capturing {
            if let Some(old) = old_sc {
                let _ = gs.unregister(old);
            }
            if let Err(e) = gs.register(new_sc) {
                if let Some(old) = old_sc {
                    let _ = gs.register(old);
                }
                return Err(conflict_os(&binding.accelerator, e));
            }
        }
        if let Err(e) = persist(&cur.file, &plan.next) {
            if !cur.capturing {
                let _ = gs.unregister(new_sc);
                if let Some(old) = old_sc {
                    let _ = gs.register(old);
                }
            }
            return Err(e);
        }
        Ok(self.commit(plan.next))
    }

    pub fn remove<R: Runtime>(&self, app: &AppHandle<R>, accelerator: &str) -> AppResult<Vec<HotkeyBinding>> {
        let _op = self.op();
        let cur = self.lock().clone();
        let target = parse(accelerator).ok();
        let pos = cur
            .bindings
            .iter()
            .position(|b| b.accelerator == accelerator.trim() || target.as_ref().is_some_and(|t| same_keys(&b.accelerator, t)))
            .ok_or_else(|| AppError::new("not_found", format!("хоткей {accelerator} не назначен")))?;
        let mut next = cur.bindings.clone();
        let removed = next.remove(pos);
        persist(&cur.file, &next)?;
        if !cur.capturing
            && let Ok(sc) = parse(&removed.accelerator)
        {
            let _ = app.global_shortcut().unregister(sc);
        }
        Ok(self.commit(next))
    }

    /// Снимает хоткеи удалённых звуков (зовёт библиотека; не с главного потока).
    pub fn remove_for_sounds<R: Runtime>(&self, app: &AppHandle<R>, ids: &[String]) {
        let _op = self.op();
        let cur = self.lock().clone();
        let (gone, keep): (Vec<_>, Vec<_>) = cur
            .bindings
            .into_iter()
            .partition(|b| matches!(&b.action, HotkeyAction::PlaySound { sound_id } if ids.contains(sound_id)));
        if gone.is_empty() {
            return;
        }
        if let Err(e) = persist(&cur.file, &keep) {
            eprintln!("[boomly] hotkeys.json: {e}");
        }
        if !cur.capturing {
            for b in &gone {
                if let Ok(sc) = parse(&b.accelerator) {
                    let _ = app.global_shortcut().unregister(sc);
                }
            }
        }
        self.commit(keep);
    }

    pub fn set_capture<R: Runtime>(&self, app: &AppHandle<R>, active: bool) {
        let _op = self.op();
        let bindings = {
            let mut inner = self.lock();
            if inner.capturing == active {
                return;
            }
            inner.capturing = active;
            inner.bindings.clone()
        };
        let gs = app.global_shortcut();
        for b in &bindings {
            let Ok(sc) = parse(&b.accelerator) else { continue };
            if active {
                let _ = gs.unregister(sc);
            } else if let Err(e) = gs.register(sc) {
                eprintln!("[boomly] хоткей {}: {e}", b.accelerator);
            }
        }
        self.pressed.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// `true`, если это новое нажатие (не автоповтор и не отпускание).
    fn debounce(&self, id: u32, pressed: bool) -> bool {
        let mut set = self.pressed.lock().unwrap_or_else(|e| e.into_inner());
        if pressed {
            set.insert(id)
        } else {
            set.remove(&id);
            false
        }
    }

    fn trigger<R: Runtime>(&self, app: &AppHandle<R>, shortcut: &Shortcut) {
        let binding = {
            let inner = self.lock();
            if inner.capturing {
                return;
            }
            inner.bindings.iter().find(|b| same_keys(&b.accelerator, shortcut)).cloned()
        };
        let Some(b) = binding else { return };
        #[cfg(debug_assertions)]
        eprintln!("[boomly] hotkey {} -> {:?}", b.accelerator, b.action);
        if let Err(e) = run_action(app, &b.action) {
            eprintln!("[boomly] хоткей {}: {e}", b.accelerator);
            let _ = app.emit(events::AUDIO_ERROR, &e);
        }
        let _ = app.emit(events::HOTKEYS_TRIGGERED, HotkeyTriggered { accelerator: b.accelerator, action: b.action });
    }
}

/// Выполняет действие хоткея. Работает и при свёрнутом окне.
pub fn run_action<R: Runtime>(app: &AppHandle<R>, action: &HotkeyAction) -> AppResult<()> {
    let engine = app.state::<AudioEngine>();
    match action {
        HotkeyAction::PlaySound { sound_id } => {
            let sound = app
                .state::<Library>()
                .sound(sound_id)
                .ok_or_else(|| AppError::new("not_found", format!("звук {sound_id} не найден")))?;
            engine.play(&sound)
        }
        HotkeyAction::StopAll => engine.stop_all(),
        HotkeyAction::ToggleMicMute => {
            // `audio://settings-changed` шлёт сам движок из сеттера.
            engine.set_mic_muted(!engine.settings().mic_muted).map(|_| ())
        }
    }
}

/// Читает привязки, пропуская испорченные записи и повторы. Привязки к звукам, которых
/// сейчас нет в библиотеке, сохраняем: библиотека могла не открыться, а при удалении звука
/// его хоткей снимает `remove_for_sounds`.
fn load_bindings(file: &Path) -> AppResult<Vec<HotkeyBinding>> {
    let Some(v) = store::read_json(file)?.into_value() else { return Ok(vec![]) };
    let raw = serde_json::from_value::<FileIn>(v).map(|f| f.bindings).unwrap_or_default();
    let mut out: Vec<HotkeyBinding> = vec![];
    for item in raw {
        let Ok(b) = serde_json::from_value::<HotkeyBinding>(item) else { continue };
        if parse(&b.accelerator).is_err() {
            continue;
        }
        if out.iter().any(|o| o.action == b.action) {
            continue;
        }
        if let Ok(sc) = parse(&b.accelerator)
            && out.iter().any(|o| same_keys(&o.accelerator, &sc))
        {
            continue;
        }
        out.push(b);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(acc: &str, id: &str) -> HotkeyBinding {
        HotkeyBinding { accelerator: acc.into(), action: HotkeyAction::PlaySound { sound_id: id.into() } }
    }
    fn stop(acc: &str) -> HotkeyBinding {
        HotkeyBinding { accelerator: acc.into(), action: HotkeyAction::StopAll }
    }

    #[test]
    fn parses_contract_examples() {
        assert!(parse("Ctrl+Shift+F1").is_ok());
        assert!(parse("Alt+Numpad3").is_ok());
        assert_eq!(parse("Ctrl+Банан").unwrap_err().code, "invalid_argument");
        assert_eq!(parse("").unwrap_err().code, "invalid_argument");
    }

    #[test]
    fn conflict_with_other_action_even_if_spelled_differently() {
        let cur = vec![stop("Ctrl+Shift+S")];
        let err = plan_set(&cur, play("control+shift+KeyS", "a")).unwrap_err();
        assert_eq!(err.code, "hotkey_conflict");
    }

    #[test]
    fn replaces_binding_of_same_action() {
        let cur = vec![play("F1", "a"), stop("F12")];
        let plan = plan_set(&cur, play("F2", "a")).unwrap();
        assert_eq!(plan.replaced, Some(play("F1", "a")));
        assert!(!plan.unchanged);
        assert_eq!(plan.next, vec![stop("F12"), play("F2", "a")]);
    }

    #[test]
    fn same_action_same_keys_is_unchanged() {
        let cur = vec![play("Ctrl+F1", "a")];
        let plan = plan_set(&cur, play("Control+F1", "a")).unwrap();
        assert!(plan.unchanged);
        assert_eq!(plan.next.len(), 1);
    }

    #[test]
    fn different_sounds_are_different_actions() {
        let cur = vec![play("F1", "a")];
        assert_eq!(plan_set(&cur, play("F1", "b")).unwrap_err().code, "hotkey_conflict");
        assert_eq!(plan_set(&cur, play("F2", "b")).unwrap().next.len(), 2);
    }

    #[test]
    fn debounce_ignores_autorepeat() {
        let h = Hotkeys::new();
        assert!(h.debounce(7, true));
        assert!(!h.debounce(7, true), "автоповтор");
        assert!(!h.debounce(7, false), "отпускание");
        assert!(h.debounce(7, true));
    }

    #[test]
    fn load_skips_broken_entries_but_keeps_sound_bindings() {
        // Библиотека при старте могла не открыться: хоткеи звуков терять нельзя.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(HOTKEYS_FILE);
        std::fs::write(
            &file,
            serde_json::to_vec(&serde_json::json!({"version": 1, "bindings": [
                {"accelerator": "F12", "action": {"type": "stopAll"}},
                {"accelerator": "F11", "action": {"type": "playSound", "soundId": "deleted"}},
                {"accelerator": "Ctrl+???", "action": {"type": "toggleMicMute"}},
                {"accelerator": "F10", "action": {"type": "future"}},
                {"accelerator": "F9", "action": {"type": "stopAll"}},
                {"accelerator": "F12", "action": {"type": "toggleMicMute"}},
                {"accelerator": "F8", "action": {"type": "toggleMicMute"}}
            ]}))
            .unwrap(),
        )
        .unwrap();
        let got = load_bindings(&file).unwrap();
        assert_eq!(got, vec![stop("F12"), play("F11", "deleted"), HotkeyBinding { accelerator: "F8".into(), action: HotkeyAction::ToggleMicMute }]);
    }
}
