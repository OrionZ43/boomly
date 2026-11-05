// Владелец: A (поддерживает D). Мок бэкенда в памяти: работает в браузере (`pnpm dev`)
// и в окне при VITE_BOOMLY_MOCK=1. Повторяет контракт docs/contracts.md, включая события.
import type { ArgsOf, CommandName, ResultOf } from "./commands";
import type { EventMap, EventName } from "./events";
import type { Transport } from "./transport";
import type {
  AppError,
  AppSettings,
  AudioSettings,
  CableStatus,
  DeviceList,
  HotkeyBinding,
  LibraryData,
  Sound,
} from "./types";

const listeners = new Map<string, Set<(p: unknown) => void>>();
function emit<E extends EventName>(event: E, payload: EventMap[E]) {
  listeners.get(event)?.forEach((h) => h(payload));
}
function fail(code: AppError["code"], message: string): never {
  throw { code, message } satisfies AppError;
}
const clone = <T>(v: T): T => structuredClone(v);
const uid = () => crypto.randomUUID();

const devices: DeviceList = {
  outputs: [
    { id: "out-headphones", name: "Наушники (Realtek)", isDefault: true, isCable: false },
    { id: "out-cable", name: "CABLE Input (VB-Audio Virtual Cable)", isDefault: false, isCable: true },
  ],
  inputs: [
    { id: "in-mic", name: "Микрофон (fifine)", isDefault: true, isCable: false },
    { id: "in-cable", name: "CABLE Output (VB-Audio Virtual Cable)", isDefault: false, isCable: true },
  ],
};
const cable: CableStatus = {
  installed: true,
  outputDeviceName: "CABLE Input (VB-Audio Virtual Cable)",
  inputDeviceName: "CABLE Output (VB-Audio Virtual Cable)",
  downloadUrl: "https://vb-audio.com/Cable/",
};
let audio: AudioSettings = {
  monitorDeviceId: null,
  cableDeviceId: null,
  micDeviceId: null,
  monitorVolume: 0.8,
  cableVolume: 0.8,
  micVolume: 1,
  micMuted: false,
  micPassthrough: true,
};
let app: AppSettings = {
  language: "ru",
  effects: "normal",
  accent: "cyan",
  autostart: false,
  closeToTray: true,
  startMinimized: false,
  copyImportedFiles: true,
  firstRunCompleted: false,
  checkUpdates: true,
};
const demoColors = ["cyan", "purple", "magenta", "yellow", "green", "orange"];
const demo: Sound[] = ["Бум", "Аплодисменты", "Сверчки", "Ба-дум-тсс", "Вау", "Сирена"].map(
  (name, i) => ({
    id: `demo-${i}`,
    name,
    path: `C:\\Users\\demo\\AppData\\Roaming\\Boomly\\sounds\\${i}.mp3`,
    color: demoColors[i % demoColors.length],
    emoji: null,
    volume: 1,
    mode: "toggle",
    favorite: i < 2,
    durationMs: 1500 + i * 700,
    addedAt: Date.now(),
  }),
);
const library: LibraryData = {
  version: 1,
  sounds: demo,
  boards: [
    { id: "board-memes", name: "Мемы", emoji: "😂", soundIds: demo.slice(0, 4).map((s) => s.id) },
    { id: "board-fx", name: "Эффекты", emoji: null, soundIds: demo.slice(3).map((s) => s.id) },
  ],
};
let hotkeys: HotkeyBinding[] = [{ accelerator: "Ctrl+Shift+S", action: { type: "stopAll" } }];

// Имитация воспроизведения: таймер вместо звука.
const playing = new Map<string, ReturnType<typeof setInterval>>();
function stopSound(id: string, state: "stopped" | "finished") {
  const t = playing.get(id);
  if (!t) return;
  clearInterval(t);
  playing.delete(id);
  emit("audio://playback", { soundId: id, state, durationMs: null });
}
function playSound(id: string) {
  const s = library.sounds.find((x) => x.id === id) ?? fail("not_found", `звук ${id} не найден`);
  if (playing.has(id)) {
    stopSound(id, "stopped");
    if (s.mode === "toggle") return;
  }
  const duration = s.durationMs ?? 2000;
  const started = Date.now();
  emit("audio://playback", { soundId: id, state: "started", durationMs: duration });
  playing.set(
    id,
    setInterval(() => {
      const pos = Date.now() - started;
      if (pos >= duration) return stopSound(id, "finished");
      emit("audio://progress", [{ soundId: id, positionMs: pos, durationMs: duration }]);
    }, 66),
  );
}
const libChanged = () => emit("library://changed", clone(library));
const audioChanged = () => {
  emit("audio://settings-changed", clone(audio));
  return clone(audio);
};

