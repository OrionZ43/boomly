// Участок E — состояние апдейтера (Svelte 5 runes). Один экземпляр на приложение.
//
// Автопроверка: при старте (через STARTUP_DELAY_MS) и раз в CHECK_INTERVAL_MS, пока
// AppSettings.checkUpdates = true. Ручная проверка (кнопка в «Инфо») — updater.checkNow(),
// работает и при выключенной автопроверке.
import { api, on } from "$lib/api";
import type { AppError, UpdateInfo } from "$lib/api";
import { checkForUpdate, installUpdate, restartApp, updaterEnabled } from "$lib/api/updater";

/** Раз в 6 часов. */
export const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;
/** Не мешаем старту приложения. */
export const STARTUP_DELAY_MS = 5_000;

export type UpdaterStatus =
  | "disabled" // в сборке нет ключа подписи — обновления не ищем
  | "idle" // ещё не проверяли
  | "checking"
  | "upToDate"
  | "available" // показать баннер: «Доступна версия X» [Обновить] [Позже]
  | "downloading" // прогресс
  | "installing"
  | "ready" // установлено, нужен перезапуск: [Перезапустить]
  | "error";

export interface UpdaterState {
  status: UpdaterStatus;
  info: UpdateInfo | null;
  downloaded: number;
  total: number | null;
  error: AppError | null;
  /** Unix ms последней успешной проверки. */
  lastCheckedAt: number | null;
  /** Пользователь нажал «Позже» для этой версии — баннер не показываем до следующей версии. */
  dismissedVersion: string | null;
}

class Updater implements UpdaterState {
  status = $state<UpdaterStatus>("idle");
  info = $state<UpdateInfo | null>(null);
  downloaded = $state(0);
  total = $state<number | null>(null);
  error = $state<AppError | null>(null);
  lastCheckedAt = $state<number | null>(null);
  dismissedVersion = $state<string | null>(null);

  /** Показывать ли баннер сейчас. */
  get bannerVisible(): boolean {
    if (this.status === "downloading" || this.status === "installing" || this.status === "ready") return true;
    if (this.status === "error") return this.info !== null; // ошибка установки — показать в баннере
    return this.status === "available" && this.info?.version !== this.dismissedVersion;
  }

  private busy(): boolean {
    return this.status === "checking" || this.status === "downloading" || this.status === "installing";
  }

  /** Проверить сейчас. Возвращает найденное обновление или null. */
  async checkNow(): Promise<UpdateInfo | null> {
    if (this.busy() || this.status === "ready") return this.info;
    if (!(await updaterEnabled())) {
      this.status = "disabled";
      return null;
    }
    this.status = "checking";
    this.error = null;
    try {
      const info = await checkForUpdate();
      this.info = info;
      this.lastCheckedAt = Date.now();
      this.status = info ? "available" : "upToDate";
      return info;
    } catch (e) {
      this.error = e as AppError;
      this.info = null; // ошибка проверки — баннер не показываем, только статус в «Инфо»
      this.status = "error";
      return null;
    }
  }

  /** «Обновить». */
  async install(): Promise<void> {
    if (this.busy() || !this.info) return;
    this.status = "downloading";
    this.downloaded = 0;
    this.total = null;
    this.error = null;
    try {
      await installUpdate((p) => {
        if (p.phase === "downloading") {
          this.downloaded = p.downloaded;
          this.total = p.total;
        } else if (p.phase === "installing") {
          this.status = "installing";
        } else {
          this.status = "ready";
        }
      });
      this.status = "ready";
    } catch (e) {
      this.error = e as AppError;
      this.status = "error";
    }
  }

  /** «Перезапустить» (если установщик не перезапустил приложение сам). */
  restart(): Promise<void> {
    return restartApp();
  }

  /** «Позже». */
  dismiss(): void {
    if (this.info) this.dismissedVersion = this.info.version;
    if (this.status === "error") this.status = this.info ? "available" : "idle";
  }
}

export const updater = new Updater();

/**
 * Запускает автопроверку. Вызывать один раз из корневого layout (onMount); возвращает
 * функцию остановки. Сам следит за AppSettings.checkUpdates.
 */
export function startAutoCheck(): () => void {
  let enabled = false;
  let timer: ReturnType<typeof setInterval> | null = null;
  let startup: ReturnType<typeof setTimeout> | null = null;
  let stopped = false;
  let unlisten: (() => void) | null = null;

  const apply = (checkUpdates: boolean) => {
    if (checkUpdates === enabled) return;
    enabled = checkUpdates;
    if (startup) clearTimeout(startup);
    if (timer) clearInterval(timer);
    startup = timer = null;
    if (!enabled) return;
    startup = setTimeout(() => void updater.checkNow(), STARTUP_DELAY_MS);
    timer = setInterval(() => void updater.checkNow(), CHECK_INTERVAL_MS);
  };

  api.app
    .getSettings()
    .then((s) => !stopped && apply(s.checkUpdates))
    .catch(() => !stopped && apply(true));
  on("app://settings-changed", (s) => apply(s.checkUpdates)).then((u) => {
    if (stopped) u();
    else unlisten = u;
  });

  return () => {
    stopped = true;
    apply(false);
    unlisten?.();
  };
}
