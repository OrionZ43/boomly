// Участок D — состояние приложения. Один экземпляр `app`, данные только через $lib/api.
// init() загружает всё и подписывается на события Rust; экраны читают поля и зовут действия.
import { api, on, type AppSettings, type AudioSettings, type CableStatus, type DeviceList } from "$lib/api";
import type { AppInfo, HotkeyBinding, LibraryData, Sound, VolumeChannel } from "$lib/api";
import { attempt, showError } from "$lib/features/errors";
import { setLanguage } from "$lib/i18n";
import { applyTheme } from "$lib/ui";

/** Виртуальные доски. */
export const ALL = "__all";
export const FAVORITES = "__fav";

export interface PlayState {
  /** 0..1 или null, если длительность неизвестна. */
  progress: number | null;
}

class AppStore {
  ready = $state(false);
  library = $state<LibraryData>({ version: 1, sounds: [], boards: [] });
  audio = $state<AudioSettings | null>(null);
  settings = $state<AppSettings | null>(null);
  devices = $state<DeviceList>({ outputs: [], inputs: [] });
  cable = $state<CableStatus | null>(null);
  hotkeys = $state<HotkeyBinding[]>([]);
  info = $state<AppInfo | null>(null);
  playing = $state<Record<string, PlayState>>({});
  micLevel = $state(0);

  activeBoard = $state<string>(ALL);
  query = $state("");

  #unlisten: (() => void)[] = [];

  // ---------- производные ----------
  get soundsById(): Map<string, Sound> {
    return new Map(this.library.sounds.map((s) => [s.id, s]));
  }

  /** Хоткей звука → сочетание. */
  get hotkeyBySound(): Map<string, string> {
    const m = new Map<string, string>();
    for (const h of this.hotkeys) if (h.action.type === "playSound") m.set(h.action.soundId, h.accelerator);
    return m;
  }

  hotkeyFor(type: "stopAll" | "toggleMicMute"): string | null {
    return this.hotkeys.find((h) => h.action.type === type)?.accelerator ?? null;
  }

  get currentBoard() {
    return this.library.boards.find((b) => b.id === this.activeBoard) ?? null;
  }

  /** Звуки на экране: поиск идёт по всей библиотеке. */
  get visibleSounds(): Sound[] {
    const q = this.query.trim().toLowerCase();
    if (q) return this.library.sounds.filter((s) => s.name.toLowerCase().includes(q));
    if (this.activeBoard === FAVORITES) return this.library.sounds.filter((s) => s.favorite);
    const board = this.currentBoard;
    if (board) {
      const by = this.soundsById;
      return board.soundIds.map((id) => by.get(id)).filter((s): s is Sound => !!s);
    }
    return this.library.sounds;
  }

  get anyPlaying(): boolean {
    return Object.keys(this.playing).length > 0;
  }