type Handlers = { [C in CommandName]: (args: ArgsOf<C>) => ResultOf<C> };

const handlers: Handlers = {
  audio_list_devices: () => clone(devices),
  audio_get_settings: () => clone(audio),
  audio_set_devices: ({ selection }) => {
    audio = { ...audio, ...selection };
    return audioChanged();
  },
  audio_set_volume: ({ channel, value }) => {
    if (value < 0 || value > 1) fail("invalid_argument", "громкость вне 0..1");
    audio = { ...audio, [`${channel}Volume`]: value };
    return audioChanged();
  },
  audio_set_mic_muted: ({ muted }) => {
    audio = { ...audio, micMuted: muted };
    return audioChanged();
  },
  audio_set_mic_passthrough: ({ enabled }) => {
    audio = { ...audio, micPassthrough: enabled };
    return audioChanged();
  },
  audio_cable_status: () => clone(cable),
  audio_play: ({ soundId }) => (playSound(soundId), null),
  audio_stop: ({ soundId }) => (stopSound(soundId, "stopped"), null),
  audio_stop_all: () => ([...playing.keys()].forEach((id) => stopSound(id, "stopped")), null),
  audio_play_test: () => null,

  library_get: () => clone(library),
  library_import: ({ request }) => {
    const added: Sound[] = request.paths.map((path) => ({
      id: uid(),
      name: path.split(/[\\/]/).pop()!.replace(/\.[^.]+$/, ""),
      path,
      color: "cyan",
      emoji: null,
      volume: 1,
      mode: "toggle",
      favorite: false,
      durationMs: 2000,
      addedAt: Date.now(),
    }));
    library.sounds.push(...added);
    const board = library.boards.find((b) => b.id === request.boardId);
    board?.soundIds.push(...added.map((s) => s.id));
    libChanged();
    return { added: clone(added), skipped: [] };
  },
  library_update_sound: ({ sound }) => {
    const i = library.sounds.findIndex((s) => s.id === sound.id);
    if (i < 0) fail("not_found", sound.id);
    library.sounds[i] = clone(sound);
    libChanged();
    return clone(sound);
  },
  library_delete_sounds: ({ ids }) => {
    library.sounds = library.sounds.filter((s) => !ids.includes(s.id));
    library.boards.forEach((b) => (b.soundIds = b.soundIds.filter((id) => !ids.includes(id))));
    libChanged();
    return null;
  },
  library_create_board: ({ name, emoji }) => {
    const board = { id: uid(), name, emoji, soundIds: [] };
    library.boards.push(board);
    libChanged();
    return clone(board);
  },
  library_update_board: ({ board }) => {
    const i = library.boards.findIndex((b) => b.id === board.id);
    if (i < 0) fail("not_found", board.id);
    library.boards[i] = clone(board);
    libChanged();
    return clone(board);
  },
  library_delete_board: ({ id }) => {
    library.boards = library.boards.filter((b) => b.id !== id);
    libChanged();
    return null;
  },
  library_reorder_boards: ({ ids }) => {
    library.boards.sort((a, b) => ids.indexOf(a.id) - ids.indexOf(b.id));
    libChanged();
    return null;
  },

  hotkeys_get: () => clone(hotkeys),
  hotkeys_set: ({ binding }) => {
    const taken = hotkeys.find(
      (h) => h.accelerator === binding.accelerator && JSON.stringify(h.action) !== JSON.stringify(binding.action),
    );
    if (taken) fail("hotkey_conflict", `${binding.accelerator} уже занято`);
    hotkeys = hotkeys.filter((h) => JSON.stringify(h.action) !== JSON.stringify(binding.action));
    hotkeys.push(clone(binding));
    return clone(hotkeys);
  },
  hotkeys_remove: ({ accelerator }) => {
    hotkeys = hotkeys.filter((h) => h.accelerator !== accelerator);
    return clone(hotkeys);
  },
  hotkeys_set_capture: () => null,

  app_get_settings: () => clone(app),
  app_set_settings: ({ settings }) => {
    app = clone(settings);
    emit("app://settings-changed", clone(app));
    return clone(app);
  },
  app_info: () => ({ version: "0.0.0-mock", dataDir: "C:\\Users\\demo\\AppData\\Roaming\\Boomly" }),
  app_open_data_dir: () => null,
};

export const mockTransport: Transport = {
  async call(cmd, args) {
    await new Promise((r) => setTimeout(r, 10));
    return (handlers[cmd] as (a: typeof args) => ResultOf<typeof cmd>)(args);
  },
  async on(event, handler) {
    const set = listeners.get(event) ?? new Set();
    listeners.set(event, set);
    const h = handler as (p: unknown) => void;
    set.add(h);
    return () => set.delete(h);
  },
};
