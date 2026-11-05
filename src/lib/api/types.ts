// Владелец: A. Зеркало Rust-типов контракта (src-tauri/src/*/types.rs). Менять вместе с
// docs/contracts.md и Rust-типами — одним коммитом.

// ---------- общее ----------
export type ErrorCode =
  | "not_implemented"
  | "not_found"
  | "device_unavailable"
  | "cable_missing"
  | "io"
  | "decode"
  | "hotkey_conflict"
  | "invalid_argument"
  | "internal";

export interface AppError {
  code: ErrorCode;
  message: string;
}

// ---------- audio (B) ----------
export interface AudioDevice {
  id: string;
  name: string;
  isDefault: boolean;
  isCable: boolean;
}

export interface DeviceList {
  outputs: AudioDevice[];
  inputs: AudioDevice[];
}

/** monitor — «Я слышу», cable — «В микрофон» (звуки), mic — «Голос». */
export type VolumeChannel = "monitor" | "cable" | "mic";

export interface AudioSettings {
  monitorDeviceId: string | null;
  cableDeviceId: string | null;
  micDeviceId: string | null;
  monitorVolume: number;
  cableVolume: number;
  micVolume: number;
  micMuted: boolean;
  micPassthrough: boolean;
}

/** Поле отсутствует — не менять; `null` — устройство по умолчанию / авто. */
export interface DeviceSelection {
  monitorDeviceId?: string | null;
  cableDeviceId?: string | null;
  micDeviceId?: string | null;
}

export interface CableStatus {
  installed: boolean;
  outputDeviceName: string | null;
  inputDeviceName: string | null;
  downloadUrl: string;
}

export type TestTarget = "monitor" | "cable";

export type PlaybackState = "started" | "stopped" | "finished" | "error";

export interface PlaybackEvent {
  soundId: string;
  state: PlaybackState;
  durationMs: number | null;
}

export interface ProgressEvent {
  soundId: string;
  positionMs: number;
  durationMs: number;
}

export interface MicLevel {
  peak: number;
}

// ---------- library (B2) ----------
export type PlayMode = "toggle" | "restart" | "overlay";

export interface Sound {
  id: string;
  name: string;
  path: string;
  /** Ключ палитры (`cyan`, `purple`, ...) или `#rrggbb`. */
  color: string;
  emoji: string | null;
  /** 0..2, 1 — как есть. */
  volume: number;
  mode: PlayMode;
  favorite: boolean;
  durationMs: number | null;
  addedAt: number;
}

export interface Board {
  id: string;
  name: string;
  emoji: string | null;
  soundIds: string[];
}

export interface LibraryData {
  version: number;
  sounds: Sound[];
  boards: Board[];
}

export interface ImportRequest {
  paths: string[];
  boardId?: string | null;
  copyFiles?: boolean | null;
}

export interface ImportResult {
  added: Sound[];
  skipped: string[];
}

// ---------- hotkeys (B2) ----------
export type HotkeyAction =
  | { type: "playSound"; soundId: string }
  | { type: "stopAll" }
  | { type: "toggleMicMute" };

export interface HotkeyBinding {
  /** Формат global-shortcut: `Ctrl+Shift+F1`. */
  accelerator: string;
  action: HotkeyAction;
}

export interface HotkeyTriggered {
  accelerator: string;
  action: HotkeyAction;
}

// ---------- system (B2) ----------
export type Language = "ru" | "en";
export type EffectsLevel = "minimal" | "normal";

export interface AppSettings {
  language: Language;
  effects: EffectsLevel;
  accent: string;
  autostart: boolean;
  closeToTray: boolean;
  startMinimized: boolean;
  copyImportedFiles: boolean;
  firstRunCompleted: boolean;
  checkUpdates: boolean;
}

export interface AppInfo {
  version: string;
  dataDir: string;
}

// ---------- updater (E) ----------
export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes: string | null;
  date: string | null;
}

export type UpdateProgress =
  | { phase: "downloading"; downloaded: number; total: number | null }
  | { phase: "installing" }
  | { phase: "done" };
