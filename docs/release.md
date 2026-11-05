# Релизы и автообновление Boomly

Как устроено: тег `vX.Y.Z` → GitHub Actions (`.github/workflows/release.yml`) собирает
NSIS-установщик, подписывает его ключом апдейтера и публикует GitHub Release с тремя файлами:
`Boomly_X.Y.Z_x64-setup.exe`, `…setup.exe.sig` и `latest.json`. Установленные копии Boomly
при старте (и раз в 6 часов) скачивают
`https://github.com/OrionZ43/boomly/releases/latest/download/latest.json`, проверяют подпись
публичным ключом, зашитым в приложение, и предлагают обновиться. Схема та же, что у Protogenix:
статичный файл, без GitHub API и токенов.

## Один раз: ключ подписи

> ⚠ Ключ — самое ценное в этой схеме. **Потеряете приватный ключ — ни одна установленная копия
> больше не примет обновление**, пользователям придётся вручную скачивать установщик.
> **Утечёт ключ** — кто-то сможет подписать свой «апдейт». Ключ не попадает в репозиторий,
> чат, логи и агентам.

### 1. Сгенерировать ключ (на своём компьютере)

В PowerShell, в папке проекта:

```powershell
mkdir "$env:USERPROFILE\.boomly" -Force
pnpm tauri signer generate -w "$env:USERPROFILE\.boomly\boomly-updater.key"
```

Программа спросит пароль — придумайте надёжный и сохраните в менеджере паролей.
Появятся два файла:

- `boomly-updater.key` — **приватный ключ** (секрет);
- `boomly-updater.key.pub` — публичный ключ (не секрет, он будет внутри приложения).

**Сделайте две копии приватного ключа и пароля вне репозитория**, например:
1. `%USERPROFILE%\.boomly\boomly-updater.key` (основная);
2. флешка / зашифрованный архив в облаке / менеджер паролей (резервная).

Папка `.boomly` в профиле — вне проекта, `*.key` и `*.key.pub` к тому же в `.gitignore`.

### 2. Вписать публичный ключ в приложение

Откройте `boomly-updater.key.pub` (одна длинная строка) и вставьте её содержимое в
`src-tauri/tauri.conf.json`:

```json
"plugins": {
  "updater": {
    "pubkey": "ВСТАВЬТЕ_СЮДА_СОДЕРЖИМОЕ_boomly-updater.key.pub",
```

Закоммитьте это изменение (`chore: updater public key`). Пока `pubkey` пустой, приложение
обновлений не ищет, а `release.yml` откажется публиковать релиз.

### 3. Добавить секреты в GitHub Actions

Через `gh` (в PowerShell, из папки проекта):

```powershell
Get-Content "$env:USERPROFILE\.boomly\boomly-updater.key" -Raw | gh secret set TAURI_SIGNING_PRIVATE_KEY
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD   # вставьте пароль, Enter
```

Или в браузере: репозиторий → Settings → Secrets and variables → Actions → New repository
secret. Имена ровно такие:

| Секрет | Значение |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | всё содержимое файла `boomly-updater.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | пароль от ключа |

Проверка: `gh secret list` показывает оба имени (значения GitHub не показывает никому).

## Каждый релиз

1. **Поднять версию** в трёх местах одинаково (semver, должна расти):
   `src-tauri/tauri.conf.json` → `version`, `src-tauri/Cargo.toml` → `version`,
   `package.json` → `version`. Обновить `Cargo.lock`: `cd src-tauri; cargo check`.
2. **Описать изменения**: скопировать `docs/release-notes/TEMPLATE.md` в
   `docs/release-notes/vX.Y.Z.md`. Этот текст увидят в релизе и в баннере обновления.
3. Закоммитить в `main` (через PR, CI должен быть зелёным).
4. **Поставить тег и отправить**:
   ```powershell
   git checkout main; git pull
   git tag v0.1.0
   git push origin v0.1.0
   ```
5. Смотреть **Actions → Release**. Workflow проверит, что тег = версии, `pubkey` вписан,
   секрет есть и есть файл заметок; соберёт, опубликует релиз и в конце убедится, что
   `releases/latest/download/latest.json` отдаёт новую версию.
6. Проверить обновление: на машине со старой версией перезапустить Boomly — через ~5 секунд
   появится баннер «Доступна версия X.Y.Z». «Обновить» → прогресс → установщик сам закроет
   и снова запустит приложение.

Если что-то упало до публикации — исправить, удалить тег (`git tag -d v0.1.0;
git push origin :refs/tags/v0.1.0`) и поставить заново. Опубликованный релиз не правьте
задним числом — выпустите следующую версию.

### Сухой прогон

Actions → Release → **Run workflow** (ветка любая): то же самое, но без подписи и без
публикации. Установщик приложен артефактом `boomly-setup-dry-run`. Ключ не нужен.

### Собрать установщик локально

```powershell
pnpm install
pnpm tauri build --bundles nsis --config .github/tauri.nosign.json
```

Результат: `src-tauri\target\release\bundle\nsis\Boomly_X.Y.Z_x64-setup.exe`. Без
`--config …` сборка попросит ключ подписи (`TAURI_SIGNING_PRIVATE_KEY`).

## Установщик

- NSIS, ставится **для текущего пользователя** (`%LOCALAPPDATA%\Boomly`), прав администратора
  не просит. Языки — русский и английский (выбирается по языку Windows).
- `identifier` = `com.orionz43.boomly` — **не менять никогда**: по нему новая версия
  находит и обновляет старую.
- Данные пользователя — `%APPDATA%\Boomly` (звуки, `library.json`, настройки). Обновление их не
  трогает. При удалении программы они остаются, если не поставить галочку «Удалить данные
  приложения» (хук `src-tauri/installer/hooks.nsh`).
- Если на машине нет WebView2, установщик скачает его сам (нужен интернет).
- Обновление ставится в режиме `passive`: видно только окно прогресса, вопросов нет.

## Смена ключа (если понадобится)

У Tauri в приложении один публичный ключ. Порядок: выпустить версию N, подписанную **старым**
ключом, но с **новым** `pubkey` в конфиге; все, кто обновился до N, дальше принимают только
новый ключ. Затем заменить секреты в Actions на новый ключ. Утёк ключ — сделать это как можно
скорее; потерян — смена невозможна, только ручная переустановка.

## Что проверяет CI

`ci.yml` на каждый PR и push в `main` (Windows): `pnpm check`, `pnpm build`,
`cargo clippy -D warnings`, `cargo test`, затем пробная сборка NSIS без подписи (артефакт
`boomly-setup-unsigned`, хранится 7 дней). Секреты для этого не нужны.
