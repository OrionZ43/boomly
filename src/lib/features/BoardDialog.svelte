<script lang="ts">
  // Создание и переименование доски: название + значок.
  import { t } from "$lib/i18n";
  import { app } from "$lib/stores/app.svelte";
  import { Button, Modal, TextField } from "$lib/ui";
  import EmojiPicker from "./EmojiPicker.svelte";

  interface Props {
    open: boolean;
    /** null — новая доска. */
    boardId: string | null;
  }
  let { open = $bindable(), boardId }: Props = $props();

  let name = $state("");
  let emoji = $state<string | null>(null);
  let input: HTMLInputElement | null = $state(null);

  $effect(() => {
    if (!open) return;
    const b = boardId ? app.library.boards.find((x) => x.id === boardId) : null;
    name = b?.name ?? "";
    emoji = b?.emoji ?? null;
    setTimeout(() => input?.focus(), 30);
  });

  async function save(e?: Event) {
    e?.preventDefault();
    const n = name.trim();
    if (!n) return;
    if (boardId) await app.updateBoard(boardId, n, emoji);
    else await app.createBoard(n, emoji);
    open = false;
  }
</script>

<Modal bind:open title={boardId ? t("boards.editTitle") : t("boards.newTitle")} size="sm" closeLabel={t("common.close")}>
  <form class="flex flex-col gap-4" onsubmit={save}>
    <TextField label={t("boards.name")} placeholder={t("boards.namePlaceholder")} bind:value={name} bind:ref={input} maxlength={60} />
    <div class="flex flex-col gap-2">
      <span class="font-medium">{t("boards.emoji")}</span>
      <EmojiPicker bind:value={emoji} label={t("boards.emoji")} />
    </div>
  </form>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (open = false)}>{t("common.cancel")}</Button>
    <Button variant="primary" disabled={!name.trim()} onclick={() => save()}>{t("common.save")}</Button>
  {/snippet}
</Modal>
