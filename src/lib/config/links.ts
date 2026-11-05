// Владелец: A. Ссылки кнопки «Инфо» — как в Protogenix (lib/core/widgets/z43_branding.dart).
export const REPO_URL = "https://github.com/OrionZ43/boomly";
export const RELEASES_URL = `${REPO_URL}/releases`;
export const VB_CABLE_URL = "https://vb-audio.com/Cable/";

export interface InfoLink {
  id: "site" | "github" | "telegram";
  /** Ключ перевода; RU-текст по умолчанию — `label`. */
  label: string;
  url: string;
}

export const INFO_LINKS: readonly InfoLink[] = [
  { id: "site", label: "Сайт", url: "https://z43-studios.vercel.app/" },
  { id: "github", label: "GitHub", url: REPO_URL },
  { id: "telegram", label: "Telegram", url: "https://t.me/Orion_Z43" },
];

export const BRAND_SIGNATURE = "СДЕЛАНО С ❤ Z43 STUDIOS";
