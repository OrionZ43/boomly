<script lang="ts">
  // Сегментный переключатель (радиогруппа): «Эффекты: мин / обычные», режим воспроизведения.
  import Icon from "./icons/Icon.svelte";
  import type { IconName } from "./icons/paths";

  interface Props {
    value: string;
    options: { value: string; label: string; icon?: IconName }[];
    label: string;
    onchange?: (value: string) => void;
    size?: "sm" | "md";
  }
  let { value = $bindable(), options, label, onchange, size = "md" }: Props = $props();

  function keydown(e: KeyboardEvent) {
    const i = options.findIndex((o) => o.value === value);
    const d = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const n = options[(i + d + options.length) % options.length];
    select(n.value);
    (e.currentTarget as HTMLElement).querySelector<HTMLElement>(`[data-v="${CSS.escape(n.value)}"]`)?.focus();
  }
  function select(v: string) {
    value = v;
    onchange?.(v);
  }
</script>

<div class="seg size-{size}" role="radiogroup" aria-label={label} tabindex="-1" onkeydown={keydown}>
  {#each options as o (o.value)}
    <button
      type="button"
      role="radio"
      data-v={o.value}
      aria-checked={o.value === value}
      tabindex={o.value === value ? 0 : -1}
      class:on={o.value === value}
      onclick={() => select(o.value)}
    >
      {#if o.icon}<Icon name={o.icon} size={14} />{/if}
      {o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border-radius: var(--radius-md);
    background: var(--surface-1);
    border: 1px solid var(--line-strong);
  }
  .seg:focus-visible {
    box-shadow: none;
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color var(--dur) var(--ease),
      color var(--dur) var(--ease);
  }
  .size-sm button {
    height: 24px;
    padding: 0 10px;
    font-size: 12px;
  }
  .size-md button {
    height: 30px;
    padding: 0 14px;
    font-size: 13px;
  }
  button:hover {
    color: var(--text);
  }
  button.on {
    background: var(--surface-3);
    color: var(--text);
    box-shadow: inset 0 0 0 1px var(--accent-line);
  }
  button.on :global(svg) {
    color: var(--accent);
  }
</style>
