//! Общая ошибка команд. Владелец: A (контракт). В UI приходит как `{ code, message }`.
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    /// Машинный код: `not_implemented`, `not_found`, `device_unavailable`,
    /// `cable_missing`, `io`, `decode`, `hotkey_conflict`, `invalid_argument`, `internal`.
    pub code: String,
    /// Человекочитаемое описание (для логов; UI показывает свой текст по `code`).
    pub message: String,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into() }
    }
    pub fn not_implemented(what: &str) -> Self {
        Self::new("not_implemented", format!("{what}: ещё не реализовано"))
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = Result<T, AppError>;