  // ---------- старт ----------
  async init() {
    if (this.ready) return;
    try {
      const [library, audio, settings, devices, cable, hotkeys, info] = await Promise.all([
        api.library.get(),
        api.audio.getSettings(),
        api.app.getSettings(),
        api.audio.listDevices(),
        api.audio.cableStatus(),
        api.hotkeys.get(),
        api.app.info(),
      ]);
      this.library = library;
      this.audio = audio;
      this.setSettingsLocal(settings);
      this.devices = devices;
      this.cable = cable;
      this.hotkeys = hotkeys;
      this.info = info;
    } catch (e) {
      showError(e);
    }
    this.#unlisten = await Promise.all([
      on("library://changed", (d) => (this.library = d)),
      on("audio://settings-changed", (a) => (this.audio = a)),
      on("app://settings-changed", (s) => this.setSettingsLocal(s)),
      on("audio://devices-changed", (d) => (this.devices = d)),
      on("audio://cable-status", (c) => (this.cable = c)),
      on("audio://mic-level", (m) => (this.micLevel = m.peak)),
      on("audio://error", (e) => showError(e)),
      on("audio://playback", (p) => {
        if (p.state === "started") this.playing[p.soundId] = { progress: p.durationMs ? 0 : null };
        else delete this.playing[p.soundId];
        if (p.state === "error") showError({ code: "decode", message: this.soundsById.get(p.soundId)?.name ?? "" });
      }),
      on("audio://progress", (list) => {
        for (const p of list)
          if (this.playing[p.soundId]) this.playing[p.soundId].progress = p.durationMs ? p.positionMs / p.durationMs : null;
      }),
      on("hotkeys://triggered", () => {
        // Действие уже выполнил Rust; UI обновится по событиям playback/settings.
      }),
    ]);
    this.ready = true;
  }

  destroy() {
    this.#unlisten.forEach((u) => u());
    this.#unlisten = [];
  }

  private setSettingsLocal(s: AppSettings) {
    this.settings = s;
    setLanguage(s.language);
    applyTheme({ accent: s.accent, effects: s.effects });
  }

  // ---------- воспроизведение ----------
  play = (id: string) => attempt(() => api.audio.play(id));
  stopAll = () => attempt(() => api.audio.stopAll());

  // ---------- громкость (с прореживанием вызовов во время перетаскивания) ----------
  #volTimers: Partial<Record<VolumeChannel, ReturnType<typeof setTimeout>>> = {};
  setVolume(channel: VolumeChannel, value: number) {
    if (this.audio) this.audio = { ...this.audio, [`${channel}Volume`]: value };
    clearTimeout(this.#volTimers[channel]);
    this.#volTimers[channel] = setTimeout(() => attempt(() => api.audio.setVolume(channel, value)), 60);
  }
  async toggleMicMute() {
    if (!this.audio) return;
    const a = await attempt(() => api.audio.setMicMuted(!this.audio!.micMuted));
    if (a) this.audio = a;
  }
  async setPassthrough(enabled: boolean) {
    const a = await attempt(() => api.audio.setMicPassthrough(enabled));
    if (a) this.audio = a;
  }
  async setDevices(sel: Parameters<typeof api.audio.setDevices>[0]) {
    const a = await attempt(() => api.audio.setDevices(sel));
    if (a) this.audio = a;
  }
  async refreshCable() {
    const [c, d] = await Promise.all([attempt(() => api.audio.cableStatus()), attempt(() => api.audio.listDevices())]);
    if (c) this.cable = c;
    if (d) this.devices = d;
  }

  // ---------- настройки ----------
  async updateSettings(patch: Partial<AppSettings>) {
    if (!this.settings) return;
    const next = { ...this.settings, ...patch };
    this.setSettingsLocal(next); // сразу, чтобы язык/тема переключались без задержки
    const s = await attempt(() => api.app.setSettings(next));
    if (s) this.setSettingsLocal(s);
  }

  // ---------- библиотека ----------
  /** Доска для импорта: реальная текущая или никакая. */
  get importBoardId(): string | null {
    return this.currentBoard?.id ?? null;
  }
  async importPaths(paths: string[]) {
    if (!paths.length) return null;
    return attempt(() =>
      api.library.import({
        paths,
        boardId: this.importBoardId,
        copyFiles: this.settings?.copyImportedFiles ?? null,
      }),
    );
  }
  async saveSound(sound: Sound) {
    return attempt(() => api.library.updateSound(sound));
  }
  async toggleFavorite(id: string) {
    const s = this.soundsById.get(id);
    if (s) await this.saveSound({ ...s, favorite: !s.favorite });
  }
  async deleteSounds(ids: string[]) {
    for (const id of ids) {
      const acc = this.hotkeyBySound.get(id);
      if (acc) await this.removeHotkey(acc);
    }
    return attempt(() => api.library.deleteSounds(ids));
  }
  async setBoardMembership(soundId: string, boardIds: string[]) {
    for (const b of this.library.boards) {
      const has = b.soundIds.includes(soundId);
      const want = boardIds.includes(b.id);
      if (has === want) continue;
      const soundIds = want ? [...b.soundIds, soundId] : b.soundIds.filter((x) => x !== soundId);
      await attempt(() => api.library.updateBoard({ ...b, soundIds }));
    }
  }
  async removeFromBoard(soundId: string, boardId: string) {
    const b = this.library.boards.find((x) => x.id === boardId);
    if (b) await attempt(() => api.library.updateBoard({ ...b, soundIds: b.soundIds.filter((x) => x !== soundId) }));
  }
  async createBoard(name: string, emoji: string | null) {
    const b = await attempt(() => api.library.createBoard(name, emoji));
    if (b) this.activeBoard = b.id;
    return b;
  }
  async updateBoard(id: string, name: string, emoji: string | null) {
    const b = this.library.boards.find((x) => x.id === id);
    if (b) await attempt(() => api.library.updateBoard({ ...b, name, emoji }));
  }
  async deleteBoard(id: string) {
    await attempt(() => api.library.deleteBoard(id));
    if (this.activeBoard === id) this.activeBoard = ALL;
  }
  async moveBoard(id: string, delta: -1 | 1) {
    const ids = this.library.boards.map((b) => b.id);
    const i = ids.indexOf(id);
    const j = i + delta;
    if (i < 0 || j < 0 || j >= ids.length) return;
    [ids[i], ids[j]] = [ids[j], ids[i]];
    await attempt(() => api.library.reorderBoards(ids));
  }

  // ---------- хоткеи ----------
  /** Назначить; ошибку (в т.ч. hotkey_conflict) пробрасывает вызывающему. */
  async setHotkey(binding: HotkeyBinding) {
    this.hotkeys = await api.hotkeys.set(binding);
  }
  async removeHotkey(accelerator: string) {
    const h = await attempt(() => api.hotkeys.remove(accelerator));
    if (h) this.hotkeys = h;
  }
}

export const app = new AppStore();
