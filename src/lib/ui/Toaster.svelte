<script lang="ts">
  // Контейнер тостов: правый нижний угол над нижней панелью. Ставится один раз (layout у D).
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import Toast from "./Toast.svelte";
  import { toasts, dismiss } from "./toasts.svelte";
  import { prefersReducedMotion } from "./theme";

  interface Props {
    /** Отступ снизу, px (высота нижней панели). */
    offset?: number;
  }
  let { offset = 88 }: Props = $props();
  const dur = prefersReducedMotion() ? 0 : 200;
</script>

<div class="toaster" style:bottom="{offset}px" aria-live="polite">
  {#each toasts.items as t (t.id)}
    <div animate:flip={{ duration: dur }} transition:fly={{ x: 24, duration: dur }}>
      <Toast kind={t.kind} title={t.title} text={t.text} action={t.action} ondismiss={() => dismiss(t.id)} />
    </div>
  {/each}
</div>

<style>
  .toaster {
    position: fixed;
    right: 16px;
    z-index: 50;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
    pointer-events: none;
  }
  .toaster > :global(*) {
    pointer-events: auto;
  }
</style>
