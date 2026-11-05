// Участок D — i18n RU/EN. Язык берётся из AppSettings.language (store.app), по умолчанию ru.
//   import { t } from "$lib/i18n";   t("top.search"), t("import.added", { count: 3 })
import type { Language } from "$lib/api";
import { en } from "./en";
import { ru, type MessageKey } from "./ru";

export type { MessageKey };

const dicts: Record<Language, Record<MessageKey, string>> = { ru, en };

export const i18n = $state<{ lang: Language }>({ lang: "ru" });

export function setLanguage(lang: Language) {
  i18n.lang = lang;
  if (typeof document !== "undefined") document.documentElement.lang = lang;
}

export function t(key: MessageKey, params?: Record<string, string | number>): string {
  let s = dicts[i18n.lang][key] ?? ru[key] ?? key;
  if (params) for (const [k, v] of Object.entries(params)) s = s.replaceAll(`{${k}}`, String(v));
  return s;
}

export const LANGUAGES: { value: Language; label: string }[] = [
  { value: "ru", label: "Русский" },
  { value: "en", label: "English" },
];
