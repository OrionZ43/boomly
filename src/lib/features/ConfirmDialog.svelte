<script lang="ts">
  // Подтверждение разрушительного действия.
  import { t } from "$lib/i18n";
  import { Button, Modal } from "$lib/ui";

  interface Props {
    open: boolean;
    title: string;
    text?: string;
    confirmLabel?: string;
    onconfirm: () => void | Promise<void>;
  }
  let { open = $bindable(), title, text, confirmLabel, onconfirm }: Props = $props();
  let busy = $state(false);
</script>

<Modal bind:open {title} description={text} size="sm" closeLabel={t("common.close")}>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (open = false)}>{t("common.cancel")}</Button>
    <Button
      variant="danger"
      icon="trash-2"
      loading={busy}
      onclick={async () => {
        busy = true;
        await onconfirm();
        busy = false;
        open = false;
      }}>{confirmLabel ?? t("common.delete")}</Button
    >
  {/snippet}
</Modal>
