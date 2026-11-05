<script lang="ts">
  // Адаптивная сетка плиток. Внутрь кладутся SoundTile / AddTile (D решает, что и в каком порядке).
  // `empty` показывается, если children нет или `isEmpty`.
  import type { Snippet } from "svelte";

  interface Props {
    /** Минимальная ширина плитки, px. */
    minTile?: number;
    gap?: number;
    isEmpty?: boolean;
    empty?: Snippet;
    children?: Snippet;
    class?: string;
    label?: string;
  }
  let { minTile = 156, gap = 12, isEmpty = false, empty, children, class: klass = "", label }: Props = $props();
</script>

{#if isEmpty && empty}
  <div class="empty {klass}">{@render empty()}</div>
{:else}
  <div
    class="grid-tiles {klass}"
    role="group"
    aria-label={label}
    style:--min-tile="{minTile}px"
    style:gap="{gap}px"
  >
    {@render children?.()}
  </div>
{/if}

<style>
  .grid-tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--min-tile), 1fr));
    align-content: start;
  }
  .empty {
    display: grid;
    place-items: center;
    min-height: 240px;
  }
</style>
