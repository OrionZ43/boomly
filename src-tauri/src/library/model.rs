//! Чистая логика правок библиотеки (без файлов и Tauri) — удобно тестировать.
use std::collections::HashSet;
use std::path::Path;

use super::types::*;
use crate::error::{AppError, AppResult};

pub const DEFAULT_COLOR: &str = "#00f0ff";

/// Цвета новых плиток по кругу (акценты Z43, без жёлтого — он для «играет/избранное»).
pub const IMPORT_COLORS: &[&str] =
    &["#00f0ff", "#9d00ff", "#ff00c1", "#00ff9c", "#ff6b00", "#3d7bff", "#ff3860", "#b4ff00"];

/// Поддерживаемые форматы (расширение в нижнем регистре).
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "flac"];

pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn not_found(what: &str, id: &str) -> AppError {
    AppError::new("not_found", format!("{what} {id} не найден(а)"))
}

fn invalid(msg: impl Into<String>) -> AppError {
    AppError::new("invalid_argument", msg)
}

fn clean_emoji(e: Option<String>) -> Option<String> {
    e.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn clean_name(name: &str) -> AppResult<String> {
    let n = name.trim();
    if n.is_empty() {
        return Err(invalid("имя не может быть пустым"));
    }
    Ok(n.chars().take(100).collect())
}

pub fn board_mut<'a>(data: &'a mut LibraryData, id: &str) -> AppResult<&'a mut Board> {
    data.boards.iter_mut().find(|b| b.id == id).ok_or_else(|| not_found("доска", id))
}

/// Заменяет звук по id. `path`, `addedAt` и `durationMs` UI не меняет — берём сохранённые.
pub fn update_sound(data: &mut LibraryData, sound: Sound) -> AppResult<Sound> {
    if !sound.volume.is_finite() || !(0.0..=2.0).contains(&sound.volume) {
        return Err(invalid("громкость звука вне 0..2"));
    }
    let name = clean_name(&sound.name)?;
    let color = sound.color.trim().to_string();
    if color.is_empty() {
        return Err(invalid("пустой цвет"));
    }
    let cur = data.sounds.iter_mut().find(|s| s.id == sound.id).ok_or_else(|| not_found("звук", &sound.id))?;
    *cur = Sound {
        name,
        color,
        emoji: clean_emoji(sound.emoji),
        path: std::mem::take(&mut cur.path),
        duration_ms: cur.duration_ms,
        added_at: cur.added_at,
        ..sound
    };
    Ok(cur.clone())
}

/// Удаляет звуки отовсюду. Возвращает удалённые (чтобы вызывающий убрал файлы и хоткеи).
pub fn delete_sounds(data: &mut LibraryData, ids: &[String]) -> Vec<Sound> {
    let ids: HashSet<&str> = ids.iter().map(String::as_str).collect();
    let (removed, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut data.sounds).into_iter().partition(|s| ids.contains(s.id.as_str()));
    data.sounds = kept;
    for b in &mut data.boards {
        b.sound_ids.retain(|id| !ids.contains(id.as_str()));
    }
    removed
}

pub fn create_board(data: &mut LibraryData, name: &str, emoji: Option<String>) -> AppResult<Board> {
    let board = Board { id: new_id(), name: clean_name(name)?, emoji: clean_emoji(emoji), sound_ids: vec![] };
    data.boards.push(board.clone());
    Ok(board)
}

/// Заменяет доску. Неизвестные и повторные `soundIds` отбрасываются.
pub fn update_board(data: &mut LibraryData, board: Board) -> AppResult<Board> {
    let known: HashSet<&str> = data.sounds.iter().map(|s| s.id.as_str()).collect();
    let mut seen = HashSet::new();
    let sound_ids: Vec<String> =
        board.sound_ids.into_iter().filter(|id| known.contains(id.as_str()) && seen.insert(id.clone())).collect();
    let name = clean_name(&board.name)?;
    let cur = board_mut(data, &board.id)?;
    cur.name = name;
    cur.emoji = clean_emoji(board.emoji);
    cur.sound_ids = sound_ids;
    Ok(cur.clone())
}

pub fn delete_board(data: &mut LibraryData, id: &str) -> AppResult<()> {
    let before = data.boards.len();
    data.boards.retain(|b| b.id != id);
    if data.boards.len() == before {
        return Err(not_found("доска", id));
    }
    Ok(())
}

