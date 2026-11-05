# Boomly

Саундпад для Windows: играет звуки в наушники и в микрофон (через VB-CABLE) одновременно,
подмешивает ваш голос, глобальные хоткеи, цветные плитки и доски.

> Статус: ранняя разработка (MVP). Сборок пока нет.

## Возможности MVP

- звуки в «себе» и «в микрофон» с раздельной громкостью;
- сквозной микрофон: голос + звуки в одном виртуальном устройстве, громкость голоса и mute;
- доски, избранное, drag & drop файлов, цвет и эмодзи плитки;
- глобальные хоткеи, трей, автозапуск;
- мастер первого запуска с проверкой VB-CABLE;
- автообновление из GitHub Releases.

Для вывода в Discord/игры нужен бесплатный [VB-CABLE](https://vb-audio.com/Cable/) — Boomly его не бандлит.

## Разработка

Нужны: Windows 10/11, Node 22+, pnpm, Rust (MSVC toolchain), Visual Studio Build Tools (C++), WebView2.

```powershell
pnpm install
pnpm tauri dev          # окно приложения
pnpm dev                # только UI в браузере, на моках (http://localhost:1420)
pnpm check              # svelte-check
cd src-tauri; cargo clippy --all-targets -- -D warnings; cargo test
```

`VITE_BOOMLY_MOCK=1 pnpm tauri dev` — окно на моковом бэкенде.

Архитектура и контракты Rust ↔ UI — [docs/contracts.md](docs/contracts.md), решения — [docs/decisions.md](docs/decisions.md).

## Стек

Tauri 2 · SvelteKit (static) · Svelte 5 · Tailwind CSS 4 · Rust (cpal, symphonia).

## Лицензия

[MIT](LICENSE) © Orion Z43 · Z43 Studios — [сайт](https://z43-studios.vercel.app/) · [Telegram](https://t.me/Orion_Z43)
