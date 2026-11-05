<script lang="ts">
  // Плитка звука — как кнопка на пульте: свой цвет, эмодзи, название, хоткей.
  // Клик — onplay (играть/стоп решает Rust по Sound.mode). ПКМ или «⋯» — onmenu (контекстное меню).
  // Играет: рамка и свечение цветом плитки, жёлтая метка «играет», полоска прогресса снизу.
  import type { HTMLAttributes } from "svelte/elements";
  import Icon from "./icons/Icon.svelte";
  import Kbd from "./Kbd.svelte";
  import { resolveColor } from "./palette";

  interface Props extends Omit<HTMLAttributes<HTMLDivElement>, "color"> {
    name: string;
    /** Ключ палитры или #rrggbb (Sound.color). */
    color?: string;
    emoji?: string | null;
    hotkey?: string | null;
    playing?: boolean;
    /** 0..1; null при playing — неопределённый прогресс. */
    progress?: number | null;
    favorite?: boolean;
    /** Файл не найден / ошибка декодирования. */
    broken?: boolean;
    /** Выделение (мультивыбор, редактирование). */
    selected?: boolean;
    /** Длительность для подписи, мс. */
    durationMs?: number | null;
    onplay?: () => void;
    onmenu?: (e: MouseEvent) => void;
    labels?: { play?: string; stop?: string; menu?: string; broken?: string };
  }
  let {
    name,
    color = "cyan",
    emoji = null,
    hotkey = null,
    playing = false,
    progress = null,
    favorite = false,
    broken = false,
    selected = false,
    durationMs = null,
    onplay,
    onmenu,
    labels = {},
    class: klass = "",
    ...rest
  }: Props = $props();

  const hex = $derived(resolveColor(color));
  const pct = $derived(progress == null ? null : Math.max(0, Math.min(1, progress)) * 100);
  const duration = $derived(durationMs == null ? null : fmt(durationMs));

  function fmt(ms: number) {
    const s = Math.round(ms / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  }
  function contextmenu(e: MouseEvent) {
    if (!onmenu) return;
    e.preventDefault();
    onmenu(e);
  }
</script>

<div
  class="tile {klass}"
  class:playing
  class:broken
  class:selected
  style:--tile={hex}
  style:--glow-color={hex}
  oncontextmenu={contextmenu}
  {...rest}
>
  <button
    type="button"
    class="hit"
    onclick={() => onplay?.()}
    aria-pressed={playing}
    aria-label="{playing ? (labels.stop ?? 'Стоп') : (labels.play ?? 'Играть')}: {name}"
  >
    <span class="top">
      <span class="emoji" aria-hidden="true">
        {#if broken}
          <Icon name="triangle-alert" size={18} />
        {:else if emoji}
          {emoji}
        {:else}
          <Icon name="music" size={18} />
        {/if}
      </span>
      {#if playing}
        <span class="eq" aria-hidden="true"><i></i><i></i><i></i><i></i></span>
      {/if}
    </span>
    <span class="name">
      {#if favorite}<Icon name="star" size={12} class="fav" />{/if}
      {name}
    </span>
    <span class="meta">
      {#if broken}
        <span class="broken-label">{labels.broken ?? "Файл не найден"}</span>
      {:else if duration}
        <span class="dur">{duration}</span>
      {/if}
      {#if hotkey}<Kbd keys={hotkey} size="sm" class="ml-auto" />{/if}
    </span>
  </button>

  {#if onmenu}
    <button
      type="button"
      class="menu"
      aria-label="{labels.menu ?? 'Меню'}: {name}"
      title={labels.menu ?? "Меню"}
      onclick={(e) => {
        e.stopPropagation();
        onmenu?.(e);
      }}
    >
      <Icon name="ellipsis" size={16} />
    </button>
  {/if}

  {#if playing}
    <span class="progress" aria-hidden="true">
      {#if pct == null}
        <span class="bar indeterminate"></span>
      {:else}
        <span class="bar" style:width="{pct}%"></span>
      {/if}
    </span>
  {/if}
</div>

<style>
  .tile {
    position: relative;
    min-height: 96px;
    border-radius: var(--radius-tile);
    background: color-mix(in oklab, var(--tile) 9%, var(--surface-2));
    border: 1px solid color-mix(in oklab, var(--tile) 26%, transparent);
    overflow: hidden;
    transition:
      background-color var(--dur) var(--ease),
      border-color var(--dur) var(--ease),
      box-shadow var(--dur-slow) var(--ease),
      transform var(--dur-fast) var(--ease);
  }
  /* Цветная кромка сверху — «колпачок» кнопки пульта. */
  .tile::before {
    content: "";
    position: absolute;
    inset: 0 0 auto 0;
    height: 3px;
    background: var(--tile);
    opacity: 0.85;
    pointer-events: none;
  }
  .tile:hover {
    background: color-mix(in oklab, var(--tile) 15%, var(--surface-2));
    border-color: color-mix(in oklab, var(--tile) 55%, transparent);
    box-shadow: var(--glow-sm);
  }
  .tile:active:not(:has(.menu:active)) {
    transform: scale(0.985);
  }
  .tile.selected {
    border-color: var(--text);
    box-shadow: 0 0 0 1px var(--text);
  }
  .tile.playing {
    background: color-mix(in oklab, var(--tile) 22%, var(--surface-2));
    border-color: var(--tile);
    box-shadow:
      0 0 0 1px var(--tile),
      var(--glow-md);
    animation: tile-pulse 1.6s var(--ease) infinite;
  }
  :global(:root[data-effects="minimal"]) .tile.playing {
    animation: none;
  }
  @keyframes tile-pulse {
    50% {
      box-shadow:
        0 0 0 1px var(--tile),
        0 0 32px color-mix(in oklab, var(--tile) 60%, transparent);
    }
  }
  .tile.broken {
    background: var(--surface-2);
    border-style: dashed;
    border-color: color-mix(in oklab, var(--warn) 50%, transparent);
  }
  .tile.broken::before {
    background: var(--warn);
  }

  .hit {
    all: unset;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    height: 100%;
    min-height: inherit;
    padding: 12px 12px 10px;
    cursor: pointer;
    border-radius: inherit;
  }
  .hit:focus-visible {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 30px;
    padding-right: 24px; /* место под «⋯» */
  }
  .emoji {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: var(--radius-md);
    background: color-mix(in oklab, var(--tile) 18%, transparent);
    color: var(--tile);
    font-size: 18px;
    line-height: 1;
  }
  .broken .emoji {
    color: var(--warn);
    background: color-mix(in oklab, var(--warn) 14%, transparent);
  }
  .name {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    color: var(--text);
    font-weight: 600;
    font-size: 13.5px;
    line-height: 1.25;
    word-break: break-word;
  }
  .name :global(.fav) {
    display: inline;
    vertical-align: -1px;
    color: var(--yellow);
    fill: var(--yellow);
    margin-right: 2px;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: auto;
    min-height: 18px;
  }
  .dur {
    font-family: var(--font-mono);
    font-weight: 500;
    font-size: 11.5px;
    color: var(--text-faint);
  }
  .broken-label {
    font-size: 11.5px;
    color: var(--warn);
  }

  .menu {
    position: absolute;
    top: 8px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--dur) var(--ease);
  }
  .tile:hover .menu,
  .tile:focus-within .menu,
  .menu:focus-visible {
    opacity: 1;
  }
  .menu:hover {
    background: rgb(255 255 255 / 0.08);
    color: var(--text);
  }

  /* «Играет»: жёлтый эквалайзер — единственное место жёлтого на плитке, кроме звезды. */
  .eq {
    display: inline-flex;
    align-items: flex-end;
    gap: 2px;
    height: 14px;
    margin-right: 4px;
  }
  .eq i {
    width: 3px;
    height: 100%;
    border-radius: 1px;
    background: var(--yellow);
    transform-origin: bottom;
    animation: eq 0.9s ease-in-out infinite;
  }
  .eq i:nth-child(2) {
    animation-delay: -0.3s;
  }
  .eq i:nth-child(3) {
    animation-delay: -0.6s;
  }
  .eq i:nth-child(4) {
    animation-delay: -0.15s;
  }
  @keyframes eq {
    0%,
    100% {
      transform: scaleY(0.3);
    }
    50% {
      transform: scaleY(1);
    }
  }
  :global(:root[data-effects="minimal"]) .eq i {
    animation: none;
    transform: scaleY(0.7);
  }

  .progress {
    position: absolute;
    inset: auto 0 0 0;
    height: 3px;
    background: rgb(255 255 255 / 0.08);
    pointer-events: none;
  }
  .bar {
    display: block;
    height: 100%;
    background: var(--tile);
    box-shadow: 0 0 8px var(--tile);
    transition: width 90ms linear;
  }
  .bar.indeterminate {
    width: 30%;
    animation: indet 1.2s var(--ease) infinite;
  }
  @keyframes indet {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
</style>
