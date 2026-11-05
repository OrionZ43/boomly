// Участок E — обёртка апдейтера для UI. Сейчас заглушка: «обновлений нет».
// E подключит @tauri-apps/plugin-updater и оставит эти сигнатуры (их использует D/баннер).
import type { UpdateInfo, UpdateProgress } from "./types";

export async function checkForUpdate(): Promise<UpdateInfo | null> {
  return null;
}

export async function installUpdate(_onProgress?: (p: UpdateProgress) => void): Promise<void> {
  throw { code: "not_implemented", message: "updater ещё не подключён" };
}
