<script lang="ts">
  // Выбор цвета плитки или акцента. value — ключ палитры.
  import Icon from "./icons/Icon.svelte";
  import { TILE_COLORS, type TileColor } from "./palette";
  import { inkFor } from "./theme";

  interface Props {
    value: string;
    colors?: readonly TileColor[];
    label?: string;
    onchange?: (key: string) => void;
  }
  let { value = $bindable(), colors = TILE_COLORS, label = "Цвет", onchange }: Props = $props();
</script>

<div class="sw" role="radiogroup" aria-label={label}>
  {#each colors as c (c.key)}
    <button
      type="button"
      role="radio"
      aria-checked={c.key === value}
      aria-label={c.label}
      title={c.label}
      style:--c={c.hex}
      style:color={inkFor(c.hex)}
      onclick={() => {
        value = c.key;
        onchange?.(c.key);
      }}
    >
      {#if c.key === value}<Icon name="check" size={14} strokeWidth={3} />{/if}
    </button>
  {/each}
</div>

<style>
  .sw {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 0;
    background: var(--c);
    cursor: pointer;
    transition: transform var(--dur-fast) var(--ease);
  }
  button:hover {
    transform: scale(1.12);
  }
  button[aria-checked="true"] {
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--c);
  }
</style>
