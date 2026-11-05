<script lang="ts">
  // Пустое состояние: иконка (или маскот позже), заголовок, текст, действие.
  import type { Snippet } from "svelte";
  import Icon from "./icons/Icon.svelte";
  import type { IconName } from "./icons/paths";

  interface Props {
    icon?: IconName;
    title: string;
    text?: string;
    actions?: Snippet;
  }
  let { icon = "audio-lines", title, text, actions }: Props = $props();
</script>

<div class="es">
  <span class="ic"><Icon name={icon} size={28} /></span>
  <h3>{title}</h3>
  {#if text}<p>{text}</p>{/if}
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</div>

<style>
  .es {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;
    max-width: 360px;
    padding: 24px;
  }
  .ic {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: var(--radius-lg);
    background: var(--accent-soft);
    color: var(--accent);
    margin-bottom: 4px;
  }
  h3 {
    margin: 0;
    font-size: 18px;
  }
  p {
    margin: 0;
    color: var(--text-muted);
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
</style>
