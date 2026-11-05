<script lang="ts">
  // Баннер обновления — только вид. Состояние и действия — стор `updater` участка E
  // (src/lib/updater/): D передаёт поля пропсами и колбэки. idle/checking — баннер скрыт.
  import Button from "./Button.svelte";
  import IconButton from "./IconButton.svelte";
  import Icon from "./icons/Icon.svelte";

  import type { UpdaterState, UpdateBannerLabels } from "./types";

  interface Props {
    state: UpdaterState;
    info?: { version: string; notes?: string | null; date?: string | null } | null;
    progress?: { downloaded: number; total: number | null } | null;
    error?: string | null;
    oninstall?: () => void;
    onrestart?: () => void;
    ondismiss?: () => void;
    oncheck?: () => void;
    labels?: Partial<UpdateBannerLabels>;
  }

  const RU: UpdateBannerLabels = {
    available: "Доступна версия",
    install: "Обновить",
    later: "Позже",
    notes: "Что нового",
    downloading: "Загрузка обновления",
    ready: "Обновление установлено",
    readyText: "Перезапустите Boomly, чтобы применить.",
    restart: "Перезапустить",
    error: "Не удалось обновиться",
    retry: "Повторить",
    close: "Закрыть",
  };

  let { state: status, info, progress, error, oninstall, onrestart, ondismiss, oncheck, labels = {} }: Props = $props();
  const L = $derived({ ...RU, ...labels });
  let showNotes = $state(false);

  const pct = $derived(
    progress && progress.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null,
  );
  const mb = (b: number) => (b / 1048576).toFixed(1);
</script>

{#if status !== "idle" && status !== "checking"}
  <div class="ub s-{status}" role={status === "error" ? "alert" : "status"}>
    <span class="ic">
      <Icon
        name={status === "error" ? "circle-alert" : status === "ready" ? "circle-check" : status === "downloading" ? "download" : "sparkles"}
        size={18}
      />
    </span>

    <div class="txt">
      {#if status === "available"}
        <strong>{L.available} <span class="ver">{info?.version}</span></strong>
        {#if info?.notes}
          <button type="button" class="link" aria-expanded={showNotes} onclick={() => (showNotes = !showNotes)}>
            {L.notes}
            <Icon name="chevron-down" size={14} class={showNotes ? "rotate-180" : ""} />
          </button>
        {/if}
      {:else if status === "downloading"}
        <strong>{L.downloading} <span class="ver">{info?.version}</span></strong>
        <span class="bar" aria-hidden="true">
          <span class:indet={pct == null} style:width={pct == null ? undefined : `${pct}%`}></span>
        </span>
        <span class="num">
          {[pct != null ? `${pct}%` : null, progress ? `${mb(progress.downloaded)}${progress.total ? ` / ${mb(progress.total)}` : ""} MB` : null]
            .filter(Boolean)
            .join(" · ")}
        </span>
      {:else if status === "ready"}
        <strong>{L.ready}</strong>
        <span class="sub">{L.readyText}</span>
      {:else if status === "error"}
        <strong>{L.error}</strong>
        {#if error}<span class="sub">{error}</span>{/if}
      {/if}
    </div>

    <div class="actions">
      {#if status === "available"}
        {#if ondismiss}<Button variant="ghost" size="sm" onclick={ondismiss}>{L.later}</Button>{/if}
        <Button variant="primary" size="sm" icon="download" onclick={oninstall}>{L.install}</Button>
      {:else if status === "ready"}
        <Button variant="primary" size="sm" icon="refresh-cw" onclick={onrestart}>{L.restart}</Button>
      {:else if status === "error"}
        {#if oncheck}<Button variant="secondary" size="sm" icon="rotate-ccw" onclick={oncheck}>{L.retry}</Button>{/if}
      {/if}
      {#if ondismiss && status !== "downloading" && status !== "available"}
        <IconButton icon="x" label={L.close} size="sm" onclick={ondismiss} />
      {/if}
    </div>

    {#if status === "available" && showNotes && info?.notes}
      <div class="notes">{info.notes}</div>
    {/if}
  </div>
{/if}

<style>
  .ub {
    --t: var(--accent);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 12px;
    padding: 10px 12px 10px 14px;
    border-radius: var(--radius-md);
    background: linear-gradient(90deg, color-mix(in oklab, var(--t) 14%, var(--surface-2)), var(--surface-2) 70%);
    border: 1px solid color-mix(in oklab, var(--t) 40%, transparent);
  }
  .s-ready {
    --t: var(--ok);
  }
  .s-error {
    --t: var(--danger);
  }
  .ic {
    color: var(--t);
    display: grid;
  }
  .txt {
    flex: 1;
    min-width: 200px;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 12px;
  }
  strong {
    font-weight: 600;
  }
  .ver,
  .num {
    font-family: var(--font-mono);
    font-weight: 600;
  }
  .ver {
    color: var(--t);
  }
  .num {
    font-size: 12px;
    color: var(--text-muted);
  }
  .sub {
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    border: 0;
    background: none;
    padding: 0;
    color: var(--text-muted);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .link:hover {
    color: var(--text);
  }
  .link :global(svg) {
    transition: transform var(--dur) var(--ease);
  }
  .bar {
    position: relative;
    width: 160px;
    height: 4px;
    border-radius: 2px;
    background: rgb(255 255 255 / 0.1);
    overflow: hidden;
  }
  .bar > span {
    display: block;
    height: 100%;
    background: var(--t);
    transition: width 200ms linear;
  }
  .bar > .indet {
    width: 35%;
    animation: indet 1.2s var(--ease) infinite;
  }
  @keyframes indet {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(300%);
    }
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .notes {
    flex-basis: 100%;
    padding: 8px 0 2px 30px;
    white-space: pre-line;
    color: var(--text-muted);
    font-size: 12.5px;
    max-height: 160px;
    overflow: auto;
  }
</style>
