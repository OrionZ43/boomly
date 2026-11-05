<script lang="ts">
  // Выпадающий список на нативном <select> (клавиатура и скринридер — бесплатно).
  import Icon from "./icons/Icon.svelte";

  interface Props {
    value: string;
    options: { value: string; label: string; disabled?: boolean }[];
    label?: string;
    hint?: string;
    disabled?: boolean;
    onchange?: (value: string) => void;
    class?: string;
  }
  let { value = $bindable(), options, label, hint, disabled = false, onchange, class: klass = "" }: Props = $props();
  const id = $props.id();
</script>

<div class="field {klass}">
  {#if label}<label for={id}>{label}</label>{/if}
  <div class="wrap">
    <select
      {id}
      bind:value
      {disabled}
      aria-describedby={hint ? `${id}-h` : undefined}
      onchange={() => onchange?.(value)}
    >
      {#each options as o (o.value)}
        <option value={o.value} disabled={o.disabled}>{o.label}</option>
      {/each}
    </select>
    <Icon name="chevron-down" size={16} class="chev" />
  </div>
  {#if hint}<p id="{id}-h">{hint}</p>{/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  label {
    font-weight: 500;
    color: var(--text);
  }
  p {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .wrap {
    position: relative;
  }
  select {
    appearance: none;
    width: 100%;
    height: 36px;
    padding: 0 34px 0 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    cursor: pointer;
    text-overflow: ellipsis;
  }
  select:hover:not(:disabled) {
    border-color: color-mix(in oklab, var(--accent) 45%, var(--line-strong));
  }
  select:focus-visible {
    border-color: var(--accent);
  }
  select:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  option {
    background: var(--surface-3);
    color: var(--text);
  }
  .wrap :global(.chev) {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
  }
</style>
