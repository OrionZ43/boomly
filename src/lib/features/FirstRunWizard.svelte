<script lang="ts">
  // Мастер первого запуска: VB-CABLE → устройства + тест → Discord → готово.
  // Цель: новичок за минуту слышит звук в Discord. Закрытие на любом шаге = firstRunCompleted.
  import { api, type DeviceSelection } from "$lib/api";
  import { VB_CABLE_URL } from "$lib/config/links";
  import { t } from "$lib/i18n";
  import { app } from "$lib/stores/app.svelte";
  import { Button, Icon, Modal, Select, StatusIndicator, Switch, toast } from "$lib/ui";
  import { attempt } from "./errors";
  import { platform } from "$lib/api";

  interface Props {
    open: boolean;
    onadd?: () => void;
  }
  let { open = $bindable(), onadd }: Props = $props();

  const TOTAL = 4;
  let step = $state(1);
  let checking = $state(false);
  let tested = $state<{ monitor: boolean; cable: boolean }>({ monitor: false, cable: false });

  $effect(() => {
    if (open) {
      step = 1;
      tested = { monitor: false, cable: false };
    }
  });

  const cableOk = $derived(!!app.cable?.installed);
  const DEF = "__default";
  const outputs = $derived([
    { value: DEF, label: t("common.default") },
    ...app.devices.outputs.filter((d) => !d.isCable).map((d) => ({ value: d.id, label: d.name })),
  ]);
  const inputs = $derived([
    { value: DEF, label: t("common.default") },
    ...app.devices.inputs.filter((d) => !d.isCable).map((d) => ({ value: d.id, label: d.name })),
  ]);

  async function recheck() {
    checking = true;
    await app.refreshCable();
    checking = false;
  }
  async function test(target: "monitor" | "cable") {
    const r = await attempt(() => api.audio.playTest(target));
    if (r !== undefined) {
      tested[target] = true;
      toast.info(t("settings.testSent"));
    }
  }
  function setDevice(key: keyof DeviceSelection, v: string) {
    app.setDevices({ [key]: v === DEF ? null : v });
  }
  async function finish(addSounds = false) {
    open = false;
    await app.updateSettings({ firstRunCompleted: true });
    if (addSounds) onadd?.();
  }
</script>

