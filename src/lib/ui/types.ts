// Общие типы пропсов компонентов дизайн-системы.
import type { IconName } from "./icons/paths";

export interface BoardItem {
  id: string;
  name: string;
  emoji?: string | null;
  icon?: IconName;
  count?: number;
  /** Сейчас играет звук из этой доски. */
  playing?: boolean;
  /** Для «Избранного»: иконка жёлтая. */
  tone?: "default" | "favorite";
}

export type MenuItem =
  | {
      id: string;
      label: string;
      icon?: IconName;
      shortcut?: string;
      danger?: boolean;
      disabled?: boolean;
      /** Отметка (режим воспроизведения и т.п.). */
      checked?: boolean;
    }
  | { separator: true };

/** Состояния стора `updater` участка E (src/lib/updater/). */
export type UpdaterState = "idle" | "checking" | "available" | "downloading" | "ready" | "error";

export interface UpdateBannerLabels {
  available: string;
  install: string;
  later: string;
  notes: string;
  downloading: string;
  ready: string;
  readyText: string;
  restart: string;
  error: string;
  retry: string;
  close: string;
}
