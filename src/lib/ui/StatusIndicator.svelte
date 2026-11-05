<script lang="ts">
  // Индикатор состояния для нижней панели: VB-CABLE, микрофон, устройство.
  // tone: ok (зелёный), warn (оранжевый), error (красный), off (серый), live (жёлтый — «в эфире»).
  // Если передан onclick — это кнопка (например, открыть мастер установки VB-CABLE).
  import Icon from "./icons/Icon.svelte";
  import type { IconName } from "./icons/paths";

  interface Props {
    tone?: "ok" | "warn" | "error" | "off" | "live";
    icon?: IconName;
    label: string;
    /** Пояснение во всплывающей подсказке и для скринридера. */
    detail?: string;
    onclick?: () => void;
    /** Мигающая точка (например, «нет кабеля»). Гасится в спокойном режиме. */
    pulse?: boolean;
  }
  let { tone = "ok", icon, label, detail, onclick, pulse = false }: Props = $props();
</script>

<svelte:element
  this={onclick ? "button" : "span"}
  type={onclick ? "button" : undefined}
  class="si t-{tone}"
  class:clickable={!!onclick}
  role={onclick ? undefined : "status"}
  title={detail}
  {onclick}
>
  <span class="dot" class:pulse aria-hidden="true"></span>
  {#if icon}<Icon name={icon} size={15} />{/if}
  <span class="lbl">{label}</span>
  {#if detail}<span class="sr-only">— {detail}</span>{/if}
</svelte:element>

<style>
  .si {
    --t: var(--ok);
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 10px;
    border-radius: 14px;
    border: 1px solid color-mix(in oklab, var(--t) 35%, transparent);
    background: color-mix(in oklab, var(--t) 9%, transparent);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    font-weight: 500;
    white-space: nowrap;
  }
  .si :global(svg) {
    color: var(--t);
  }
  .t-warn {
    --t: var(--warn);
  }
  .t-error {
    --t: var(--danger);
  }
  .t-live {
    --t: var(--yellow);
  }
  .t-off {
    --t: var(--text-faint);
    color: var(--text-muted);
  }
  .clickable {
    cursor: pointer;
    transition: background-color var(--dur) var(--ease);
  }
  .clickable:hover {
    background: color-mix(in oklab, var(--t) 18%, transparent);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--t);
    box-shadow: 0 0 6px var(--t);
  }
  .t-off .dot {
    box-shadow: none;
  }
  .dot.pulse {
    animation: blink 1.4s ease-in-out infinite;
  }
  :global(:root[data-effects="minimal"]) .dot.pulse {
    animation: none;
  }
  @keyframes blink {
    50% {
      opacity: 0.25;
    }
  }
</style>
