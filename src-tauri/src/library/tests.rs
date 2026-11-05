//! Тесты хранилища библиотеки: миграции схемы, атомарность, импорт.
use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::*;

fn req(paths: &[&Path], board: Option<&str>) -> ImportRequest {
    ImportRequest {
        paths: paths.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        board_id: board.map(Into::into),
        copy_files: None,
    }
}

fn read(p: &Path) -> Value {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}

fn opened(dir: &Path) -> Library {
    let l = Library::new();
    l.open(dir).unwrap();
    l
}

// ---------- миграции ----------

#[test]
fn first_run_is_empty_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let l = opened(dir.path());
    assert_eq!(l.snapshot(), LibraryData::default());
    assert!(!dir.path().join(LIBRARY_FILE).exists());
    assert!(dir.path().join(SOUNDS_DIR).is_dir());
}

#[test]
fn migrates_v0_without_version_and_keeps_original() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(LIBRARY_FILE);
    let v0 = json!({
        "sounds": [
            {"id": "s1", "path": "C:/x/Bruh.mp3"},
            {"id": "s2", "path": "C:/x/b.wav", "name": "B", "volume": 0.5, "favorite": true},
            {"name": "битый, без id"}
        ],
        "boards": [{"id": "b1", "soundIds": ["s1", "ghost", "s1"]}]
    });
    fs::write(&file, serde_json::to_vec(&v0).unwrap()).unwrap();

    let l = opened(dir.path());
    let d = l.snapshot();
    assert_eq!(d.version, migrate::CURRENT_VERSION);
    assert_eq!(d.sounds.len(), 2);
    assert_eq!(d.sounds[0].name, "Bruh");
    assert_eq!(d.sounds[0].volume, 1.0);
    assert_eq!(d.sounds[0].mode, PlayMode::Toggle);
    assert_eq!(d.sounds[1].volume, 0.5);
    assert!(d.sounds[1].favorite);
    assert_eq!(d.boards[0].sound_ids, vec!["s1".to_string()]);
    assert_eq!(d.boards[0].name, "Доска");
    // Файл переписан в новой схеме, оригинал сохранён.
    assert_eq!(read(&file)["version"], json!(1));
    assert_eq!(read(&dir.path().join("library.json.v0.bak")), v0);
}

#[test]
fn current_version_is_read_as_is() {
    let dir = tempfile::tempdir().unwrap();
    let l = opened(dir.path());
    l.mutate(|d| model::create_board(d, "Мемы", None)).unwrap();
    let before = fs::read(dir.path().join(LIBRARY_FILE)).unwrap();
    let l2 = opened(dir.path());
    assert_eq!(l2.snapshot(), l.snapshot());
    assert_eq!(fs::read(dir.path().join(LIBRARY_FILE)).unwrap(), before, "без миграции файл не трогаем");
}

#[test]
fn newer_schema_keeps_a_copy_of_the_original() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(LIBRARY_FILE);
    let v9 = json!({"version": 9, "sounds": [], "boards": [{"id": "b", "name": "X", "emoji": null, "soundIds": [], "color": "new-field"}]});
    fs::write(&file, serde_json::to_vec(&v9).unwrap()).unwrap();
    let l = opened(dir.path());
    assert_eq!(l.snapshot().boards[0].name, "X");
    assert_eq!(read(&dir.path().join("library.json.v9.bak")), v9);
}

#[test]
fn unreadable_structure_is_kept_aside() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(LIBRARY_FILE);
    fs::write(&file, br#"{"version": 1, "sounds": "oops", "boards": []}"#).unwrap();
    let l = opened(dir.path());
    assert_eq!(l.snapshot(), LibraryData::default());
    let kept = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .any(|e| e.file_name().to_string_lossy().starts_with("library.json.unreadable-"));
    assert!(kept);
}

#[test]
fn not_an_object_is_an_error_not_a_wipe() {
    assert_eq!(migrate::migrate(json!([1, 2])).unwrap_err().code, "decode");
}

