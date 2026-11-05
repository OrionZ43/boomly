# Участок E — логика баннера обновления

Визуальный компонент баннера делает C (`src/lib/ui/**`), подключает его в layout A/D.
Логика и состояние — здесь, компонент только читает поля и зовёт методы.

```ts
import { updater, startAutoCheck, progressRatio, formatBytes } from "$lib/updater";

// один раз в корневом +layout.svelte:
onMount(() => startAutoCheck()); // проверка через 5 с после старта и раз в 6 ч, если AppSettings.checkUpdates
```

| Поле / метод | Что это |
|---|---|
| `updater.bannerVisible` | показывать ли баннер |
| `updater.status` | `disabled` · `idle` · `checking` · `upToDate` · `available` · `downloading` · `installing` · `ready` · `error` |
| `updater.info` | `{ version, currentVersion, notes, date }` или `null` |
| `updater.downloaded`, `updater.total` | байты; `progressRatio(downloaded, total)` → 0..1 или `null` (неизвестно) |
| `updater.error` | `AppError` (`code`: `io` — сеть/подпись, `not_found`, `internal`) |
| `updater.install()` | «Обновить» |
| `updater.dismiss()` | «Позже» (скрыть до следующей версии) |
| `updater.restart()` | «Перезапустить» (статус `ready`) |
| `updater.checkNow()` | «Проверить обновления» на странице «Инфо»; `lastCheckedAt` — время проверки |

Состояния баннера: `available` → «Доступна версия {info.version}» [Обновить] [Позже], `notes` —
простым текстом. `downloading` → полоска прогресса и «{formatBytes(downloaded)} из
{formatBytes(total)}». `installing` → «Устанавливаем…» (на Windows приложение закроется само и
откроется новая версия). `ready` → [Перезапустить]. `error` с `info` → текст ошибки и [Повторить]
(`install()`), [Позже].

Для витрины без Tauri: `VITE_BOOMLY_MOCK_UPDATE=1 pnpm dev` — мок «находит» 9.9.9 и имитирует
скачивание.
