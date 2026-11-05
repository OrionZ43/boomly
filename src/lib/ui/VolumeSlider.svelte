<script lang="ts">
  // Слайдер громкости: подпись, иконка-mute, дорожка, процент (Chakra Petch).
  // value bindable. max: 1 для каналов, 2 для громкости звука. `level` (0..1) — живой уровень
  // (audio://mic-level) поверх дорожки. Колесо мыши и стрелки — ±5%.
  import Icon from "./icons/Icon.svelte";
  import type { IconName } from "./icons/paths";

  interface Props {
    label: string;
    value: number;
    max?: number;
    step?: number;
    icon?: IconName;
    muted?: boolean;
    /** Есть кнопка mute — передайте колбэк. */
    onmute?: () => void;
    onchange?: (value: number) => void;
    level?: number | null;
    disabled?: boolean;
    compact?: boolean;
    muteLabel?: string;
    unmuteLabel?: string;
  }
  let {
    label,
    value = $bindable(1),
    max = 1,
    step = 0.01,
    icon = "volume-2",
    muted = false,
    onmute,
    onchange,
    level = null,
    disabled = false,
    compact = false,
    muteLabel = "Выключить звук",
    unmuteLabel = "Включить звук",
  }: Props = $props();

  const id = $props.id();
  const fill = $derived(Math.max(0, Math.min(1, value / max)) * 100);
  const pct = $derived(Math.round(value * 100));
  const shownIcon = $derived<IconName>(
    muted || value === 0 ? (icon === "mic" ? "mic-off" : "volume-x") : icon === "volume-2" && value < max * 0.4 ? "volume-1" : icon,
  );

  function set(v: number) {
    value = Math.round(Math.max(0, Math.min(max, v)) / step) * step;
    onchange?.(value);
  }
  function wheel(e: WheelEvent) {
    if (disabled) return;
    e.preventDefault();
    set(value + (e.deltaY < 0 ? 0.05 : -0.05) * max);
  }
</script>

<div class="vs" class:muted class:disabled class:compact>
  {#if onmute}
    <button
      type="button"
      class="mute"
      onclick={onmute}
      aria-pressed={muted}
      aria-label={muted ? unmuteLabel : muteLabel}
      title={muted ? unmuteLabel : muteLabel}
      {disabled}
    >
      <Icon name={shownIcon} size={18} />
    </button>
  {:else}
    <span class="mute static" aria-hidden="true"><Icon name={shownIcon} size={18} /></span>
  {/if}
  <div class="body">
    <div class="row">
      <label for={id}>{label}</label>
      <output for={id} class="pct">{pct}%</output>
    </div>
    <div class="track-wrap" style:--fill="{fill}%" style:--level="{(level ?? 0) * 100}%">
      {#if level != null}<span class="level" aria-hidden="true"></span>{/if}
      <input
        {id}
        type="range"
        min="0"
        {max}
        {step}
        {value}
        {disabled}
        oninput={(e) => set(+e.currentTarget.value)}
        onwheel={wheel}
        aria-valuetext="{pct}%"
      />
    </div>
  </div>
</div>

<style>
  .vs {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 180px;
  }
  .mute {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }
  .mute:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--text);
  }
  .mute.static {
    cursor: default;
  }
  .muted .mute {
    color: var(--danger);
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 4px;
  }
  label {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-muted);
  }
  .pct {
    font-family: var(--font-mono);
    font-weight: 600;
    font-size: 12.5px;
    color: var(--text);
  }
  .compact .row {
    margin-bottom: 0;
  }
  .track-wrap {
    position: relative;
    height: 18px;
    display: flex;
    align-items: center;
  }
  .level {
    position: absolute;
    left: 0;
    top: calc(50% + 5px);
    height: 2px;
    width: var(--level);
    border-radius: 2px;
    background: color-mix(in oklab, var(--ok) 70%, transparent);
    pointer-events: none;
    transition: width 60ms linear;
    z-index: 1;
  }
  input[type="range"] {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 18px;
    margin: 0;
    background: transparent;
    cursor: pointer;
  }
  input[type="range"]:focus-visible {
    box-shadow: none;
  }
  input[type="range"]::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 2px;
    background: linear-gradient(to right, var(--accent) var(--fill), rgb(255 255 255 / 0.12) var(--fill));
  }
  input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    margin-top: -5px;
    border-radius: 50%;
    background: #fff;
    border: 0;
    box-shadow:
      0 0 0 3px color-mix(in oklab, var(--accent) 35%, transparent),
      0 1px 3px rgb(0 0 0 / 0.5);
    transition: transform var(--dur-fast) var(--ease);
  }
  input[type="range"]:hover::-webkit-slider-thumb {
    transform: scale(1.15);
  }
  input[type="range"]:focus-visible::-webkit-slider-thumb {
    box-shadow: var(--focus-ring);
  }
  .muted input[type="range"]::-webkit-slider-runnable-track {
    background: linear-gradient(to right, var(--text-faint) var(--fill), rgb(255 255 255 / 0.12) var(--fill));
  }
  .disabled {
    opacity: 0.45;
  }
  .disabled input {
    cursor: not-allowed;
  }
</style>