// ---------- атомарность ----------

#[test]
fn every_change_is_on_disk_and_previous_in_bak() {
    let dir = tempfile::tempdir().unwrap();
    let l = opened(dir.path());
    let (b, _) = l.mutate(|d| model::create_board(d, "Один", None)).unwrap();
    l.mutate(|d| model::create_board(d, "Два", None)).unwrap();
    let main = read(&dir.path().join(LIBRARY_FILE));
    assert_eq!(main["boards"].as_array().unwrap().len(), 2);
    let bak = read(&dir.path().join("library.json.bak"));
    assert_eq!(bak["boards"][0]["id"], json!(b.id));
    assert_eq!(bak["boards"].as_array().unwrap().len(), 1);
}

#[test]
fn failed_change_leaves_memory_and_disk_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let l = opened(dir.path());
    l.mutate(|d| model::create_board(d, "Один", None)).unwrap();
    let disk = fs::read(dir.path().join(LIBRARY_FILE)).unwrap();
    let mem = l.snapshot();
    let err = l
        .mutate(|d| {
            d.boards.clear(); // частичная правка до ошибки
            model::delete_board(d, "нет-такой")
        })
        .unwrap_err();
    assert_eq!(err.code, "not_found");
    assert_eq!(l.snapshot(), mem);
    assert_eq!(fs::read(dir.path().join(LIBRARY_FILE)).unwrap(), disk);
}

#[test]
fn write_failure_does_not_change_memory() {
    let dir = tempfile::tempdir().unwrap();
    let l = opened(dir.path());
    // Подменяем путь файла на папку — запись поверх неё невозможна.
    let blocker = dir.path().join("blocker");
    fs::create_dir_all(blocker.join("x")).unwrap();
    l.lock().file = Some(blocker);
    assert!(l.mutate(|d| model::create_board(d, "X", None)).is_err());
    assert!(l.snapshot().boards.is_empty());
}

#[test]
fn corrupt_main_recovers_from_bak_on_open() {
    let dir = tempfile::tempdir().unwrap();
    let l = opened(dir.path());
    l.mutate(|d| model::create_board(d, "Один", None)).unwrap();
    l.mutate(|d| model::create_board(d, "Два", None)).unwrap();
    fs::write(dir.path().join(LIBRARY_FILE), b"{\"version\":1,\"sou").unwrap();
    let l2 = opened(dir.path());
    assert_eq!(l2.snapshot().boards.len(), 1, "взята предыдущая версия из .bak");
    // И восстановленное сразу записано как основной файл.
    assert_eq!(read(&dir.path().join(LIBRARY_FILE))["boards"].as_array().unwrap().len(), 1);
}

// ---------- импорт ----------

fn fixture(dir: &Path) -> std::path::PathBuf {
    let src = dir.join("src");
    fs::create_dir_all(src.join("sub/deeper")).unwrap();
    fs::write(src.join("a.mp3"), b"mp3-a").unwrap();
    fs::write(src.join("B.WAV"), b"wav-b").unwrap();
    fs::write(src.join("cover.jpg"), b"jpg").unwrap();
    fs::write(src.join("sub/c.ogg"), b"ogg-c").unwrap();
    fs::write(src.join("sub/deeper/d.flac"), b"flac-d").unwrap();
    fs::write(src.join("sub/readme.txt"), b"txt").unwrap();
    src
}

