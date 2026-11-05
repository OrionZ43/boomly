// Участок E — логика баннера обновления: когда проверять, состояние, действия.
// Визуальный компонент (участок C) получает `updater` и вызывает его методы; см. README.md.
export { updater, startAutoCheck, CHECK_INTERVAL_MS } from "./store.svelte";
export type { UpdaterState, UpdaterStatus } from "./store.svelte";
export { formatBytes, progressRatio } from "./format";
