<script lang="ts">
  // Текстовое поле с подписью, подсказкой и ошибкой.
  import type { HTMLInputAttributes } from "svelte/elements";

  interface Props extends Omit<HTMLInputAttributes, "value"> {
    value?: string;
    label?: string;
    hint?: string;
    error?: string | null;
    ref?: HTMLInputElement | null;
  }
  let {
    value = $bindable(""),
    label,
    hint,
    error = null,
    ref = $bindable(null),
    class: klass = "",
    ...rest
  }: Props = $props();
  const id = $props.id();
</script>

<div class="field {klass}">
  {#if label}<label for={id}>{label}</label>{/if}
  <input
    {id}
    bind:this={ref}
    bind:value
    class:invalid={!!error}
    aria-invalid={!!error || undefined}
    aria-describedby={error || hint ? `${id}-h` : undefined}
    autocomplete="off"
    {...rest}
  />
  {#if error}
    <p id="{id}-h" class="err">{error}</p>
  {:else if hint}
    <p id="{id}-h">{hint}</p>
  {/if}
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
  }
  input {
    height: 36px;
    padding: 0 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    transition:
      border-color var(--dur) var(--ease),
      box-shadow var(--dur) var(--ease);
  }
  input::placeholder {
    color: var(--text-faint);
  }
  input:focus-visible {
    border-color: var(--accent);
    box-shadow: var(--glow-sm);
  }
  input.invalid {
    border-color: var(--danger);
  }
  p {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  p.err {
    color: #ff6b84;
  }
</style>
