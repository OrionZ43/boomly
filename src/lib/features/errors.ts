// Участок D — текст ошибки по коду контракта (docs/contracts.md §2) и тост.
import { isAppError } from "$lib/api";
import { t, type MessageKey } from "$lib/i18n";
import { toast } from "$lib/ui";

export function errorText(e: unknown): { title: string; text?: string } {
  if (isAppError(e)) {
    const key = `error.${e.code}` as MessageKey;
    const title = t(key) === key ? t("error.unknown") : t(key);
    return { title, text: e.message || undefined };
  }
  return { title: t("error.unknown"), text: e instanceof Error ? e.message : String(e ?? "") || undefined };
}

export function showError(e: unknown) {
  console.error("[boomly]", e);
  toast.error(errorText(e));
}

/** Выполнить и показать тост при ошибке. Возвращает результат или undefined. */
export async function attempt<T>(fn: () => Promise<T>): Promise<T | undefined> {
  try {
    return await fn();
  } catch (e) {
    showError(e);
    return undefined;
  }
}
