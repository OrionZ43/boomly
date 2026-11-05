// Владелец: A. Имена событий Rust → UI (зеркало src-tauri/src/events.rs) и их полезная нагрузка.
import type {
  AppError,
  AppSettings,
  AudioSettings,
  CableStatus,
  DeviceList,
  HotkeyTriggered,
  LibraryData,
  MicLevel,
  PlaybackEvent,
  ProgressEvent,
} from "./types";

export interface EventMap {
  "audio://playback": PlaybackEvent;
  "audio://progress": ProgressEvent[];
  "audio://devices-changed": DeviceList;
  "audio://cable-status": CableStatus;
  "audio://mic-level": MicLevel;
  "audio://settings-changed": AudioSettings;
  "audio://error": AppError;
  "library://changed": LibraryData;
  "hotkeys://triggered": HotkeyTriggered;
  "app://settings-changed": AppSettings;
}

export type EventName = keyof EventMap;
