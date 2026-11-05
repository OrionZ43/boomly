<script lang="ts">
  // Запись глобального хоткея. Пока идёт запись, глобальные хоткеи сняты (hotkeys_set_capture),
  // чтобы нажатие не запустило звук. value — сочетание или null.
  import { api } from "$lib/api";
  import { t } from "$lib/i18n";
  import { Button, Icon, Kbd } from "$lib/ui";
  import { captureKey } from "./accelerator";

  interface Props {
    value: string | null;
    onchange?: (value: string | null) => void;
    error?: string | null;
    label?: string;
  }
  let { value = $bindable(), onchange, error = null, label }: Props = $props();

  let recording = $state(false);
  let preview = $state("");
  let hint = $state<string | null>(null);
  let box: HTMLButtonElement | undefined = $state();

  async function start() {
    recording = true;
    preview = "";
    hint = null;
    box?.focus();
    await api.hotkeys.setCapture(true).catch(() => {});
  }
  async function stop() {
    if (!recording) return;
    recording = false;
    preview = "";
    await api.hotkeys.setCapture(false).catch(() => {});
  }
  function set(v: string | null) {
    value = v;
    onchange?.(v);
  }
  function keydown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    const c = captureKey(e);
    if (c.kind === "cancel") stop();
    else if (c.kind === "clear") {
      set(null);
      stop();
    } else if (c.kind === "partial") {
      preview = c.preview;
      hint = c.preview && !c.preview.includes("+") && !["Ctrl", "Alt", "Shift", "Super"].includes(c.preview) ? t("hotkey.needKey") : null;
    } else if (c.kind === "done") {
      set(c.accelerator);
      stop();
    }
  }
  $effect(() => () => {
    if (recording) api.hotkeys.setCapture(false).catch(() => {});
  });
</script>

<div class="hr">
  <button
    bind:this={box}
    type="button"
    class="box"
    class:recording
    class:invalid={!!error}
    aria-label={label ?? t("editor.hotkey")}
    onclick={() => (recording ? stop() : start())}
    onkeydown={keydown}
    onblur={stop}
  >
    {#if recording}
      <span class="rec-dot" aria-hidden="true"></span>
      {#if preview}<Kbd keys={preview} />{:else}<span class="muted">{t("hotkey.recording")}</span>{/if}
      <span class="esc">{t("hotkey.cancelHint")}</span>
    {:else if value}
      <Icon name="keyboard" size={16} />
      <Kbd keys={value} />
    {:else}
      <Icon name="keyboard" size={16} />
      <span class="muted">{t("hotkey.none")}</span>
    {/if}
  </button>
  {#if value && !recording}
    <Button variant="ghost" size="sm" onclick={() => set(null)}>{t("hotkey.clear")}</Button>
  {:else if !recording}
    <Button variant="secondary" size="sm" onclick={start}>{t("hotkey.record")}</Button>
  {/if}
</div>
{#if error}<p class="err">{error}</p>{:else if hint}<p class="hint">{hint}</p>{/if}

<style>
  .hr {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .box {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 38px;
    padding: 0 12px;
    border-radius: var(--radius-md);
    border: 1px dashed var(--line-strong);
    background: var(--surface-2);
    color: var(--text-muted);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .box:hover {
    border-color: var(--accent-line);
  }
  .box.recording {
    border-style: solid;
    border-color: var(--accent);
    box-shadow: var(--glow-sm);
    color: var(--text);
  }
  .box.invalid {
    border-color: var(--danger);
  }
  .muted {
    color: var(--text-muted);
  }
  .esc {
    margin-left: auto;
    font-size: 12px;
    color: var(--text-faint);
  }
  .rec-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--danger);
    animation: blink 1s ease-in-out infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0.3;
    }
  }
  .err,
  .hint {
    margin: 6px 0 0;
    font-size: 12.5px;
  }
  .err {
    color: #ff6b84;
  }
  .hint {
    color: var(--text-muted);
  }
</style>
