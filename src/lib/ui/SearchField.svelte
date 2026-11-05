<script lang="ts">
  // Поле поиска: иконка, крестик очистки, подсказка хоткея (Ctrl+F — вешает D). Esc очищает.
  import type { HTMLInputAttributes } from "svelte/elements";
  import Icon from "./icons/Icon.svelte";
  import Kbd from "./Kbd.svelte";

  interface Props extends Omit<HTMLInputAttributes, "value"> {
    value?: string;
    placeholder?: string;
    shortcut?: string | null;
    clearLabel?: string;
    ref?: HTMLInputElement | null;
  }
  let {
    value = $bindable(""),
    placeholder = "Поиск звуков",
    shortcut = "Ctrl+F",
    clearLabel = "Очистить",
    ref = $bindable(null),
    class: klass = "",
    ...rest
  }: Props = $props();
</script>

<div class="sf {klass}">
  <Icon name="search" size={16} class="lead" />
  <input
    bind:this={ref}
    bind:value
    type="search"
    {placeholder}
    aria-label={placeholder}
    autocomplete="off"
    spellcheck="false"
    onkeydown={(e) => {
      if (e.key === "Escape" && value) {
        e.stopPropagation();
        value = "";
      }
    }}
    {...rest}
  />
  {#if value}
    <button
      type="button"
      class="clear"
      aria-label={clearLabel}
      title={clearLabel}
      onclick={() => {
        value = "";
        ref?.focus();
      }}
    >
      <Icon name="x" size={14} />
    </button>
  {:else if shortcut}
    <Kbd keys={shortcut} size="sm" class="hint" />
  {/if}
</div>

<style>
  .sf {
    position: relative;
    display: flex;
    align-items: center;
    height: 36px;
    padding: 0 8px 0 10px;
    gap: 8px;
    border-radius: var(--radius-md);
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    color: var(--text-faint);
    transition:
      border-color var(--dur) var(--ease),
      box-shadow var(--dur) var(--ease);
  }
  .sf:focus-within {
    border-color: var(--accent);
    color: var(--accent);
    --glow-color: var(--accent);
    box-shadow: var(--glow-sm);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
  }
  input:focus-visible {
    box-shadow: none;
  }
  input::placeholder {
    color: var(--text-faint);
  }
  input::-webkit-search-cancel-button {
    display: none;
  }
  .clear {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: var(--radius-xs);
    background: rgb(255 255 255 / 0.08);
    color: var(--text-muted);
    cursor: pointer;
  }
  .clear:hover {
    color: var(--text);
    background: rgb(255 255 255 / 0.14);
  }
</style>
