<script lang="ts">
  // Контекстное/выпадающее меню. Открыть: `open = true` + координаты (clientX/Y клика или
  // нижний левый угол кнопки — см. menuAnchor()). Стрелки/Home/End — навигация, Enter — выбор,
  // Esc, клик мимо, прокрутка и потеря фокуса окном — закрыть.
  import { tick } from "svelte";
  import Icon from "./icons/Icon.svelte";
  import Kbd from "./Kbd.svelte";
  import type { MenuItem } from "./types";

  interface Props {
    open?: boolean;
    x?: number;
    y?: number;
    items: MenuItem[];
    onselect?: (id: string) => void;
    onclose?: () => void;
    label?: string;
  }
  let { open = $bindable(false), x = 0, y = 0, items, onselect, onclose, label = "Меню" }: Props = $props();

  let el: HTMLDivElement | undefined = $state();
  let pos = $state({ left: 0, top: 0 });
  let returnFocus: Element | null = null;

  $effect(() => {
    if (!open) return;
    returnFocus = document.activeElement;
    pos = { left: x, top: y };
    tick().then(() => {
      if (!el) return;
      const r = el.getBoundingClientRect();
      pos = {
        left: Math.max(8, Math.min(x, innerWidth - r.width - 8)),
        top: y + r.height > innerHeight - 8 ? Math.max(8, y - r.height) : y,
      };
      focusables()[0]?.focus();
    });
    const outside = (e: PointerEvent) => {
      if (el && !el.contains(e.target as Node)) close();
    };
    const onscroll = () => close();
    window.addEventListener("pointerdown", outside, true);
    window.addEventListener("blur", onscroll);
    window.addEventListener("resize", onscroll);
    window.addEventListener("wheel", onscroll, { passive: true });
    return () => {
      window.removeEventListener("pointerdown", outside, true);
      window.removeEventListener("blur", onscroll);
      window.removeEventListener("resize", onscroll);
      window.removeEventListener("wheel", onscroll);
    };
  });

  function focusables() {
    return Array.from(el?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []);
  }
  function close(restore = true) {
    if (!open) return;
    open = false;
    onclose?.();
    if (restore && returnFocus instanceof HTMLElement) returnFocus.focus();
  }
  function keydown(e: KeyboardEvent) {
    const list = focusables();
    const i = list.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "ArrowDown") list[(i + 1) % list.length]?.focus();
    else if (e.key === "ArrowUp") list[(i - 1 + list.length) % list.length]?.focus();
    else if (e.key === "Home") list[0]?.focus();
    else if (e.key === "End") list[list.length - 1]?.focus();
    else if (e.key === "Escape" || e.key === "Tab") close();
    else return;
    e.preventDefault();
  }
  function pick(id: string) {
    close();
    onselect?.(id);
  }

</script>

<script lang="ts" module>
  /** Координаты для меню под кнопкой: `const { x, y } = menuAnchor(e.currentTarget)`. */
  export function menuAnchor(target: EventTarget | null, align: "start" | "end" = "start") {
    const r = (target as HTMLElement).getBoundingClientRect();
    return { x: align === "end" ? r.right - 200 : r.left, y: r.bottom + 4 };
  }
</script>

{#if open}
  <div
    bind:this={el}
    class="menu"
    role="menu"
    tabindex="-1"
    aria-label={label}
    style:left="{pos.left}px"
    style:top="{pos.top}px"
    onkeydown={keydown}
    oncontextmenu={(e) => e.preventDefault()}
  >
    {#each items as item, i (i)}
      {#if "separator" in item}
        <div class="sep" role="separator"></div>
      {:else}
        <button
          type="button"
          role={item.checked === undefined ? "menuitem" : "menuitemradio"}
          aria-checked={item.checked}
          class="mi"
          class:danger={item.danger}
          disabled={item.disabled}
          onclick={() => pick(item.id)}
        >
          <span class="ic">
            {#if item.checked}
              <Icon name="check" size={15} />
            {:else if item.icon}
              <Icon name={item.icon} size={15} />
            {/if}
          </span>
          <span class="lbl">{item.label}</span>
          {#if item.shortcut}<Kbd keys={item.shortcut} size="sm" />{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 60;
    min-width: 200px;
    max-width: 280px;
    padding: 5px;
    border-radius: var(--radius-md);
    background: var(--surface-3);
    border: 1px solid var(--line-strong);
    box-shadow: var(--shadow-pop);
    animation: menu-in var(--dur) var(--ease);
    transform-origin: top left;
  }
  .menu:focus-visible {
    box-shadow: var(--shadow-pop);
  }
  @keyframes menu-in {
    from {
      opacity: 0;
      transform: scale(0.97);
    }
  }
  .mi {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 32px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .mi:hover:not(:disabled),
  .mi:focus-visible {
    background: var(--accent-soft);
    box-shadow: none;
  }
  .mi:disabled {
    color: var(--text-faint);
    cursor: default;
  }
  .ic {
    display: grid;
    place-items: center;
    width: 18px;
    color: var(--text-muted);
  }
  .mi[aria-checked="true"] .ic {
    color: var(--accent);
  }
  .lbl {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .danger,
  .danger .ic {
    color: #ff6b84;
  }
  .danger:hover:not(:disabled),
  .danger:focus-visible {
    background: color-mix(in oklab, var(--danger) 16%, transparent);
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--line-strong);
  }
</style>
