<script lang="ts">
  // Настройки: Звук (устройства, голос, тест), Клавиши, Приложение, Вид, Обновления.
  import { api, isAppError, type AudioDevice, type DeviceSelection, type HotkeyAction } from "$lib/api";
  import { LANGUAGES, t } from "$lib/i18n";
  import { app } from "$lib/stores/app.svelte";
  import { ACCENTS, Button, ColorSwatches, Icon, Modal, Segmented, Select, Switch, toast } from "$lib/ui";
  import type { IconName } from "$lib/ui";
  import { updater } from "$lib/updater";
  import HotkeyRecorder from "./HotkeyRecorder.svelte";
  import { attempt, errorText } from "./errors";

  type Tab = "sound" | "hotkeys" | "app" | "look" | "updates";
  interface Props {
    open: boolean;
    tab?: Tab;
    onwizard?: () => void;
  }
  let { open = $bindable(), tab = $bindable("sound"), onwizard }: Props = $props();

  const tabs: { id: Tab; icon: IconName }[] = [
    { id: "sound", icon: "headphones" },
    { id: "hotkeys", icon: "keyboard" },
    { id: "app", icon: "settings" },
    { id: "look", icon: "sparkles" },
    { id: "updates", icon: "download" },
  ];

  const DEF = "__default";
  const opts = (list: AudioDevice[]) => [
    { value: DEF, label: t("common.default") },
    ...list.map((d) => ({ value: d.id, label: d.name + (d.isDefault ? " ★" : "") })),
  ];
  const outputs = $derived(opts(app.devices.outputs.filter((d) => !d.isCable)));
  const cables = $derived([
    { value: DEF, label: t("common.auto") },
    ...app.devices.outputs.map((d) => ({ value: d.id, label: d.name })),
  ]);
  const inputs = $derived(opts(app.devices.inputs.filter((d) => !d.isCable)));

  function setDevice(key: keyof DeviceSelection, v: string) {
    app.setDevices({ [key]: v === DEF ? null : v });
  }
  async function test(target: "monitor" | "cable") {
    const r = await attempt(() => api.audio.playTest(target));
    if (r !== undefined) toast.info(t("settings.testSent"));
  }

  // хоткеи действий
  let hkError = $state<Record<string, string | null>>({});
  // Пересоздать поле после отказа, чтобы оно снова показывало действующее сочетание.
  let hkRev = $state(0);
  async function setActionHotkey(type: "stopAll" | "toggleMicMute", acc: string | null) {
    hkError[type] = null;
    const old = app.hotkeyFor(type);
    try {
      if (acc) await app.setHotkey({ accelerator: acc, action: { type } as HotkeyAction });
      else if (old) await app.removeHotkey(old);
    } catch (e) {
      hkError[type] = isAppError(e) && e.code === "hotkey_conflict" ? t("error.hotkey_conflict") : errorText(e).title;
      hkRev++;
    }
  }

  const fmtDate = (ms: number) => new Date(ms).toLocaleString(app.settings?.language === "en" ? "en-US" : "ru-RU");
</script>

