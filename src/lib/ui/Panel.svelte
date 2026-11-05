<script lang="ts">
  // Поверхность: сайдбар, нижняя панель, шапка, карточка настроек.
  // `cut` — срезанные углы Z43 (по решению — только шапка); `glass` — стекло с blur (только мелкие панели).
  import type { Snippet } from "svelte";
  import type { HTMLAttributes } from "svelte/elements";

  interface Props extends HTMLAttributes<HTMLElement> {
    as?: "div" | "section" | "header" | "footer" | "aside" | "nav";
    level?: 1 | 2;
    cut?: boolean;
    glass?: boolean;
    /** Светящаяся линия бренда вдоль верхней кромки (для шапки). */
    accentEdge?: boolean;
    padding?: "none" | "sm" | "md";
    children?: Snippet;
  }
  let {
    as = "div",
    level = 1,
    cut = false,
    glass = false,
    accentEdge = false,
    padding = "md",
    class: klass = "",
    children,
    ...rest
  }: Props = $props();
</script>

<svelte:element
  this={as}
  class="panel l{level} p-{padding} {klass}"
  class:cut-corners={cut}
  class:is-cut={cut}
  class:is-glass={glass}
  class:edge={accentEdge}
  {...rest}
>
  {@render children?.()}
</svelte:element>

<style>
  .panel {
    position: relative;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line);
  }
  .l1 {
    background: var(--surface-1);
  }
  .l2 {
    background: var(--surface-2);
  }
  .is-glass {
    background: var(--glass);
    backdrop-filter: blur(12px);
    border-color: var(--line-strong);
  }
  .p-sm {
    padding: var(--space-3);
  }
  .p-md {
    padding: var(--space-4);
  }
  .is-cut {
    --cut: var(--cut-lg);
    border-radius: 0;
    border: 0;
  }
  /* clip-path срезает border, поэтому кромку для срезанной панели рисуем градиентом сверху. */
  .edge::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent) 20%, var(--magenta) 80%, transparent);
    opacity: 0.7;
    pointer-events: none;
  }
</style>
