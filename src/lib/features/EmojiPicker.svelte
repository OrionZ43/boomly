<script lang="ts">
  // Значок плитки/доски: подборка популярных эмодзи + своё значение + «без значка».
  import { t } from "$lib/i18n";
  import { Icon } from "$lib/ui";

  interface Props {
    value: string | null;
    onchange?: (v: string | null) => void;
    label?: string;
  }
  let { value = $bindable(), onchange, label }: Props = $props();

  const SET = [
    "🔥", "💥", "🎉", "👏", "😂", "🤣", "😱", "😐", "💀", "🤡", "👀", "🙏",
    "🥁", "🎺", "🎸", "🎹", "🎵", "🔔", "📢", "🚨", "⚡", "💣", "🚀", "🎮",
    "🐸", "🐱", "🐶", "🦗", "🐐", "🦆", "🍕", "❤️", "⭐", "✅", "❌", "❓",
  ];
  function set(v: string | null) {
    value = v;
    onchange?.(v);
  }
  let custom = $state("");
</script>

<div class="ep" role="radiogroup" aria-label={label ?? t("editor.emoji")}>
  <button type="button" role="radio" aria-checked={value === null} class="cell none" title={t("common.none")} onclick={() => set(null)}>
    <Icon name="x" size={14} />
  </button>
  {#each SET as e (e)}
    <button type="button" role="radio" aria-checked={value === e} class="cell" onclick={() => set(e)}>{e}</button>
  {/each}
  <input
    class="custom"
    maxlength="8"
    placeholder="✏️"
    aria-label={label ?? t("editor.emoji")}
    bind:value={custom}
    oninput={() => {
      const v = [...custom.trim()].slice(0, 2).join("");
      if (v) set(v);
    }}
  />
</div>

<style>
  .ep {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .cell {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: var(--surface-2);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    color: var(--text-muted);
  }
  .cell:hover {
    background: var(--surface-3);
  }
  .cell[aria-checked="true"] {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .custom {
    width: 72px;
    height: 34px;
    padding: 0 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: 16px;
  }
</style>
