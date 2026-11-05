// Участок E — обёртка апдейтера для UI. Сигнатуры checkForUpdate/installUpdate — контракт
// (docs/contracts.md §3), их использует баннер. Реализация: @tauri-apps/plugin-updater.
//
// В моке (браузер / VITE_BOOMLY_MOCK=1) обновлений нет. Для витрины баннера:
// VITE_BOOMLY_MOCK_UPDATE=1 — мок «находит» версию 9.9.9 и имитирует скачивание.
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { transport, useMock } from "./transport";
import type { AppError, UpdateInfo, UpdateProgress } from "./types";

const mockUpdate = import.meta.env.VITE_BOOMLY_MOCK_UPDATE === "1";

/** Найденное, но ещё не установленное обновление (между check и install). */
let pending: Update | null = null;

function toAppError(e: unknown, code: AppError["code"] = "internal"): AppError {
  if (typeof e === "object" && e !== null && "code" in e && "message" in e) return e as AppError;
  return { code, message: e instanceof Error ? e.message : String(e) };
}

/** Есть ли в сборке публичный ключ апдейтера (команда `updater_enabled`). Без него обновления не ищем. */
export async function updaterEnabled(): Promise<boolean> {
  try {
    return await transport.call("updater_enabled", {});
  } catch {
    return false;
  }
}

/** null — обновлений нет (или апдейтер выключен в этой сборке). Ошибка сети — исключение AppError. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  if (useMock) {
    if (!mockUpdate) return null;
    return { version: "9.9.9", currentVersion: "0.1.0", notes: "Демо-обновление (мок).", date: null };
  }
  if (!(await updaterEnabled())) return null;
  try {
    const update = await check({ timeout: 15_000 });
    if (pending && pending !== update) void pending.close().catch(() => {});
    pending = update;
    if (!update) return null;
    return {
      version: update.version,
      currentVersion: update.currentVersion,
      notes: update.body ?? null,
      date: update.date ?? null,
    };
  } catch (e) {
    throw toAppError(e, "io");
  }
}

/**
 * Качает и ставит найденное checkForUpdate обновление.
 * Прогресс: downloading (много раз) → installing → done.
 * На Windows установщик NSIS (passive) закрывает приложение сам и запускает новую версию,
 * так что `done` обычно не наступает. Если процесс выжил — перезапускаемся сами.
 */
export async function installUpdate(onProgress?: (p: UpdateProgress) => void): Promise<void> {
  if (useMock) {
    if (!mockUpdate) throw { code: "not_found", message: "обновлений нет" } satisfies AppError;
    const total = 5_000_000;
    for (let d = 0; d <= total; d += 500_000) {
      onProgress?.({ phase: "downloading", downloaded: d, total });
      await new Promise((r) => setTimeout(r, 150));
    }
    onProgress?.({ phase: "installing" });
    await new Promise((r) => setTimeout(r, 600));
    onProgress?.({ phase: "done" });
    return;
  }
  if (!pending) {
    await checkForUpdate();
    if (!pending) throw { code: "not_found", message: "обновлений нет" } satisfies AppError;
  }
  const update = pending;
  let downloaded = 0;
  let total: number | null = null;
  try {
    await update.download((ev) => {
      if (ev.event === "Started") {
        total = ev.data.contentLength ?? null;
        onProgress?.({ phase: "downloading", downloaded, total });
      } else if (ev.event === "Progress") {
        downloaded += ev.data.chunkLength;
        onProgress?.({ phase: "downloading", downloaded, total });
      }
    });
    onProgress?.({ phase: "installing" });
    await update.install();
  } catch (e) {
    throw toAppError(e, "io");
  }
  pending = null;
  onProgress?.({ phase: "done" });
  await restartApp();
}

/** Перезапуск приложения (кнопка «Перезапустить» в баннере). */
export async function restartApp(): Promise<void> {
  if (useMock) {
    location.reload();
    return;
  }
  await relaunch();
}
