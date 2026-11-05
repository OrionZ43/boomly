<script lang="ts">
  // Редактор плитки: название, значок, цвет, громкость, режим, хоткей, избранное, доски, удаление.
  import { isAppError, type PlayMode, type Sound } from "$lib/api";
  import { t } from "$lib/i18n";
  import { app } from "$lib/stores/app.svelte";
  import { Button, ColorSwatches, Modal, Segmented, SoundTile, Switch, TextField, VolumeSlider, toast } from "$lib/ui";
  import EmojiPicker from "./EmojiPicker.svelte";
  import HotkeyRecorder from "./HotkeyRecorder.svelte";
  import { errorText } from "./errors";

  interface Props {
    open: boolean;
    soundId: string | null;
    ondelete?: (id: string) => void;
  }
  let { open = $bindable(), soundId, ondelete }: Props = $props();

  let draft = $state<Sound | null>(null);
  let hotkey = $state<string | null>(null);
  let boards = $state<string[]>([]);
  let hotkeyError = $state<string | null>(null);
  let saving = $state(false);

  $effect(() => {
    if (!open || !soundId) return;
    const s = app.soundsById.get(soundId);
    if (!s) return;
    draft = { ...s, color: s.color || "cyan" };
    hotkey = app.hotkeyBySound.get(s.id) ?? null;
    boards = app.library.boards.filter((b) => b.soundIds.includes(s.id)).map((b) => b.id);
    hotkeyError = null;
  });

  const modes = $derived([
    { value: "toggle", label: t("editor.mode.toggle"), icon: "square" as const },
    { value: "restart", label: t("editor.mode.restart"), icon: "rotate-ccw" as const },
    { value: "overlay", label: t("editor.mode.overlay"), icon: "layers" as const },
  ]);

  async function save() {
    if (!draft) return;
    saving = true;
    hotkeyError = null;
    const old = app.hotkeyBySound.get(draft.id) ?? null;
    try {
      if (hotkey !== old) {
        if (hotkey) await app.setHotkey({ accelerator: hotkey, action: { type: "playSound", soundId: draft.id } });
        else if (old) await app.removeHotkey(old);
      }
    } catch (e) {
      hotkeyError = isAppError(e) && e.code === "hotkey_conflict" ? t("error.hotkey_conflict") : errorText(e).title;
      saving = false;
      return;
    }
    const saved = await app.saveSound({ ...draft, name: draft.name.trim() || draft.name });
    await app.setBoardMembership(draft.id, boards);
    saving = false;
    if (saved) {
      toast.success(t("editor.saved"));
      open = false;
    }
  }
</script>

<Modal bind:open title={t("editor.title")} size="lg" closeLabel={t("common.close")}>
  {#if draft}
    <div class="ed">
      <div class="preview">
        <SoundTile
          name={draft.name || "…"}
          emoji={draft.emoji}
          color={draft.color}
          hotkey={hotkey}
          favorite={draft.favorite}
          playing={draft.id in app.playing}
          progress={app.playing[draft.id]?.progress ?? null}
          onplay={() => app.play(draft!.id)}
          labels={{ play: t("tile.play"), stop: t("tile.stop") }}
        />
        <p class="file" title={draft.path}><span>{t("editor.file")}:</span> {draft.path}</p>
      </div>

      <div class="form">
        <TextField label={t("editor.name")} bind:value={draft.name} maxlength={100} />

        <div class="grp">
          <span class="lbl">{t("editor.emoji")}</span>
          <EmojiPicker bind:value={draft.emoji} />
        </div>

        <div class="grp">
          <span class="lbl">{t("editor.color")}</span>
          <ColorSwatches bind:value={draft.color} label={t("editor.color")} />
        </div>

        <VolumeSlider label={t("editor.volume")} bind:value={draft.volume} max={2} />

        <div class="grp">
          <span class="lbl">{t("editor.mode")}</span>
          <Segmented
            label={t("editor.mode")}
            value={draft.mode}
            options={modes}
            onchange={(v) => (draft!.mode = v as PlayMode)}
          />
          <p class="hint">{t(`editor.mode.hint.${draft.mode}`)}</p>
        </div>

        <div class="grp">
          <span class="lbl">{t("editor.hotkey")}</span>
          <HotkeyRecorder bind:value={hotkey} error={hotkeyError} onchange={() => (hotkeyError = null)} />
          <p class="hint">{t("editor.hotkeyHint")}</p>
        </div>

        <Switch label={t("editor.favorite")} bind:checked={draft.favorite} />

        {#if app.library.boards.length}
          <div class="grp">
            <span class="lbl">{t("editor.boards")}</span>
            <div class="chips">
              {#each app.library.boards as b (b.id)}
                {@const on = boards.includes(b.id)}
                <button
                  type="button"
                  class="chip"
                  class:on
                  aria-pressed={on}
                  onclick={() => (boards = on ? boards.filter((x) => x !== b.id) : [...boards, b.id])}
                >
                  {#if b.emoji}<span>{b.emoji}</span>{/if}{b.name}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
  {#snippet footer()}
    <Button variant="danger" icon="trash-2" class="mr-auto" onclick={() => draft && ondelete?.(draft.id)}>{t("common.delete")}</Button>
    <Button variant="ghost" onclick={() => (open = false)}>{t("common.cancel")}</Button>
    <Button variant="primary" loading={saving} disabled={!draft?.name.trim()} onclick={save}>{t("common.save")}</Button>
  {/snippet}
</Modal>

<style>
  .ed {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 24px;
  }
  @media (max-width: 640px) {
    .ed {
      grid-template-columns: 1fr;
    }
  }
  .preview {
    position: sticky;
    top: 0;
    align-self: start;
  }
  .file {
    margin: 10px 0 0;
    font-size: 12px;
    color: var(--text-faint);
    word-break: break-all;
  }
  .file span {
    color: var(--text-muted);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .grp {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .lbl {
    font-weight: 500;
  }
  .hint {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    height: 30px;
    padding: 0 12px;
    border-radius: 15px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    color: var(--text-muted);
    font: inherit;
    cursor: pointer;
  }
  .chip.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--text);
  }
</style>
