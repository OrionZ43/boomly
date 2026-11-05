# Контракты Rust ↔ UI и раскладка по участкам

Владелец документа — A (ведущий инженер). Любое изменение контракта (имя команды, аргументы,
форма JSON, событие) — **одним коммитом** в трёх местах: этот файл, Rust-тип в
`src-tauri/src/<участок>/types.rs`, TS-тип в `src/lib/api/types.ts` (+ `commands.ts` /
`events.ts` / `mock.ts`). Если изменение нужно чужому участку — сначала письмо A.

## 1. Кто где работает

| Участок | Папки и файлы (только свои) | Общее, что можно дополнять своим блоком |
|---|---|---|
| **B** — аудиодвижок, сквозной микрофон | `src-tauri/src/audio/**` | зависимости в `src-tauri/Cargo.toml` (свой блок `# B: audio`) |
| **B2** — хоткеи, трей, автозапуск, single-instance, библиотека | `src-tauri/src/library/**`, `src-tauri/src/hotkeys/**`, `src-tauri/src/system/**` | `Cargo.toml` (`# B2`), `src-tauri/capabilities/default.json` (свои permissions) |
| **C** — дизайн-система | `src/lib/ui/**`, `src/app.css`, `src/routes/ui/**` (витрина), `static/fonts/**` | `package.json` (зависимости иконок/шрифтов) |
| **D** — экраны | `src/routes/**` кроме `ui/`, `src/lib/features/**`, `src/lib/i18n/**`, `src/lib/stores/**` | — |
| **E** — установщик, апдейтер, CI | `src-tauri/src/updater/**`, `src/lib/updater/**`, `src/lib/api/updater.ts`, `.github/**`, `docs/release*.md`, `src-tauri/icons/**` | `tauri.conf.json` — раздел `bundle` и `plugins.updater`; `Cargo.toml` (`# E`) |
| **A** — каркас, контракты | `src-tauri/src/lib.rs`, `error.rs`, `events.rs`, `paths.rs`, `*/types.rs`, `src/lib/api/**` (кроме `updater.ts`), `src/lib/config/**`, `src/routes/+layout.svelte`, `docs/contracts.md`, `docs/decisions.md` | — |

Правила стыковки:
- Новую команду участок пишет в своём `commands.rs` и добавляет **одну строку** в своём блоке
  `generate_handler!` в `lib.rs` (это единственная правка чужого файла без согласования).
- Плагины и `setup` участок регистрирует в своём `init()` (`system::init`, `hotkeys::init`,
  `updater::init`), а не в `lib.rs`. `tauri-plugin-single-instance` — первым в `system::init`.
- Экраны (D) зовут бэкенд **только** через `$lib/api` (`api.*`, `on(...)`), не через `invoke`.
- `Cargo.lock` и `pnpm-lock.yaml` при конфликте не правим руками — перегенерируем.
- `tauri.conf.json`: окно (`app.windows`) — A/D, `bundle`/`plugins.updater` — E.

## 2. Соглашения

- JSON — **camelCase**; Rust-типы с `#[serde(rename_all = "camelCase")]`. Аргументы команд в
  `invoke` — camelCase (Tauri переводит в snake_case параметров).
- Ошибка любой команды: `{ code, message }` (`AppError`, `src-tauri/src/error.rs`). Коды:
  `not_implemented`, `not_found`, `device_unavailable`, `cable_missing`, `io`, `decode`,
  `hotkey_conflict`, `invalid_argument`, `internal`. UI показывает текст по `code`.
- Громкости каналов — `0.0..=1.0`; громкость отдельного звука — `0.0..=2.0`.
- id звуков и досок — UUID v4 (строка), генерирует Rust.
- Время — миллисекунды (`…Ms`), даты — Unix ms.
- Данные — `%APPDATA%\Boomly` (`crate::paths::data_dir`), не `app_data_dir()`.

## 3. Команды

Сигнатуры TS — `src/lib/api/commands.ts`, обёртки — `api.*` в `src/lib/api/index.ts`.

### Аудио (B) — `src-tauri/src/audio/commands.rs`

| Команда | Аргументы | Результат | Заметки |
|---|---|---|---|
| `audio_list_devices` | — | `DeviceList` | выходы и входы, `isCable` помечает VB-CABLE |
| `audio_get_settings` | — | `AudioSettings` | |
| `audio_set_devices` | `selection: DeviceSelection` | `AudioSettings` | поле нет — не менять, `null` — по умолчанию/авто |
| `audio_set_volume` | `channel: "monitor"\|"cable"\|"mic"`, `value` | `AudioSettings` | «Я слышу» / «В микрофон» / «Голос» |
| `audio_set_mic_muted` | `muted` | `AudioSettings` | mute голоса в кабеле |
| `audio_set_mic_passthrough` | `enabled` | `AudioSettings` | вкл/выкл сквозной микрофон |
| `audio_cable_status` | — | `CableStatus` | есть ли VB-CABLE, ссылка на скачивание |
| `audio_play` | `soundId` | `null` | Rust сам берёт путь/громкость/режим из библиотеки |
| `audio_stop` | `soundId` | `null` | |
| `audio_stop_all` | — | `null` | |
| `audio_play_test` | `target: "monitor"\|"cable"` | `null` | тестовый звук для мастера |