#[test]
fn import_folder_recursively_with_copy() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let src = fixture(dir.path());
    let l = opened(&data);
    let (res, snap) = l.import(&req(&[&src], None), true).unwrap();
    let names: Vec<_> = res.added.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["a", "B", "c", "d"]);
    assert!(res.skipped.is_empty(), "посторонние файлы в папке не считаются ошибкой");
    assert_eq!(snap.sounds.len(), 4);
    for s in &res.added {
        let p = Path::new(&s.path);
        assert!(p.starts_with(data.join(SOUNDS_DIR)), "{} скопирован в папку приложения", s.path);
        assert!(p.exists());
        assert_eq!(s.volume, 1.0);
        assert!(uuid::Uuid::parse_str(&s.id).is_ok());
    }
    assert_eq!(fs::read(&res.added[1].path).unwrap(), b"wav-b");
    assert!(res.added[1].path.ends_with(".wav"), "расширение в нижнем регистре");
    // Разные цвета у соседних плиток.
    assert_ne!(res.added[0].color, res.added[1].color);
    // Записано на диск.
    assert_eq!(read(&data.join(LIBRARY_FILE))["sounds"].as_array().unwrap().len(), 4);
}

#[test]
fn import_without_copy_references_originals() {
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let l = opened(&dir.path().join("data"));
    let (res, _) = l.import(&req(&[&src.join("a.mp3")], None), false).unwrap();
    assert_eq!(res.added.len(), 1);
    assert_eq!(Path::new(&res.added[0].path).canonicalize().unwrap(), src.join("a.mp3").canonicalize().unwrap());
}

#[test]
fn import_skips_bad_paths_and_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let l = opened(&dir.path().join("data"));
    let missing = src.join("nope.mp3");
    let jpg = src.join("cover.jpg");
    let (res, _) = l.import(&req(&[&src.join("a.mp3"), &src.join("a.mp3"), &missing, &jpg], None), true).unwrap();
    assert_eq!(res.added.len(), 1, "один и тот же файл в запросе — один звук");
    assert_eq!(res.skipped.len(), 2);
}

#[test]
fn import_into_board() {
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let l = opened(&dir.path().join("data"));
    let (b, _) = l.mutate(|d| model::create_board(d, "Мемы", None)).unwrap();
    let (res, snap) = l.import(&req(&[&src.join("sub")], Some(&b.id)), true).unwrap();
    let ids: Vec<_> = res.added.iter().map(|s| s.id.clone()).collect();
    assert_eq!(snap.boards[0].sound_ids, ids);
    let err = l.import(&req(&[&src], Some("ghost")), true).unwrap_err();
    assert_eq!(err.code, "not_found");
    assert_eq!(l.snapshot().sounds.len(), 2, "при ошибке ничего не добавлено");
}

#[test]
fn delete_removes_copied_file_but_never_originals() {
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let l = opened(&dir.path().join("data"));
    let (copied, _) = l.import(&req(&[&src.join("a.mp3")], None), true).unwrap();
    let (linked, _) = l.import(&req(&[&src.join("B.WAV")], None), false).unwrap();
    let ids = vec![copied.added[0].id.clone(), linked.added[0].id.clone()];
    let (removed, snap) = l.delete_sounds(&ids).unwrap();
    assert_eq!(removed.len(), 2);
    assert!(snap.sounds.is_empty());
    assert!(!Path::new(&copied.added[0].path).exists(), "копия в папке приложения удалена");
    assert!(src.join("B.WAV").exists(), "оригинал пользователя не тронут");
    assert!(src.join("a.mp3").exists());
}

#[test]
fn data_survives_reopen_like_app_update() {
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let data = dir.path().join("data");
    let l = opened(&data);
    let (b, _) = l.mutate(|d| model::create_board(d, "Мемы", Some("😂".into()))).unwrap();
    l.import(&req(&[&src], Some(&b.id)), true).unwrap();
    let mut s = l.snapshot().sounds[0].clone();
    s.favorite = true;
    s.mode = PlayMode::Overlay;
    l.mutate(|d| model::update_sound(d, s)).unwrap();
    let before = l.snapshot();
    drop(l);
    assert_eq!(opened(&data).snapshot(), before);
}

// ---------- найдено ревью ----------

