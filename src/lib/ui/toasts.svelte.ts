// Очередь тостов. Использование: `toast.success("Импортировано 5 звуков")`, `<Toaster />` один раз в layout.
export type ToastKind = "info" | "success" | "warning" | "error";

export interface ToastItem {
  id: number;
  kind: ToastKind;
  title: string;
  text?: string;
  /** мс; 0 — пока не закроют. */
  duration: number;
  action?: { label: string; run: () => void };
}

type ToastInput = Omit<ToastItem, "id" | "kind" | "duration"> & { duration?: number };

let next = 1;
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export const toasts = $state<{ items: ToastItem[] }>({ items: [] });

function push(kind: ToastKind, input: ToastInput | string): number {
  const t: ToastInput = typeof input === "string" ? { title: input } : input;
  const id = next++;
  const duration = t.duration ?? (kind === "error" ? 8000 : 4000);
  toasts.items.push({ ...t, id, kind, duration });
  if (toasts.items.length > 4) dismiss(toasts.items[0].id);
  if (duration > 0) timers.set(id, setTimeout(() => dismiss(id), duration));
  return id;
}

export function dismiss(id: number) {
  clearTimeout(timers.get(id));
  timers.delete(id);
  const i = toasts.items.findIndex((t) => t.id === id);
  if (i >= 0) toasts.items.splice(i, 1);
}

export const toast = {
  info: (t: ToastInput | string) => push("info", t),
  success: (t: ToastInput | string) => push("success", t),
  warning: (t: ToastInput | string) => push("warning", t),
  error: (t: ToastInput | string) => push("error", t),
  dismiss,
};
