<script lang="ts">
  // Переключатель (role="switch"). checked bindable. Подпись и описание — справа или слева.
  interface Props {
    checked?: boolean;
    label: string;
    description?: string;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
    /** Скрыть подпись визуально (останется для скринридера). */
    hideLabel?: boolean;
  }
  let { checked = $bindable(false), label, description, disabled = false, onchange, hideLabel = false }: Props =
    $props();
  const id = $props.id();
</script>

<div class="sw-row" class:disabled>
  {#if label}
    <div class="text" class:sr-only={hideLabel}>
      <label for={id}>{label}</label>
      {#if description}<p id="{id}-d">{description}</p>{/if}
    </div>
  {/if}
  <button
    {id}
    type="button"
    role="switch"
    aria-label={label}
    aria-checked={checked}
    aria-describedby={description ? `${id}-d` : undefined}
    class="sw"
    class:on={checked}
    {disabled}
    onclick={() => {
      checked = !checked;
      onchange?.(checked);
    }}
  >
    <span class="knob"></span>
  </button>
</div>

<style>
  .sw-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .text label {
    font-weight: 500;
    color: var(--text);
    cursor: pointer;
  }
  .text p {
    margin: 2px 0 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .sw {
    position: relative;
    flex-shrink: 0;
    width: 40px;
    height: 22px;
    border-radius: 11px;
    border: 1px solid var(--line-strong);
    background: var(--surface-3);
    cursor: pointer;
    transition:
      background-color var(--dur) var(--ease),
      border-color var(--dur) var(--ease),
      box-shadow var(--dur) var(--ease);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-muted);
    transition:
      transform var(--dur) var(--ease),
      background-color var(--dur) var(--ease);
  }
  .sw.on {
    background: var(--accent);
    border-color: var(--accent);
    --glow-color: var(--accent);
    box-shadow: var(--glow-sm);
  }
  .sw.on .knob {
    transform: translateX(18px);
    background: var(--accent-ink);
  }
  .disabled {
    opacity: 0.45;
  }
  .disabled .sw,
  .disabled label {
    cursor: not-allowed;
  }
</style>
