<script lang="ts">
  // Баннер обновления, подключённый к стору участка E ($lib/updater). Скрыт, если
  // updater_enabled=false (status "disabled") или показывать нечего (updater.bannerVisible).
  import { t } from "$lib/i18n";
  import { UpdateBanner, type UpdaterState } from "$lib/ui";
  import { updater } from "$lib/updater";
  import { errorText } from "./errors";

  const bannerState = $derived<UpdaterState>(
    !updater.bannerVisible
      ? "idle"
      : updater.status === "available" ||
          updater.status === "downloading" ||
          updater.status === "installing" ||
          updater.status === "ready" ||
          updater.status === "error"
        ? updater.status
        : "idle",
  );
  const labels = $derived({
    available: t("update.available"),
    install: t("update.install"),
    later: t("update.later"),
    notes: t("update.notes"),
    downloading: t("update.downloading"),
    installing: t("update.installing"),
    ready: t("update.ready"),
    readyText: t("update.readyText"),
    restart: t("update.restart"),
    error: t("update.error"),
    retry: t("update.retry"),
    close: t("common.close"),
  });
</script>

{#if bannerState !== "idle"}
  <div class="px-4 pt-3">
    <UpdateBanner
      state={bannerState}
      info={updater.info}
      progress={{ downloaded: updater.downloaded, total: updater.total }}
      error={updater.error ? errorText(updater.error).title : null}
      oninstall={() => updater.install()}
      onrestart={() => updater.restart()}
      ondismiss={() => updater.dismiss()}
      oncheck={() => updater.install()}
      {labels}
    />
  </div>
{/if}
