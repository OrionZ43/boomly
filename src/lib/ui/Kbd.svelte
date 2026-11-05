<script lang="ts">
  // Сочетание клавиш: `Ctrl+Shift+F1` → [Ctrl][Shift][F1]. Шрифт Chakra Petch (латиница/цифры).
  interface Props {
    keys: string;
    size?: "sm" | "md";
    class?: string;
  }
  let { keys, size = "md", class: klass = "" }: Props = $props();
  const parts = $derived(keys.split("+").filter(Boolean));
</script>

<span class="kbd-group size-{size} {klass}" aria-label={keys}>
  {#each parts as k, i (i)}
    <kbd>{k}</kbd>
  {/each}
</span>

<style>
  .kbd-group {
    display: inline-flex;
    gap: 2px;
    flex-shrink: 0;
  }
  kbd {
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--text-muted);
    background: rgb(255 255 255 / 0.06);
    border: 1px solid var(--line-strong);
    border-bottom-width: 2px;
    border-radius: var(--radius-xs);
    line-height: 1;
    letter-spacing: 0.02em;
  }
  .size-sm kbd {
    font-size: 10.5px;
    padding: 2px 4px 1px;
  }
  .size-md kbd {
    font-size: 12px;
    padding: 3px 6px 2px;
  }
</style>
