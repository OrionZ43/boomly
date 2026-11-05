//! Миграции схемы `library.json`. Каждая функция поднимает JSON ровно на одну версию.
//! Работаем на уровне `serde_json::Value`, чтобы старые файлы читались, даже если в них
//! нет полей, которые теперь обязательны в Rust-типах.
use serde_json::{Map, Value, json};

use super::types::{Board, LibraryData, Sound};
use crate::error::{AppError, AppResult};

/// Текущая версия схемы. Поднимая её, добавь шаг в `step` и тест.
pub const CURRENT_VERSION: u32 = 1;

/// Результат приведения файла к текущей схеме.
#[derive(Debug)]
pub struct Migrated {
    pub value: Value,
    /// С какой версии поднимали (`None` — уже текущая).
    pub from: Option<u32>,
    /// Файл записан более новой версией приложения: читаем, что понимаем,
    /// но перед первой записью сохраняем оригинал отдельно.
    pub newer: Option<u32>,
}

fn version_of(v: &Value) -> u32 {
    // Файлы до введения версии (ранние сборки) считаем версией 0.
    v.get("version").and_then(Value::as_u64).map(|n| n as u32).unwrap_or(0)
}

pub fn migrate(mut value: Value) -> AppResult<Migrated> {
    if !value.is_object() {
        return Err(AppError::new("decode", "library.json: ожидался объект"));
    }
    let start = version_of(&value);
    if start > CURRENT_VERSION {
        return Ok(Migrated { value, from: None, newer: Some(start) });
    }
    let mut v = start;
    while v < CURRENT_VERSION {
        value = step(v, value)?;
        v += 1;
        value["version"] = json!(v);
    }
    Ok(Migrated { value, from: (start != CURRENT_VERSION).then_some(start), newer: None })
}

fn step(from: u32, value: Value) -> AppResult<Value> {
    match from {
        0 => v0_to_v1(value),
        _ => Err(AppError::new("internal", format!("нет миграции со схемы {from}"))),
    }
}

/// v0 → v1: проставить значения по умолчанию для полей, которых могло не быть.
/// Также лечит частично заполненные записи (ручная правка файла).
fn v0_to_v1(mut value: Value) -> AppResult<Value> {
    let obj = value.as_object_mut().expect("checked in migrate");
    let sounds = obj.entry("sounds").or_insert_with(|| json!([]));
    if let Some(list) = sounds.as_array_mut() {
        list.retain(|s| s.get("id").and_then(Value::as_str).is_some() && s.get("path").and_then(Value::as_str).is_some());
        for s in list.iter_mut().filter_map(Value::as_object_mut) {
            fill_sound_defaults(s);
        }
    }
    let boards = obj.entry("boards").or_insert_with(|| json!([]));
    if let Some(list) = boards.as_array_mut() {
        list.retain(|b| b.get("id").and_then(Value::as_str).is_some());
        for b in list.iter_mut().filter_map(Value::as_object_mut) {
            fill_board_defaults(b);
        }
    }
    Ok(value)
}

/// Значение отсутствует или `null` там, где поле обязательно.
fn set_default(s: &mut Map<String, Value>, key: &str, default: impl FnOnce() -> Value) {
    if s.get(key).is_none_or(Value::is_null) {
        s.insert(key.into(), default());
    }
}

/// Чинит запись звука: обязательные поля без значения получают умолчания.
pub fn fill_sound_defaults(s: &mut Map<String, Value>) {
    let stem = s
        .get("path")
        .and_then(Value::as_str)
        .and_then(|p| std::path::Path::new(p).file_stem().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Звук".into());
    set_default(s, "name", || json!(stem));
    set_default(s, "color", || json!(super::model::DEFAULT_COLOR));
    set_default(s, "volume", || json!(1.0));
    set_default(s, "mode", || json!("toggle"));
    set_default(s, "favorite", || json!(false));
    set_default(s, "addedAt", || json!(crate::system::store::now_ms()));
    s.entry("emoji").or_insert(Value::Null);
    s.entry("durationMs").or_insert(Value::Null);
}

pub fn fill_board_defaults(b: &mut Map<String, Value>) {
    set_default(b, "name", || json!("Доска"));
    set_default(b, "soundIds", || json!([]));
    b.entry("emoji").or_insert(Value::Null);
}

/// Разбирает данные текущей схемы поэлементно: одна испорченная запись не обнуляет всю
/// библиотеку. Возвращает данные и число отброшенных записей.
pub fn parse_lenient(value: &Value) -> (LibraryData, usize) {
    let mut dropped = 0;
    let mut data = LibraryData { version: CURRENT_VERSION, sounds: vec![], boards: vec![] };
    let items = |key: &str, dropped: &mut usize| -> Vec<Map<String, Value>> {
        match value.get(key) {
            None | Some(Value::Null) => vec![],
            Some(Value::Array(list)) => list
                .iter()
                .filter_map(|v| match v.as_object() {
                    Some(o) => Some(o.clone()),
                    None => {
                        *dropped += 1;
                        None
                    }
                })
                .collect(),
            Some(_) => {
                *dropped += 1;
                vec![]
            }
        }
    };
    for mut s in items("sounds", &mut dropped) {
        fill_sound_defaults(&mut s);
        match serde_json::from_value::<Sound>(Value::Object(s)) {
            Ok(v) => data.sounds.push(v),
            Err(_) => dropped += 1,
        }
    }
    for mut b in items("boards", &mut dropped) {
        fill_board_defaults(&mut b);
        match serde_json::from_value::<Board>(Value::Object(b)) {
            Ok(v) => data.boards.push(v),
            Err(_) => dropped += 1,
        }
    }
    (data, dropped)
}
