<script lang="ts">
  // «О программе» (кнопка ⓘ в шапке): логотип, версия, ссылки Z43 — как в Protogenix.
  import { INFO_LINKS } from "$lib/config/links";
  import { t } from "$lib/i18n";
  import { app } from "$lib/stores/app.svelte";
  import { BoomlyLogo, Button, Modal, type IconName } from "$lib/ui";
  import { platform } from "$lib/api";

  interface Props {
    open: boolean;
  }
  let { open = $bindable() }: Props = $props();

  const ICON: Record<string, IconName> = { site: "globe", github: "github", telegram: "send" };
</script>

<Modal bind:open title={t("about.title")} size="sm" closeLabel={t("common.close")}>
  <div class="ab">
    <BoomlyLogo size={84} badge wordmark={false} />
    <BoomlyLogo size={30} wordmark class="-mt-1" />
    <p class="ver">{t("about.version", { version: app.info?.version ?? "—" })}</p>
    <p class="tag">{t("about.tagline")}</p>
    <div class="links">
      {#each INFO_LINKS as l (l.id)}
        <Button variant="secondary" icon={ICON[l.id]} onclick={() => platform.openExternal(l.url)}>{t(`about.${l.id}`)}</Button>
      {/each}
    </div>
    <p class="sig">{t("about.signature")}</p>
    <p class="lic">{t("about.license")}</p>
  </div>
</Modal>

<style>
  .ab {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 10px;
    padding: 4px 0 6px;
  }
  .ab :global(.logo:nth-child(2) svg) {
    display: none;
  }
  p {
    margin: 0;
  }
  .ver {
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--text-muted);
  }
  .tag {
    color: var(--text-muted);
    max-width: 300px;
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin: 8px 0 6px;
  }
  .sig {
    font-family: var(--font-display);
    font-size: 12px;
    letter-spacing: 0.12em;
    color: var(--text-faint);
  }
  .lic {
    font-size: 11.5px;
    color: var(--text-faint);
  }
</style>
