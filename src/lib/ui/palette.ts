// Палитра плиток звука. `Sound.color` — ключ отсюда или `#rrggbb` (docs/contracts.md, types.ts).
// Жёлтого здесь нет намеренно: жёлтый в интерфейсе означает «играет/избранное».

export interface TileColor {
  key: string;
  hex: string;
  /** Подпись для пикера (RU/EN — переводит D, тут — ключ i18n по умолчанию). */
  label: string;
}

export const TILE_COLORS: readonly TileColor[] = [
  { key: "cyan", hex: "#00f0ff", label: "Cyan" },
  { key: "blue", hex: "#3d8bff", label: "Blue" },
  { key: "purple", hex: "#a43dff", label: "Purple" },
  { key: "magenta", hex: "#ff00c1", label: "Magenta" },
  { key: "red", hex: "#ff3355", label: "Red" },
  { key: "orange", hex: "#ff8a1f", label: "Orange" },
  { key: "lime", hex: "#a6ff3d", label: "Lime" },
  { key: "green", hex: "#20e38a", label: "Green" },
  { key: "teal", hex: "#14b8a6", label: "Teal" },
  { key: "slate", hex: "#8a94a8", label: "Slate" },
] as const;

export const DEFAULT_TILE_COLOR = "cyan";

const HEX_RE = /^#[0-9a-f]{6}$/i;

/** Ключ палитры или `#rrggbb` → `#rrggbb`. Неизвестное → цвет по умолчанию. */
export function resolveColor(color: string | null | undefined): string {
  if (!color) return TILE_COLORS[0].hex;
  if (HEX_RE.test(color)) return color.toLowerCase();
  return TILE_COLORS.find((c) => c.key === color)?.hex ?? TILE_COLORS[0].hex;
}

/** Детерминированный цвет по строке (id звука) — для новых плиток без цвета. */
export function colorForId(id: string): string {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) | 0;
  return TILE_COLORS[Math.abs(h) % TILE_COLORS.length].key;
}
