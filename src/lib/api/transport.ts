// Владелец: A. Выбор транспорта: настоящий Tauri или мок (браузер / VITE_BOOMLY_MOCK=1).
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ArgsOf, CommandName, ResultOf } from "./commands";
import type { EventMap, EventName } from "./events";
import { mockTransport } from "./mock";

export type Unlisten = () => void;

export interface Transport {
  call<C extends CommandName>(cmd: C, args: ArgsOf<C>): Promise<ResultOf<C>>;
  on<E extends EventName>(event: E, handler: (payload: EventMap[E]) => void): Promise<Unlisten>;
}

const tauriTransport: Transport = {
  call: (cmd, args) => invoke(cmd, args as Record<string, unknown>),
  on: (event, handler) => listen(event, (e) => handler(e.payload as never)),
};

export const useMock = import.meta.env.VITE_BOOMLY_MOCK === "1" || !isTauri();

export const transport: Transport = useMock ? mockTransport : tauriTransport;
