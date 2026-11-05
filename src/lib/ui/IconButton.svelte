<script lang="ts">
  // Квадратная кнопка с одной иконкой. `label` обязателен: это aria-label и подсказка.
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Icon from "./icons/Icon.svelte";
  import type { IconName } from "./icons/paths";

  interface Props extends HTMLButtonAttributes {
    icon: IconName;
    label: string;
    size?: "sm" | "md" | "lg";
    /** Включённое состояние (toggle): подсвечивается акцентом. */
    active?: boolean;
    tone?: "default" | "danger" | "playing";
  }
  let {
    icon,
    label,
    size = "md",
    active = false,
    tone = "default",
    type = "button",
    class: klass = "",
    ...rest
  }: Props = $props();
  const px = $derived(size === "sm" ? 28 : size === "lg" ? 44 : 36);
  const iconPx = $derived(size === "sm" ? 15 : size === "lg" ? 22 : 18);
</script>

<button
  {type}
  class="ib tone-{tone} {klass}"
  class:active
  style:width="{px}px"
  style:height="{px}px"
  aria-label={label}
  title={label}
  aria-pressed={active || undefined}
  {...rest}
>
  <Icon name={icon} size={iconPx} />
</button>

<style>
  .ib {
    display: inline-grid;
    place-items: center;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition:
      background-color var(--dur) var(--ease),
      color var(--dur) var(--ease),
      border-color var(--dur) var(--ease);
  }
  .ib:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--text);
  }
  .ib:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .ib.active {
    color: var(--accent);
    background: var(--accent-soft);
    border-color: var(--accent-line);
  }
  .tone-danger.active,
  .tone-danger:hover:not(:disabled) {
    color: #ff6b84;
    background: color-mix(in oklab, var(--danger) 14%, transparent);
    border-color: color-mix(in oklab, var(--danger) 45%, transparent);
  }
  .tone-playing.active {
    color: var(--yellow);
    background: color-mix(in oklab, var(--yellow) 12%, transparent);
    border-color: color-mix(in oklab, var(--yellow) 40%, transparent);
  }
</style>
