<script lang="ts">
  // Список досок для левой колонки. «Избранное» и «Все звуки» — обычные элементы с `icon`
  // (виртуальные доски делает D). Перестановку перетаскиванием подключает D через `itemAttrs`.
  import type { HTMLAttributes } from "svelte/elements";
  import Icon from "./icons/Icon.svelte";

  import type { BoardItem } from "./types";
  interface Props {
    items: BoardItem[];
    activeId?: string | null;
    onselect?: (id: string) => void;
    onmenu?: (id: string, e: MouseEvent) => void;
    onadd?: () => void;
    addLabel?: string;
    label?: string;
    itemAttrs?: (item: BoardItem) => HTMLAttributes<HTMLLIElement>;
  }
  let {
    items,
    activeId = null,
    onselect,
    onmenu,
    onadd,
    addLabel = "Новая доска",
    label = "Доски",
    itemAttrs,
  }: Props = $props();
</script>

<nav aria-label={label} class="boards">
  <ul>
    {#each items as item (item.id)}
      <li {...itemAttrs?.(item)}>
        <button
          type="button"
          class="item"
          class:active={item.id === activeId}
          aria-current={item.id === activeId ? "page" : undefined}
          onclick={() => onselect?.(item.id)}
          oncontextmenu={(e) => {
            if (!onmenu) return;
            e.preventDefault();
            onmenu(item.id, e);
          }}
        >
          <span class="ic" class:fav={item.tone === "favorite"} aria-hidden="true">
            {#if item.icon}
              <Icon name={item.icon} size={16} />
            {:else if item.emoji}
              {item.emoji}
            {:else}
              <Icon name="layout-grid" size={16} />
            {/if}
          </span>
          <span class="name">{item.name}</span>
          {#if item.playing}
            <span class="dot" title="▶" aria-hidden="true"></span>
          {/if}
          {#if item.count != null}<span class="count">{item.count}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
  {#if onadd}
    <button type="button" class="item add" onclick={onadd}>
      <span class="ic" aria-hidden="true"><Icon name="plus" size={16} /></span>
      <span class="name">{addLabel}</span>
    </button>
  {/if}
</nav>

<style>
  .boards ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 38px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition:
      background-color var(--dur) var(--ease),
      color var(--dur) var(--ease);
  }
  .item:hover {
    background: var(--surface-3);
    color: var(--text);
  }
  .item.active {
    background: var(--accent-soft);
    color: var(--text);
  }
  /* Активная доска — акцентная полоска слева (вместо вертикальных панелей Z43). */
  .item.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: var(--glow-sm);
  }
  .ic {
    display: grid;
    place-items: center;
    width: 20px;
    font-size: 16px;
    line-height: 1;
  }
  .active .ic {
    color: var(--accent);
  }
  .ic.fav {
    color: var(--yellow);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-faint);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--yellow);
    box-shadow: 0 0 6px var(--yellow);
  }
  .add {
    margin-top: 6px;
    color: var(--text-faint);
  }
</style>
