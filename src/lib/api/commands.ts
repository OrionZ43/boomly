// Владелец: A. Сигнатуры всех Tauri-команд: имя команды → аргументы → результат.
// Ключи аргументов — camelCase (Tauri сам переводит в snake_case параметров Rust).
import type {
  AppInfo,
  AppSettings,
  AudioSettings,
  Board,
  CableStatus,
  DeviceList,
  DeviceSelection,
  HotkeyBinding,
  ImportRequest,
  ImportResult,
  LibraryData,
  Sound,
  TestTarget,
  VolumeChannel,
} from "./types";

type Cmd<A, R> = { args: A; result: R };
type None = Record<string, never>;

export interface CommandMap {
  // audio (B)
  audio_list_devices: Cmd<None, DeviceList>;
  audio_get_settings: Cmd<None, AudioSettings>;
  audio_set_devices: Cmd<{ selection: DeviceSelection }, AudioSettings>;
  audio_set_volume: Cmd<{ channel: VolumeChannel; value: number }, AudioSettings>;
  audio_set_mic_muted: Cmd<{ muted: boolean }, AudioSettings>;
  audio_set_mic_passthrough: Cmd<{ enabled: boolean }, AudioSettings>;
  audio_cable_status: Cmd<None, CableStatus>;
  audio_play: Cmd<{ soundId: string }, null>;
  audio_stop: Cmd<{ soundId: string }, null>;
  audio_stop_all: Cmd<None, null>;
  audio_play_test: Cmd<{ target: TestTarget }, null>;
  // library (B2)
  library_get: Cmd<None, LibraryData>;
  library_import: Cmd<{ request: ImportRequest }, ImportResult>;
  library_update_sound: Cmd<{ sound: Sound }, Sound>;
  library_delete_sounds: Cmd<{ ids: string[] }, null>;
  library_create_board: Cmd<{ name: string; emoji: string | null }, Board>;
  library_update_board: Cmd<{ board: Board }, Board>;
  library_delete_board: Cmd<{ id: string }, null>;
  library_reorder_boards: Cmd<{ ids: string[] }, null>;
  // hotkeys (B2)
  hotkeys_get: Cmd<None, HotkeyBinding[]>;
  hotkeys_set: Cmd<{ binding: HotkeyBinding }, HotkeyBinding[]>;
  hotkeys_remove: Cmd<{ accelerator: string }, HotkeyBinding[]>;
  hotkeys_set_capture: Cmd<{ active: boolean }, null>;
  // system (B2)
  app_get_settings: Cmd<None, AppSettings>;
  app_set_settings: Cmd<{ settings: AppSettings }, AppSettings>;
  app_info: Cmd<None, AppInfo>;
  app_open_data_dir: Cmd<None, null>;
  // updater (E)
  updater_enabled: Cmd<None, boolean>;
}

export type CommandName = keyof CommandMap;
export type ArgsOf<C extends CommandName> = CommandMap[C]["args"];
export type ResultOf<C extends CommandName> = CommandMap[C]["result"];
