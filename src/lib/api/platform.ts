// Владелец: A. Платформенные возможности окна: выбор файлов, drag&drop, внешние ссылки.
// В браузере и при VITE_BOOMLY_MOCK=1 — мок (window.open, HTML5-drop с фиктивными путями).
import type { Unlisten } from "./transport";
import { useMock } from "./transport";

export const SOUND_EXTENSIONS = ["mp3", "wav", "ogg", "flac"] as const;

export type FileDropEvent = {
  type: "enter" | "over" | "leave" | "drop";
  /** Пустой у `over` и `leave`. */
  paths: string[];
};

export interface Platform {
  /** Диалог выбора звуков. `null` — пользователь отменил. */
  pickSoundFiles(opts?: { multiple?: boolean }): Promise<string[] | null>;
  /** Диалог выбора папки со звуками. */
  pickSoundFolder(): Promise<string | null>;
  /** Перетаскивание файлов/папок на окно. */
  onFileDrop(handler: (e: FileDropEvent) => void): Promise<Unlisten>;
  /** Открыть http(s)-ссылку во внешнем браузере. */
  openExternal(url: string): Promise<void>;
}

const asArray = (v: string | string[] | null): string[] | null =>
  v === null ? null : Array.isArray(v) ? v : [v];

const tauriPlatform: Platform = {
  async pickSoundFiles(opts) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const res = await open({
      multiple: opts?.multiple ?? true,
      directory: false,
      filters: [{ name: "Audio", extensions: [...SOUND_EXTENSIONS] }],
    });
    return asArray(res);
  },
  async pickSoundFolder() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const res = await open({ multiple: false, directory: true });
    return Array.isArray(res) ? (res[0] ?? null) : res;
  },
  async onFileDrop(handler) {
    const { getCurrentWebview } = await import("@tauri-apps/api/webview");
    return getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "enter" || p.type === "drop") handler({ type: p.type, paths: p.paths });
      else handler({ type: p.type, paths: [] });
    });
  },
  async openExternal(url) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  },
};

const mockPlatform: Platform = {
  async pickSoundFiles() {
    return ["C:\\Users\\demo\\Music\\boom.mp3", "C:\\Users\\demo\\Music\\clap.wav"];
  },
  async pickSoundFolder() {
    return "C:\\Users\\demo\\Music\\Memes";
  },
  async onFileDrop(handler) {
    const fake = (e: DragEvent) =>
      [...(e.dataTransfer?.files ?? [])].map((f) => `C:\\Users\\demo\\Dropped\\${f.name}`);
    const enter = (e: DragEvent) => (e.preventDefault(), handler({ type: "enter", paths: fake(e) }));
    const over = (e: DragEvent) => (e.preventDefault(), handler({ type: "over", paths: [] }));
    const leave = () => handler({ type: "leave", paths: [] });
    const drop = (e: DragEvent) => (e.preventDefault(), handler({ type: "drop", paths: fake(e) }));
    window.addEventListener("dragenter", enter);
    window.addEventListener("dragover", over);
    window.addEventListener("dragleave", leave);
    window.addEventListener("drop", drop);
    return () => {
      window.removeEventListener("dragenter", enter);
      window.removeEventListener("dragover", over);
      window.removeEventListener("dragleave", leave);
      window.removeEventListener("drop", drop);
    };
  },
  async openExternal(url) {
    window.open(url, "_blank", "noopener");
  },
};

export const platform: Platform = useMock ? mockPlatform : tauriPlatform;
