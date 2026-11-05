<script lang="ts">
  // Модальное окно на нативном <dialog>: фокус-ловушка, Esc и клик по фону закрывают.
  // Единственное место с blur (стекло Z43) — на большой площади блюр грузит GPU рядом с игрой.
  import type { Snippet } from "svelte";
  import IconButton from "./IconButton.svelte";

  interface Props {
    open?: boolean;
    title: string;
    description?: string;
    size?: "sm" | "md" | "lg";
    /** Закрывать по клику на фон и Esc. */
    dismissible?: boolean;
    onclose?: () => void;
    closeLabel?: string;
    children?: Snippet;
    footer?: Snippet;
  }
  let {
    open = $bindable(false),
    title,
    description,
    size = "md",
    dismissible = true,
    onclose,
    closeLabel = "Закрыть",
    children,
    footer,
  }: Props = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  const id = $props.id();

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });

  function close() {
    open = false;
    onclose?.();
  }
</script>

<dialog
  bind:this={dialog}
  class="modal size-{size}"
  aria-labelledby="{id}-t"
  aria-describedby={description ? `${id}-d` : undefined}
  oncancel={(e) => {
    e.preventDefault();
    if (dismissible) close();
  }}
  onclick={(e) => {
    if (dismissible && e.target === dialog) close();
  }}
>
  {#if open}
    <div class="panel">
      <header>
        <div>
          <h2 id="{id}-t">{title}</h2>
          {#if description}<p id="{id}-d">{description}</p>{/if}
        </div>
        {#if dismissible}
          <IconButton icon="x" label={closeLabel} size="sm" onclick={close} />
        {/if}
      </header>
      {#if children}<div class="body">{@render children()}</div>{/if}
      {#if footer}<footer>{@render footer()}</footer>{/if}
    </div>
  {/if}
</dialog>

<style>
  .modal {
    margin: auto; /* preflight Tailwind обнуляет margin, а центрирует <dialog> именно он */
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    max-height: calc(100vh - 48px);
    overflow: visible;
  }
  .size-sm {
    width: min(380px, calc(100vw - 32px));
  }
  .size-md {
    width: min(520px, calc(100vw - 32px));
  }
  .size-lg {
    width: min(720px, calc(100vw - 32px));
  }
  .modal::backdrop {
    background: rgb(3 3 6 / 0.72);
  }
  .modal[open] {
    animation: pop var(--dur-slow) var(--ease);
  }
  .modal[open]::backdrop {
    animation: fade var(--dur-slow) var(--ease);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  .panel {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 48px);
    background: var(--glass);
    backdrop-filter: blur(12px);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
  }
  /* Тонкая градиентная кромка бренда сверху. */
  .panel::before {
    content: "";
    height: 2px;
    flex-shrink: 0;
    background: var(--gradient-brand);
    opacity: 0.8;
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 18px 8px 22px;
  }
  h2 {
    margin: 0;
    font-size: 19px;
  }
  header p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }
  .body {
    padding: 8px 22px 18px;
    overflow: auto;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 22px;
    border-top: 1px solid var(--line);
    background: rgb(0 0 0 / 0.2);
  }
</style>
