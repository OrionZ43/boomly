//! Миграции схемы `library.json`. Каждая функция поднимает JSON ровно на одну версию.
//! Работаем на уровне `serde_json::Value`, чтобы старые файлы читались, даже если в них
//! нет полей, которые теперь обязательны в Rust-типах.
use serde_json::{Map, Value, json};

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
            b.entry("name").or_insert_with(|| json!("Доска"));
            b.entry("emoji").or_insert(Value::Null);
            b.entry("soundIds").or_insert_with(|| json!([]));
        }
    }
    Ok(value)
}

pub fn fill_sound_defaults(s: &mut Map<String, Value>) {
    if !s.contains_key("name") {
        let stem = s
            .get("path")
            .and_then(Value::as_str)
            .and_then(|p| std::path::Path::new(p).file_stem().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "Звук".into());
        s.insert("name".into(), json!(stem));
    }
    s.entry("color").or_insert_with(|| json!(super::model::DEFAULT_COLOR));
    s.entry("emoji").or_insert(Value::Null);
    s.entry("volume").or_insert_with(|| json!(1.0));
    s.entry("mode").or_insert_with(|| json!("toggle"));
    s.entry("favorite").or_insert_with(|| json!(false));
    s.entry("durationMs").or_insert(Value::Null);
    s.entry("addedAt").or_insert_with(|| json!(0));
}