Повторный `audio_play` по играющему звуку зависит от `Sound.mode`: `toggle` — стоп,
`restart` — сначала, `overlay` — ещё одна копия.

### Библиотека (B2) — `src-tauri/src/library/commands.rs`

| Команда | Аргументы | Результат |
|---|---|---|
| `library_get` | — | `LibraryData` |
| `library_import` | `request: ImportRequest` (`paths`, `boardId?`, `copyFiles?`) | `ImportResult` (`added`, `skipped`) |
| `library_update_sound` | `sound: Sound` (целиком) | `Sound` |
| `library_delete_sounds` | `ids: string[]` | `null` |
| `library_create_board` | `name`, `emoji: string\|null` | `Board` |
| `library_update_board` | `board: Board` (целиком, вкл. порядок `soundIds`) | `Board` |
| `library_delete_board` | `id` | `null` |
| `library_reorder_boards` | `ids: string[]` | `null` |

«Избранное» — не доска, а флаг `Sound.favorite`; UI показывает его как виртуальную доску.
Каждая изменяющая команда шлёт `library://changed`.

### Хоткеи (B2) — `src-tauri/src/hotkeys/commands.rs`

| Команда | Аргументы | Результат | Заметки |
|---|---|---|---|
| `hotkeys_get` | — | `HotkeyBinding[]` | |
| `hotkeys_set` | `binding: HotkeyBinding` | `HotkeyBinding[]` | заменяет привязку того же действия; занято — `hotkey_conflict` |
| `hotkeys_remove` | `accelerator` | `HotkeyBinding[]` | |
| `hotkeys_set_capture` | `active` | `null` | снять хоткеи, пока UI записывает сочетание |

`HotkeyAction`: `{type:"playSound", soundId}` \| `{type:"stopAll"}` \| `{type:"toggleMicMute"}`.
Действие выполняет Rust (через `AudioEngine`) даже при свёрнутом окне, затем шлёт `hotkeys://triggered`.

### Приложение (B2) — `src-tauri/src/system/commands.rs`

| Команда | Аргументы | Результат |
|---|---|---|
| `app_get_settings` | — | `AppSettings` |
| `app_set_settings` | `settings: AppSettings` (целиком) | `AppSettings` (применяет автозапуск и т.п.) |
| `app_info` | — | `AppInfo` (`version`, `dataDir`) |
| `app_open_data_dir` | — | `null` |

### Апдейтер (E) — `src/lib/api/updater.ts`

UI зовёт `checkForUpdate(): Promise<UpdateInfo | null>` и `installUpdate(onProgress)`.
Реализация на `@tauri-apps/plugin-updater` — дело E; сигнатуры не менять без A.
Настройка `AppSettings.checkUpdates` включает автопроверку (при старте и раз в N часов).

## 4. События Rust → UI

Имена — `src-tauri/src/events.rs` и `src/lib/api/events.ts`. Подписка: `on("audio://playback", fn)`.

| Событие | Payload | Кто шлёт | Когда |
|---|---|---|---|
| `audio://playback` | `PlaybackEvent` (`soundId`, `state`: started/stopped/finished/error, `durationMs`) | B | старт/стоп/конец звука |
| `audio://progress` | `ProgressEvent[]` | B | пока что-то играет, ≤ 15 раз/с |
| `audio://devices-changed` | `DeviceList` | B | подключили/отключили устройство |
| `audio://cable-status` | `CableStatus` | B | VB-CABLE появился/пропал |
| `audio://mic-level` | `MicLevel` (`peak` 0..1) | B | ≤ 15 раз/с, пока включён сквозной микрофон |
| `audio://settings-changed` | `AudioSettings` | B | любая смена (в т.ч. mute с хоткея/трея) |
| `audio://error` | `AppError` | B | фоновая ошибка (устройство пропало) |
| `library://changed` | `LibraryData` | B2 | после любой правки библиотеки |
| `hotkeys://triggered` | `HotkeyTriggered` | B2 | сработал глобальный хоткей |
| `app://settings-changed` | `AppSettings` | B2 | настройки изменились (в т.ч. из трея) |

## 5. Внутренние Rust-API между участками

- `audio::AudioEngine` (B) — `play(&Sound)`, `stop(id)`, `stop_all()`, `set_mic_muted(bool)`,
  `settings()` и др. Хоткеи и трей (B2) зовут их через `app.state::<AudioEngine>()`.
- `library::Library` (B2) — `sound(id) -> Option<Sound>`, `snapshot() -> LibraryData`.
  `audio_play` берёт звук отсюда.
- Сигнатуры этих публичных методов — часть контракта. Внутренности — на усмотрение участка.

## 6. Моки для D

`src/lib/api/mock.ts` — бэкенд в памяти с демо-досками и имитацией воспроизведения
(события `playback`/`progress`). Включается сам в браузере (`pnpm dev`, http://localhost:1420)
и в окне Tauri при `VITE_BOOMLY_MOCK=1`. Rust-заглушки возвращают пустые данные или
`not_implemented`, так что в окне без мока экраны должны переносить пустую библиотеку.
