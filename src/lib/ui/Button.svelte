<script lang="ts">
  // Кнопка. variant: primary — заливка акцентом; secondary — неоновый контур (Z43 NeonButton,
  // но без вечного блика); ghost — для панелей; danger — разрушительные действия.
  // `cut` — срезанные углы: только для «Стоп всё» и шапки (решение §1.4).
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Icon from "./icons/Icon.svelte";
  import type { IconName } from "./icons/paths";

  interface Props extends HTMLButtonAttributes {
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "sm" | "md" | "lg";
    icon?: IconName;
    iconRight?: IconName;
    cut?: boolean;
    loading?: boolean;
    /** Растянуть на всю ширину. */
    block?: boolean;
    children?: Snippet;
  }
  let {
    variant = "secondary",
    size = "md",
    icon,
    iconRight,
    cut = false,
    loading = false,
    block = false,
    disabled,
    type = "button",
    class: klass = "",
    children,
    ...rest
  }: Props = $props();

  const iconSize = $derived(size === "sm" ? 14 : size === "lg" ? 20 : 16);
</script>

<button
  {type}
  class="btn btn-{variant} btn-{size} {klass}"
  class:cut-corners={cut}
  class:is-cut={cut}
  class:w-full={block}
  disabled={disabled || loading}
  aria-busy={loading || undefined}
  {...rest}
>
  {#if loading}
    <Icon name="loader-circle" size={iconSize} class="animate-spin" />
  {:else if icon}
    <Icon name={icon} size={iconSize} />
  {/if}
  {#if children}<span class="truncate">{@render children()}</span>{/if}
  {#if iconRight}<Icon name={iconRight} size={iconSize} />{/if}
</button>

<style>
  .btn {
    --btn-color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    font-family: var(--font-body);
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
    cursor: pointer;
    user-select: none;
    transition:
      background-color var(--dur) var(--ease),
      border-color var(--dur) var(--ease),
      box-shadow var(--dur) var(--ease),
      color var(--dur) var(--ease),
      transform var(--dur-fast) var(--ease);
  }
  .btn:active:not(:disabled) {
    transform: translateY(1px);
  }
  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
  .btn-sm {
    height: 28px;
    padding: 0 var(--space-3);
    font-size: 12px;
  }
  .btn-md {
    height: 36px;
    padding: 0 var(--space-4);
    font-size: 13px;
  }
  .btn-lg {
    height: 44px;
    padding: 0 var(--space-5);
    font-size: 15px;
  }

  .btn-primary {
    background: var(--btn-color);
    color: var(--accent-ink);
    --glow-color: var(--btn-color);
  }
  .btn-primary:hover:not(:disabled) {
    box-shadow: var(--glow-md);
    filter: brightness(1.08);
  }

  .btn-secondary {
    background: color-mix(in oklab, var(--btn-color) 8%, transparent);
    border-color: color-mix(in oklab, var(--btn-color) 55%, transparent);
    color: var(--btn-color);
    --glow-color: var(--btn-color);
  }
  .btn-secondary:hover:not(:disabled) {
    background: color-mix(in oklab, var(--btn-color) 16%, transparent);
    border-color: var(--btn-color);
    box-shadow: var(--glow-sm);
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-muted);
  }
  .btn-ghost:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--text);
  }

  .btn-danger {
    --btn-color: var(--danger);
    background: color-mix(in oklab, var(--danger) 14%, transparent);
    border-color: color-mix(in oklab, var(--danger) 60%, transparent);
    color: #ff6b84;
    --glow-color: var(--danger);
  }
  .btn-danger:hover:not(:disabled) {
    background: var(--danger);
    color: #fff;
    box-shadow: var(--glow-md);
  }

  /* Срезанный угол: clip-path режет box-shadow, поэтому свечение — через drop-shadow. */
  .is-cut {
    border-radius: 0;
    border-color: transparent;
    --cut: 9px;
  }
  /* Срезанная кнопка без рамки — плотнее заливка, иначе диагональ срезает контур. */
  .is-cut.btn-secondary {
    background: color-mix(in oklab, var(--btn-color) 20%, transparent);
  }
  .is-cut.btn-danger {
    background: color-mix(in oklab, var(--danger) 24%, transparent);
  }
  .is-cut:hover:not(:disabled) {
    box-shadow: none;
  }
</style>
