<script lang="ts">
  // Плитка «+ Добавить звуки»: последняя в сетке. Сюда же можно бросать файлы (drop — у D).
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Icon from "./icons/Icon.svelte";

  interface Props extends HTMLButtonAttributes {
    label?: string;
    hint?: string;
    /** Над плиткой тащат файлы. */
    dragover?: boolean;
  }
  let { label = "Добавить звуки", hint = "или перетащите файлы", dragover = false, class: klass = "", ...rest }: Props =
    $props();
</script>

<button type="button" class="add {klass}" class:dragover {...rest}>
  <Icon name="plus" size={22} />
  <span class="label">{label}</span>
  {#if hint}<span class="hint">{hint}</span>{/if}
</button>

<style>
  .add {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-height: 96px;
    padding: 12px;
    border-radius: var(--radius-tile);
    border: 1.5px dashed var(--line-strong);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition:
      border-color var(--dur) var(--ease),
      color var(--dur) var(--ease),
      background-color var(--dur) var(--ease);
  }
  .add:hover,
  .add.dragover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-soft);
  }
  .label {
    font-weight: 600;
    font-size: 13px;
  }
  .hint {
    font-size: 11.5px;
    color: var(--text-faint);
  }
</style>