<Modal bind:open title={t("wizard.title")} description={t("wizard.step", { n: step, total: TOTAL })} size="md" closeLabel={t("common.close")} onclose={() => app.updateSettings({ firstRunCompleted: true })}>
  <ol class="dots" aria-hidden="true">
    {#each Array(TOTAL) as _, i (i)}<li class:on={i + 1 <= step}></li>{/each}
  </ol>

  {#if step === 1}
    <h3><Icon name="cable" size={20} /> {t("wizard.cable.title")}</h3>
    <p>{t("wizard.cable.text")}</p>
    {#if cableOk}
      <div class="ok">
        <StatusIndicator tone="ok" icon="cable" label={t("wizard.cable.ok")} />
        <p class="small">{t("wizard.cable.okText", { out: app.cable?.outputDeviceName ?? "CABLE Input", in: app.cable?.inputDeviceName ?? "CABLE Output" })}</p>
      </div>
    {:else}
      <StatusIndicator tone="error" icon="cable" label={t("wizard.cable.missing")} />
      <ol class="steps">
        <li>{t("wizard.cable.step1")}</li>
        <li>{t("wizard.cable.step2")}</li>
        <li>{t("wizard.cable.step3")}</li>
      </ol>
      <div class="flex flex-wrap gap-2">
        <Button variant="primary" icon="globe" onclick={() => platform.openExternal(app.cable?.downloadUrl || VB_CABLE_URL)}>{t("wizard.cable.open")}</Button>
        <Button variant="secondary" icon="refresh-cw" loading={checking} onclick={recheck}>{t("common.retry")}</Button>
      </div>
    {/if}
  {:else if step === 2}
    <h3><Icon name="headphones" size={20} /> {t("wizard.devices.title")}</h3>
    <p>{t("wizard.devices.text")}</p>
    {#if app.audio}
      <div class="flex flex-col gap-4">
        <div class="pair">
          <Select label={t("settings.monitorDevice")} value={app.audio.monitorDeviceId ?? DEF} options={outputs} onchange={(v) => setDevice("monitorDeviceId", v)} />
          <Button variant={tested.monitor ? "secondary" : "primary"} icon={tested.monitor ? "check" : "play"} onclick={() => test("monitor")}>{t("settings.test")}</Button>
        </div>
        <Select label={t("settings.micDevice")} value={app.audio.micDeviceId ?? DEF} options={inputs} onchange={(v) => setDevice("micDeviceId", v)} />
        <p class="small">{t("wizard.devices.testHint")}</p>
      </div>
    {/if}
  {:else if step === 3}
    <h3><Icon name="mic" size={20} /> {t("wizard.discord.title")}</h3>
    <p>{t("wizard.discord.text")}</p>
    <ol class="steps">
      <li>{t("wizard.discord.step1")}</li>
      <li>{@html t("wizard.discord.step2").replace(/«([^»]+)»|“([^”]+)”/, (m) => `<b>${m}</b>`)}</li>
      <li>{t("wizard.discord.step3")}</li>
    </ol>
    {#if app.audio}
      <Switch label={t("settings.passthrough")} checked={app.audio.micPassthrough} onchange={(v) => app.setPassthrough(v)} />
    {/if}
    <div class="pair mt-2">
      <p class="small">{t("settings.testCable")}</p>
      <Button variant="secondary" icon={tested.cable ? "check" : "play"} disabled={!cableOk} onclick={() => test("cable")}>{t("settings.test")}</Button>
    </div>
    <p class="tip"><Icon name="info" size={15} /> {t("wizard.discord.tip")}</p>
  {:else}
    <div class="done">
      <span class="big"><Icon name="circle-check" size={34} /></span>
      <h3>{t("wizard.done.title")}</h3>
      <p>{t("wizard.done.text")}</p>
    </div>
  {/if}

  {#snippet footer()}
    {#if step > 1 && step < TOTAL}
      <Button variant="ghost" icon="chevron-left" class="mr-auto" onclick={() => step--}>{t("common.back")}</Button>
    {/if}
    {#if step === 1 && !cableOk}
      <Button variant="ghost" onclick={() => (step = 2)}>{t("wizard.cable.skip")}</Button>
    {/if}
    {#if step < TOTAL}
      <Button variant="primary" iconRight="chevron-right" onclick={() => step++}>{t("common.next")}</Button>
    {:else}
      <Button variant="ghost" onclick={() => finish(false)}>{t("wizard.done.later")}</Button>
      <Button variant="primary" icon="plus" onclick={() => finish(true)}>{t("wizard.done.add")}</Button>
    {/if}
  {/snippet}
</Modal>

<style>
  .dots {
    display: flex;
    gap: 6px;
    list-style: none;
    margin: 0 0 18px;
    padding: 0;
  }
  .dots li {
    flex: 1;
    height: 3px;
    border-radius: 2px;
    background: var(--line-strong);
  }
  .dots li.on {
    background: var(--accent);
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px;
    font-size: 17px;
  }
  h3 :global(svg) {
    color: var(--accent);
  }
  p {
    margin: 0 0 14px;
    color: var(--text-muted);
  }
  .small {
    font-size: 12.5px;
    margin: 0;
  }
  .ok {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }
  .steps {
    margin: 14px 0 16px;
    padding-left: 22px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .steps li::marker {
    color: var(--accent);
    font-family: var(--font-mono);
    font-weight: 600;
  }
  .steps :global(b) {
    color: var(--accent);
    font-weight: 600;
  }
  .pair {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: end;
    gap: 10px;
  }
  .tip {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin: 16px 0 0;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--surface-2);
    font-size: 12.5px;
  }
  .tip :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
    color: var(--accent);
  }
  .done {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 12px 0;
  }
  .big {
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: color-mix(in oklab, var(--ok) 14%, transparent);
    color: var(--ok);
    margin-bottom: 12px;
  }
</style>