#[test]
fn one_broken_record_does_not_wipe_the_library() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(LIBRARY_FILE);
    let v1 = json!({"version": 1, "sounds": [
        {"id": "ok", "name": "OK", "path": "C:/a.mp3", "color": "#fff", "emoji": null, "volume": 1.0,
         "mode": "toggle", "favorite": false, "durationMs": null, "addedAt": 1},
        {"id": "nulls", "path": "C:/b.mp3", "volume": null, "favorite": null},
        {"id": "bad", "path": "C:/c.mp3", "volume": "громко"},
        42
    ], "boards": [{"id": "b", "soundIds": ["ok", "nulls", "bad"]}, "мусор"]});
    fs::write(&file, serde_json::to_vec(&v1).unwrap()).unwrap();
    let d = opened(dir.path()).snapshot();
    let ids: Vec<_> = d.sounds.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, vec!["ok", "nulls"], "битая запись пропущена, остальные целы");
    assert_eq!(d.sounds[1].volume, 1.0);
    assert!(d.sounds[1].added_at > 0, "дата добавления — не 1970");
    assert_eq!(d.boards.len(), 1);
    assert_eq!(d.boards[0].sound_ids, vec!["ok".to_string(), "nulls".to_string()]);
    let kept = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().starts_with("library.json.unreadable-"))
        .expect("оригинал с битыми записями сохранён");
    assert_eq!(read(&kept.path()), v1);
}

#[test]
fn unavailable_store_rejects_changes_instead_of_losing_them() {
    let dir = tempfile::tempdir().unwrap();
    // Папка данных — на самом деле файл: открыть хранилище нельзя.
    let data = dir.path().join("data");
    fs::write(&data, b"not a dir").unwrap();
    let l = Library::new();
    assert_eq!(l.open(&data).unwrap_err().code, "io");
    assert_eq!(l.mutate(|d| model::create_board(d, "X", None)).unwrap_err().code, "io");
    let src = fixture(dir.path());
    assert_eq!(l.import(&req(&[&src], None), false).unwrap_err().code, "io");
    assert!(l.snapshot().boards.is_empty());
}

#[test]
fn shared_copied_file_is_not_deleted_while_referenced() {
    let dir = tempfile::tempdir().unwrap();
    let src = fixture(dir.path());
    let data = dir.path().join("data");
    let l = opened(&data);
    let (first, _) = l.import(&req(&[&src.join("a.mp3")], None), true).unwrap();
    let copy = Path::new(&first.added[0].path).to_path_buf();
    // Пользователь импортировал ту же копию прямо из папки приложения, без копирования.
    let (second, _) = l.import(&req(&[&copy], None), false).unwrap();
    l.delete_sounds(&[second.added[0].id.clone()]).unwrap();
    assert!(copy.exists(), "файл нужен первому звуку");
    l.delete_sounds(&[first.added[0].id.clone()]).unwrap();
    assert!(!copy.exists());
}

#[test]
fn newer_schema_copy_is_refreshed_each_time() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(LIBRARY_FILE);
    let keep = dir.path().join("library.json.v9.bak");
    fs::write(&file, br#"{"version": 9, "sounds": [], "boards": [], "x": 1}"#).unwrap();
    opened(dir.path());
    fs::write(&file, br#"{"version": 9, "sounds": [], "boards": [], "x": 2}"#).unwrap();
    opened(dir.path());
    assert_eq!(read(&keep)["x"], json!(2));
}

/// Минимальный WAV: PCM 16 бит, моно, 48 кГц.
fn write_wav(path: &Path, samples: u32) {
    let data_len = samples * 2;
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&48_000u32.to_le_bytes());
    b.extend_from_slice(&96_000u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    b.resize(b.len() + data_len as usize, 0);
    fs::write(path, b).unwrap();
}

#[test]
fn import_fills_duration_for_real_audio() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("half.wav");
    write_wav(&src, 24_000); // 0.5 с
    let l = opened(&dir.path().join("data"));
    let (res, _) = l.import(&req(&[&src], None), true).unwrap();
    let ms = res.added[0].duration_ms.expect("длительность должна определиться");
    assert!((490..=510).contains(&ms), "{ms}");
}
