<script lang="ts">
  // Один тост (можно показать и вне очереди, например в витрине).
  import Icon from "./icons/Icon.svelte";
  import type { IconName } from "./icons/paths";
  import type { ToastKind } from "./toasts.svelte";

  interface Props {
    kind?: ToastKind;
    title: string;
    text?: string;
    action?: { label: string; run: () => void };
    ondismiss?: () => void;
    closeLabel?: string;
  }
  let { kind = "info", title, text, action, ondismiss, closeLabel = "Закрыть" }: Props = $props();
  const ICON: Record<ToastKind, IconName> = {
    info: "info",
    success: "circle-check",
    warning: "triangle-alert",
    error: "circle-alert",
  };
</script>

<div class="toast k-{kind}" role={kind === "error" ? "alert" : "status"}>
  <span class="ic"><Icon name={ICON[kind]} size={18} /></span>
  <div class="txt">
    <strong>{title}</strong>
    {#if text}<p>{text}</p>{/if}
  </div>
  {#if action}
    <button type="button" class="act" onclick={action.run}>{action.label}</button>
  {/if}
  {#if ondismiss}
    <button type="button" class="x" aria-label={closeLabel} title={closeLabel} onclick={ondismiss}>
      <Icon name="x" size={14} />
    </button>
  {/if}
</div>

<style>
  .toast {
    --k: var(--accent);
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 340px;
    max-width: calc(100vw - 32px);
    padding: 12px 10px 12px 12px;
    border-radius: var(--radius-md);
    background: var(--surface-3);
    border: 1px solid var(--line-strong);
    border-left: 3px solid var(--k);
    box-shadow: var(--shadow-pop);
  }
  .k-success {
    --k: var(--ok);
  }
  .k-warning {
    --k: var(--warn);
  }
  .k-error {
    --k: var(--danger);
  }
  .ic {
    color: var(--k);
    margin-top: 1px;
  }
  .txt {
    flex: 1;
    min-width: 0;
  }
  strong {
    display: block;
    font-weight: 600;
  }
  p {
    margin: 2px 0 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .act {
    border: 0;
    background: transparent;
    color: var(--accent);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: var(--radius-xs);
  }
  .act:hover {
    background: var(--accent-soft);
  }
  .x {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: var(--radius-xs);
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
  }
  .x:hover {
    background: rgb(255 255 255 / 0.08);
    color: var(--text);
  }
</style>