/// Новый порядок досок: сначала перечисленные (неизвестные id игнорируются),
/// затем не упомянутые — в прежнем порядке. Так гонка с созданием доски ничего не теряет.
pub fn reorder_boards(data: &mut LibraryData, ids: &[String]) {
    let mut rest = std::mem::take(&mut data.boards);
    let mut ordered = Vec::with_capacity(rest.len());
    for id in ids {
        if let Some(pos) = rest.iter().position(|b| &b.id == id) {
            ordered.push(rest.remove(pos));
        }
    }
    ordered.extend(rest);
    data.boards = ordered;
}

/// Убирает ссылки на несуществующие звуки (после ручной правки файла и т.п.).
pub fn sanitize(data: &mut LibraryData) {
    let known: HashSet<String> = data.sounds.iter().map(|s| s.id.clone()).collect();
    for b in &mut data.boards {
        let mut seen = HashSet::new();
        b.sound_ids.retain(|id| known.contains(id) && seen.insert(id.clone()));
    }
    for s in &mut data.sounds {
        if !s.volume.is_finite() {
            s.volume = 1.0;
        }
        s.volume = s.volume.clamp(0.0, 2.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn sound(id: &str) -> Sound {
        Sound {
            id: id.into(),
            name: id.into(),
            path: format!("C:/s/{id}.mp3"),
            color: DEFAULT_COLOR.into(),
            emoji: None,
            volume: 1.0,
            mode: PlayMode::Toggle,
            favorite: false,
            duration_ms: Some(1000),
            added_at: 5,
        }
    }

    fn lib() -> LibraryData {
        let mut d = LibraryData { sounds: vec![sound("a"), sound("b"), sound("c")], ..Default::default() };
        let b = create_board(&mut d, "Мемы", Some("😂".into())).unwrap();
        update_board(&mut d, Board { sound_ids: vec!["a".into(), "b".into()], ..b }).unwrap();
        d
    }

    #[test]
    fn update_sound_keeps_path_and_validates() {
        let mut d = lib();
        let mut s = sound("a");
        s.path = "C:/evil.exe".into();
        s.name = "  Привет  ".into();
        s.volume = 1.5;
        s.favorite = true;
        s.added_at = 999;
        let out = update_sound(&mut d, s.clone()).unwrap();
        assert_eq!(out.path, "C:/s/a.mp3");
        assert_eq!(out.added_at, 5);
        assert_eq!(out.name, "Привет");
        assert!(out.favorite);
        s.volume = 2.5;
        assert_eq!(update_sound(&mut d, s.clone()).unwrap_err().code, "invalid_argument");
        s.volume = 1.0;
        s.id = "zzz".into();
        assert_eq!(update_sound(&mut d, s).unwrap_err().code, "not_found");
    }

    #[test]
    fn delete_sounds_removes_from_boards() {
        let mut d = lib();
        let removed = delete_sounds(&mut d, &["a".into(), "nope".into()]);
        assert_eq!(removed.len(), 1);
        assert_eq!(d.sounds.len(), 2);
        assert_eq!(d.boards[0].sound_ids, vec!["b".to_string()]);
    }

    #[test]
    fn update_board_filters_unknown_and_duplicates() {
        let mut d = lib();
        let mut b = d.boards[0].clone();
        b.sound_ids = vec!["c".into(), "x".into(), "a".into(), "c".into()];
        let out = update_board(&mut d, b).unwrap();
        assert_eq!(out.sound_ids, vec!["c".to_string(), "a".to_string()]);
        assert_eq!(create_board(&mut d, "   ", None).unwrap_err().code, "invalid_argument");
    }

    #[test]
    fn reorder_boards_is_lossless() {
        let mut d = lib();
        let b2 = create_board(&mut d, "2", None).unwrap();
        let b3 = create_board(&mut d, "3", None).unwrap();
        let b1 = d.boards[0].id.clone();
        reorder_boards(&mut d, &[b3.id.clone(), "ghost".into(), b1.clone()]);
        let ids: Vec<_> = d.boards.iter().map(|b| b.id.clone()).collect();
        assert_eq!(ids, vec![b3.id, b1, b2.id]);
    }

    #[test]
    fn delete_board_keeps_sounds() {
        let mut d = lib();
        let id = d.boards[0].id.clone();
        delete_board(&mut d, &id).unwrap();
        assert!(d.boards.is_empty());
        assert_eq!(d.sounds.len(), 3);
        assert_eq!(delete_board(&mut d, &id).unwrap_err().code, "not_found");
    }

    #[test]
    fn audio_extensions() {
        assert!(is_audio_file(Path::new("x/Y.MP3")));
        assert!(is_audio_file(Path::new("a.flac")));
        assert!(!is_audio_file(Path::new("a.txt")));
        assert!(!is_audio_file(Path::new("mp3")));
    }
}
