// Участок C — дизайн-система Boomly. Импорт: `import { Button, SoundTile } from "$lib/ui"`.
// Витрина всех компонентов и состояний: /ui. Токены: src/app.css.

export { default as Icon } from "./icons/Icon.svelte";
export { icons, type IconName } from "./icons/paths";

export { default as Button } from "./Button.svelte";
export { default as IconButton } from "./IconButton.svelte";
export { default as Kbd } from "./Kbd.svelte";
export { default as Panel } from "./Panel.svelte";
export { default as BoomlyLogo } from "./BoomlyLogo.svelte";

export { default as SoundTile } from "./SoundTile.svelte";
export { default as AddTile } from "./AddTile.svelte";
export { default as SoundGrid } from "./SoundGrid.svelte";
export { default as EmptyState } from "./EmptyState.svelte";
export { default as BoardList } from "./BoardList.svelte";

export { default as VolumeSlider } from "./VolumeSlider.svelte";
export { default as Switch } from "./Switch.svelte";
export { default as Segmented } from "./Segmented.svelte";
export { default as SearchField } from "./SearchField.svelte";
export { default as ColorSwatches } from "./ColorSwatches.svelte";

export { default as Modal } from "./Modal.svelte";
export { default as Menu, menuAnchor } from "./Menu.svelte";
export { default as Toast } from "./Toast.svelte";
export { default as Toaster } from "./Toaster.svelte";
export { toast, toasts, type ToastKind, type ToastItem } from "./toasts.svelte";

export { default as StatusIndicator } from "./StatusIndicator.svelte";
export { default as UpdateBanner } from "./UpdateBanner.svelte";
export type { BoardItem, MenuItem, UpdaterState, UpdateBannerLabels } from "./types";

export { TILE_COLORS, DEFAULT_TILE_COLOR, resolveColor, colorForId, type TileColor } from "./palette";
export { ACCENTS, applyTheme, inkFor, prefersReducedMotion, type AccentKey, type ThemeOptions } from "./theme";