<Modal bind:open title={t("settings.title")} size="lg" closeLabel={t("common.close")}>
  {#if app.settings && app.audio}
    {@const s = app.settings}
    {@const a = app.audio}
    <div class="st">
      <nav class="tabs" aria-label={t("settings.title")}>
        {#each tabs as tb (tb.id)}
          <button type="button" class:on={tab === tb.id} aria-current={tab === tb.id ? "page" : undefined} onclick={() => (tab = tb.id)}>
            <Icon name={tb.icon} size={16} />
            {t(`settings.tab.${tb.id}`)}
          </button>
        {/each}
      </nav>

      <div class="pane">
        {#if tab === "sound"}
          <div class="row2">
            <Select
              label={t("settings.monitorDevice")}
              hint={t("settings.monitorDeviceHint")}
              value={a.monitorDeviceId ?? DEF}
              options={outputs}
              onchange={(v) => setDevice("monitorDeviceId", v)}
            />
            <Button variant="secondary" size="sm" icon="play" onclick={() => test("monitor")}>{t("settings.test")}</Button>
          </div>
          <div class="row2">
            <Select
              label={t("settings.cableDevice")}
              hint={t("settings.cableDeviceHint")}
              value={a.cableDeviceId ?? DEF}
              options={cables}
              onchange={(v) => setDevice("cableDeviceId", v)}
            />
            <Button variant="secondary" size="sm" icon="play" disabled={!app.cable?.installed} onclick={() => test("cable")}
              >{t("settings.test")}</Button
            >
          </div>
          <Select
            label={t("settings.micDevice")}
            hint={t("settings.micDeviceHint")}
            value={a.micDeviceId ?? DEF}
            options={inputs}
            onchange={(v) => setDevice("micDeviceId", v)}
          />
          <Switch
            label={t("settings.passthrough")}
            description={t("settings.passthroughHint")}
            checked={a.micPassthrough}
            onchange={(v) => app.setPassthrough(v)}
          />
        {:else if tab === "hotkeys"}
          <p class="note">{t("settings.hotkeysHint")}</p>
          {#key hkRev}
          <div class="grp">
            <span class="lbl">{t("settings.hotkeyStopAll")}</span>
            <HotkeyRecorder
              value={app.hotkeyFor("stopAll")}
              error={hkError.stopAll}
              label={t("settings.hotkeyStopAll")}
              onchange={(v) => setActionHotkey("stopAll", v)}
            />
          </div>
          <div class="grp">
            <span class="lbl">{t("settings.hotkeyMicMute")}</span>
            <HotkeyRecorder
              value={app.hotkeyFor("toggleMicMute")}
              error={hkError.toggleMicMute}
              label={t("settings.hotkeyMicMute")}
              onchange={(v) => setActionHotkey("toggleMicMute", v)}
            />
          </div>
          {/key}
        {:else if tab === "app"}
          <Select
            label={t("settings.language")}
            value={s.language}
            options={LANGUAGES}
            onchange={(v) => app.updateSettings({ language: v as "ru" | "en" })}
          />
          <Switch label={t("settings.autostart")} checked={s.autostart} onchange={(v) => app.updateSettings({ autostart: v })} />
          <Switch
            label={t("settings.startMinimized")}
            checked={s.startMinimized}
            disabled={!s.autostart}
            onchange={(v) => app.updateSettings({ startMinimized: v })}
          />
          <Switch
            label={t("settings.closeToTray")}
            description={t("settings.closeToTrayHint")}
            checked={s.closeToTray}
            onchange={(v) => app.updateSettings({ closeToTray: v })}
          />
          <Switch
            label={t("settings.copyFiles")}
            description={t("settings.copyFilesHint")}
            checked={s.copyImportedFiles}
            onchange={(v) => app.updateSettings({ copyImportedFiles: v })}
          />
          <div class="line">
            <div>
              <span class="lbl">{t("settings.dataDir")}</span>
              <p class="note mono" title={app.info?.dataDir}>{app.info?.dataDir ?? "—"}</p>
            </div>
            <Button variant="secondary" size="sm" icon="folder-open" onclick={() => attempt(() => api.app.openDataDir())}
              >{t("settings.openDataDir")}</Button
            >
          </div>
          <div class="line">
            <span class="lbl">{t("settings.wizard")}</span>
            <Button variant="secondary" size="sm" icon="sparkles" onclick={() => onwizard?.()}>{t("settings.wizardRun")}</Button>
          </div>
        {:else if tab === "look"}
          <div class="grp">
            <span class="lbl">{t("settings.accent")}</span>
            <ColorSwatches value={s.accent} colors={ACCENTS} label={t("settings.accent")} onchange={(v) => app.updateSettings({ accent: v })} />
          </div>
          <div class="grp">
            <span class="lbl">{t("settings.effects")}</span>
            <Segmented
              label={t("settings.effects")}
              value={s.effects}
              options={[
                { value: "minimal", label: t("settings.effects.minimal") },
                { value: "normal", label: t("settings.effects.normal") },
              ]}
              onchange={(v) => app.updateSettings({ effects: v as "minimal" | "normal" })}
            />
            <p class="note">{t("settings.effectsHint")}</p>
          </div>
        {:else if tab === "updates"}
          <div class="line">
            <div>
              <span class="lbl">{t("settings.version")}</span>
              <p class="note mono">{app.info?.version ?? "—"}</p>
            </div>
            <Button
              variant="secondary"
              size="sm"
              icon="refresh-cw"
              loading={updater.status === "checking"}
              disabled={updater.status === "disabled"}
              onclick={() => updater.checkNow()}>{t("settings.checkNow")}</Button
            >
          </div>
          <p class="status">
            {#if updater.status === "disabled"}
              {t("settings.updatesDisabled")}
            {:else if updater.status === "checking"}
              {t("settings.checking")}
            {:else if updater.status === "upToDate"}
              <Icon name="circle-check" size={16} class="text-ok" /> {t("settings.upToDate")}
            {:else if updater.info && (updater.status === "available" || updater.status === "downloading" || updater.status === "installing" || updater.status === "ready")}
              <Icon name="sparkles" size={16} class="text-accent" /> {t("settings.updateFound", { version: updater.info.version })}
            {:else if updater.status === "error" && updater.error}
              <Icon name="circle-alert" size={16} class="text-danger" /> {errorText(updater.error).title}
            {/if}
            {#if updater.lastCheckedAt}<span class="when">· {fmtDate(updater.lastCheckedAt)}</span>{/if}
          </p>
          {#if updater.status === "available"}
            <Button variant="primary" icon="download" onclick={() => updater.install()}>{t("update.install")}</Button>
          {/if}
          <Switch label={t("settings.checkUpdates")} checked={s.checkUpdates} onchange={(v) => app.updateSettings({ checkUpdates: v })} />
        {/if}
      </div>
    </div>
  {/if}
</Modal>

<style>
  .st {
    display: grid;
    grid-template-columns: 170px 1fr;
    gap: 24px;
    min-height: 380px;
  }
  .tabs {
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-right: 1px solid var(--line);
    padding-right: 12px;
  }
  .tabs button {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }
  .tabs button:hover {
    background: var(--surface-3);
    color: var(--text);
  }
  .tabs button.on {
    background: var(--accent-soft);
    color: var(--text);
  }
  .tabs button.on :global(svg) {
    color: var(--accent);
  }
  .pane {
    display: flex;
    flex-direction: column;
    gap: 20px;
    min-width: 0;
  }
  .row2 {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: start;
    gap: 10px;
  }
  .row2 > :global(button) {
    margin-top: 28px;
  }
  .grp {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .lbl {
    font-weight: 500;
  }
  .note {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12.5px;
    word-break: break-all;
  }
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--text-muted);
  }
  .when {
    color: var(--text-faint);
    font-size: 12.5px;
  }
</style>
