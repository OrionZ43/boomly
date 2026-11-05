// Участок E — мелкие помощники для баннера.

/** 0..1 или null, если размер неизвестен. */
export function progressRatio(downloaded: number, total: number | null): number | null {
  if (!total || total <= 0) return null;
  return Math.min(1, Math.max(0, downloaded / total));
}

/** «3,4 МБ» / «3.4 MB». */
export function formatBytes(bytes: number, lang: "ru" | "en" = "ru"): string {
  const units = lang === "ru" ? ["Б", "КБ", "МБ", "ГБ"] : ["B", "KB", "MB", "GB"];
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  const s = v.toLocaleString(lang === "ru" ? "ru-RU" : "en-US", { maximumFractionDigits: i === 0 ? 0 : 1 });
  return `${s} ${units[i]}`;
}
