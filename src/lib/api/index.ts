// Владелец: A. Единая точка входа UI → Rust. Экраны (D) импортируют только отсюда:
//   import { api, on } from "$lib/api";
// В браузере и при VITE_BOOMLY_MOCK=1 работает мок (./mock.ts).
import type { ArgsOf, CommandName, ResultOf } from "./commands";
import { transport } from "./transport";
import type * as T from "./types";

export * from "./types";
export type { EventMap, EventName } from "./events";
export { useMock } from "./transport";
export type { Unlisten } from "./transport";
export { platform, SOUND_EXTENSIONS } from "./platform";
export type { FileDropEvent, Platform } from "./platform";

const call = <C extends CommandName>(cmd: C, args: ArgsOf<C>): Promise<ResultOf<C>> =>
  transport.call(cmd, args);

export const on = transport.on;

export const api = {
  audio: {
    listDevices: () => call("audio_list_devices", {}),
    getSettings: () => call("audio_get_settings", {}),
    setDevices: (selection: T.DeviceSelection) => call("audio_set_devices", { selection }),
    setVolume: (channel: T.VolumeChannel, value: number) => call("audio_set_volume", { channel, value }),
    setMicMuted: (muted: boolean) => call("audio_set_mic_muted", { muted }),
    setMicPassthrough: (enabled: boolean) => call("audio_set_mic_passthrough", { enabled }),
    cableStatus: () => call("audio_cable_status", {}),
    play: (soundId: string) => call("audio_play", { soundId }),
    stop: (soundId: string) => call("audio_stop", { soundId }),
    stopAll: () => call("audio_stop_all", {}),
    playTest: (target: T.TestTarget) => call("audio_play_test", { target }),
  },
  library: {
    get: () => call("library_get", {}),
    import: (request: T.ImportRequest) => call("library_import", { request }),
    updateSound: (sound: T.Sound) => call("library_update_sound", { sound }),
    deleteSounds: (ids: string[]) => call("library_delete_sounds", { ids }),
    createBoard: (name: string, emoji: string | null = null) => call("library_create_board", { name, emoji }),
    updateBoard: (board: T.Board) => call("library_update_board", { board }),
    deleteBoard: (id: string) => call("library_delete_board", { id }),
    reorderBoards: (ids: string[]) => call("library_reorder_boards", { ids }),
  },
  hotkeys: {
    get: () => call("hotkeys_get", {}),
    set: (binding: T.HotkeyBinding) => call("hotkeys_set", { binding }),
    remove: (accelerator: string) => call("hotkeys_remove", { accelerator }),
    setCapture: (active: boolean) => call("hotkeys_set_capture", { active }),
  },
  app: {
    getSettings: () => call("app_get_settings", {}),
    setSettings: (settings: T.AppSettings) => call("app_set_settings", { settings }),
    info: () => call("app_info", {}),
    openDataDir: () => call("app_open_data_dir", {}),
  },
};

/** Ошибка от Rust/мока — `{ code, message }`. */
export function isAppError(e: unknown): e is T.AppError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}
