// Тема: акцент и уровень эффектов. Значения — из AppSettings (`accent`, `effects`).
// D зовёт applyTheme(settings) при старте и на `app://settings-changed`.
import type { EffectsLevel } from "$lib/api/types";

export const ACCENTS = [
  { key: "cyan", hex: "#00f0ff", label: "Cyan" },
  { key: "purple", hex: "#b44dff", label: "Purple" },
  { key: "magenta", hex: "#ff00c1", label: "Magenta" },
  { key: "lime", hex: "#a6ff3d", label: "Lime" },
  { key: "orange", hex: "#ffa53a", label: "Orange" },
] as const;

export type AccentKey = (typeof ACCENTS)[number]["key"];

export interface ThemeOptions {
  /** Ключ из ACCENTS или `#rrggbb`. */
  accent?: string;
  effects?: EffectsLevel;
}

export function applyTheme({ accent, effects }: ThemeOptions, root: HTMLElement = document.documentElement) {
  if (accent !== undefined) {
    if (ACCENTS.some((a) => a.key === accent)) {
      root.dataset.accent = accent;
      root.style.removeProperty("--accent");
      root.style.removeProperty("--accent-ink");
    } else if (/^#[0-9a-f]{6}$/i.test(accent)) {
      delete root.dataset.accent;
      root.style.setProperty("--accent", accent);
      root.style.setProperty("--accent-ink", inkFor(accent));
    }
  }
  if (effects !== undefined) root.dataset.effects = effects;
}

/** Чёрный или белый текст поверх цвета — что контрастнее (WCAG). */
export function inkFor(hex: string): string {
  const ch = [1, 3, 5]
    .map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((x) => (x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4));
  const l = 0.2126 * ch[0] + 0.7152 * ch[1] + 0.0722 * ch[2];
  return (l + 0.05) / 0.05 >= 1.05 / (l + 0.05) ? "#07070a" : "#ffffff";
}

/** Пользователь просит меньше движения (системная настройка Windows). */
export function prefersReducedMotion(): boolean {
  return typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}
