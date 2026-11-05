<script lang="ts">
  // Участок D — главный экран (план §3.4): доски слева, сверху поиск и «Стоп всё»,
  // по центру сетка плиток, снизу громкости и статус VB-CABLE. Всё остальное — модалки.
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { app, ALL, FAVORITES } from "$lib/stores/app.svelte";
  import {
    AddTile,
    BoardList,
    BoomlyLogo,
    Button,
    EmptyState,
    IconButton,
    Icon,
    Menu,
    Panel,
    SearchField,
    SoundGrid,
    SoundTile,
    StatusIndicator,
    VolumeSlider,
    toast,
    type BoardItem,
    type MenuItem,
  } from "$lib/ui";
  import AboutDialog from "$lib/features/AboutDialog.svelte";
  import AppUpdateBanner from "$lib/features/AppUpdateBanner.svelte";
  import BoardDialog from "$lib/features/BoardDialog.svelte";
  import ConfirmDialog from "$lib/features/ConfirmDialog.svelte";
  import FirstRunWizard from "$lib/features/FirstRunWizard.svelte";
  import SettingsDialog from "$lib/features/SettingsDialog.svelte";
  import SoundEditor from "$lib/features/SoundEditor.svelte";
  import { platform } from "$lib/api";

  // ---------- модалки ----------
  let settingsOpen = $state(false);
  let settingsTab = $state<"sound" | "hotkeys" | "app" | "look" | "updates">("sound");
  let aboutOpen = $state(false);
  let wizardOpen = $state(false);
  let editorOpen = $state(false);
  let editorId = $state<string | null>(null);
  let boardDialogOpen = $state(false);
  let boardDialogId = $state<string | null>(null);
  let confirm = $state<{ open: boolean; title: string; text: string; run: () => Promise<void> | void }>({
    open: false,
    title: "",
    text: "",
    run: () => {},
  });

  // Первый запуск — мастер.
  let wizardShown = false;
  $effect(() => {
    if (app.ready && app.settings && !app.settings.firstRunCompleted && !wizardShown) {
      wizardShown = true;
      wizardOpen = true;
    }
  });

  // ---------- доски ----------
  const boardItems = $derived<BoardItem[]>([
    { id: ALL, name: t("boards.all"), icon: "layout-grid", count: app.library.sounds.length },
    { id: FAVORITES, name: t("boards.favorites"), icon: "star", tone: "favorite", count: app.library.sounds.filter((s) => s.favorite).length },
    ...app.library.boards.map((b) => ({
      id: b.id,
      name: b.name,
      emoji: b.emoji,
      count: b.soundIds.length,
      playing: b.soundIds.some((id) => id in app.playing),
    })),
  ]);
  const title = $derived(
    app.query.trim()
      ? t("boards.searchResults", { count: app.visibleSounds.length })
      : app.activeBoard === ALL
        ? t("boards.all")
        : app.activeBoard === FAVORITES
          ? t("boards.favorites")
          : (app.currentBoard?.name ?? ""),
  );
  const titleEmoji = $derived(app.query.trim() || app.activeBoard === ALL || app.activeBoard === FAVORITES ? null : app.currentBoard?.emoji);

  // ---------- меню ----------
  let menu = $state<{ open: boolean; x: number; y: number; items: MenuItem[]; target: string }>({
    open: false,
    x: 0,
    y: 0,
    items: [],
    target: "",
  });
  function pos(e: MouseEvent) {
    if (e.clientX || e.clientY) return { x: e.clientX, y: e.clientY };
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    return { x: r.left, y: r.bottom + 4 };
  }
  function soundMenu(id: string, e: MouseEvent) {
    const s = app.soundsById.get(id);
    if (!s) return;
    const onBoard = app.currentBoard && !app.query.trim();
    menu = {
      open: true,
      ...pos(e),
      target: id,
      items: [
        { id: "play", label: id in app.playing ? t("tile.stop") : t("tile.play"), icon: id in app.playing ? "square" : "play" },
        { id: "edit", label: t("tile.edit"), icon: "pencil" },
        { id: "fav", label: s.favorite ? t("tile.favoriteRemove") : t("tile.favoriteAdd"), icon: "star" },
        { separator: true },
        ...(onBoard ? [{ id: "unboard", label: t("tile.removeFromBoard"), icon: "x" as const }] : []),
        { id: "delete", label: t("tile.delete"), icon: "trash-2", danger: true, shortcut: "Del" },
      ],
    };
  }
  function boardMenu(id: string, e: MouseEvent) {
    if (id === ALL || id === FAVORITES) return;
    const i = app.library.boards.findIndex((b) => b.id === id);
    menu = {
      open: true,
      ...pos(e),
      target: id,
      items: [
        { id: "b-rename", label: t("boards.rename"), icon: "pencil" },
        { id: "b-up", label: t("boards.moveUp"), icon: "chevron-left", disabled: i <= 0 },
        { id: "b-down", label: t("boards.moveDown"), icon: "chevron-right", disabled: i >= app.library.boards.length - 1 },
        { separator: true },
        { id: "b-delete", label: t("boards.delete"), icon: "trash-2", danger: true },
      ],
    };
  }
  function onMenu(action: string) {
    const id = menu.target;
    if (action === "play") app.play(id);
    else if (action === "edit") openEditor(id);
    else if (action === "fav") app.toggleFavorite(id);
    else if (action === "unboard" && app.currentBoard) app.removeFromBoard(id, app.currentBoard.id);
    else if (action === "delete") askDeleteSound(id);
    else if (action === "b-rename") {
      boardDialogId = id;
      boardDialogOpen = true;
    } else if (action === "b-up") app.moveBoard(id, -1);
    else if (action === "b-down") app.moveBoard(id, 1);
    else if (action === "b-delete") {
      const b = app.library.boards.find((x) => x.id === id);
      confirm = {
        open: true,
        title: t("boards.deleteTitle", { name: b?.name ?? "" }),
        text: t("boards.deleteText"),
        run: () => app.deleteBoard(id),
      };
    }
  }
  function openEditor(id: string) {
    editorId = id;
    editorOpen = true;
  }
  function askDeleteSound(id: string) {
    const s = app.soundsById.get(id);
    confirm = {
      open: true,
      title: t("tile.deleteTitle", { name: s?.name ?? "" }),
      text: t("tile.deleteText"),
      run: async () => {
        await app.deleteSounds([id]);
        editorOpen = false;
      },
    };
  }

  // ---------- добавление ----------
  let dragging = $state(false);
  async function importPaths(paths: string[]) {
    const r = await app.importPaths(paths);
    if (!r) return;
    if (r.added.length) toast.success(t("import.added", { count: r.added.length }));
    if (r.skipped.length) toast.warning({ title: t("import.skipped", { count: r.skipped.length }), text: t("import.skippedText") });
    if (!r.added.length && !r.skipped.length) toast.info(t("import.nothing"));
  }
  async function addFiles() {
    const paths = await platform.pickSoundFiles({ multiple: true });
    if (paths?.length) await importPaths(paths);
  }
  async function addFolder() {
    const dir = await platform.pickSoundFolder();
    if (dir) await importPaths([dir]);
  }
  onMount(() => {
    let un: (() => void) | undefined;
    platform
      .onFileDrop((e) => {
        if (e.type === "enter" || e.type === "over") dragging = true;
        else if (e.type === "leave") dragging = false;
        else {
          dragging = false;
          importPaths(e.paths);
        }
      })
      .then((u) => (un = u));
    return () => un?.();
  });

  // ---------- клавиатура ----------
  let search: HTMLInputElement | null = $state(null);
  function keydown(e: KeyboardEvent) {
    const modalOpen = settingsOpen || aboutOpen || wizardOpen || editorOpen || boardDialogOpen || confirm.open;
    if (modalOpen) return;
    if ((e.ctrlKey || e.metaKey) && e.code === "KeyF") {
      e.preventDefault();
      search?.focus();
      search?.select();
    } else if ((e.ctrlKey || e.metaKey) && e.code === "KeyO") {
      e.preventDefault();
      addFiles();
    } else if (e.key === "Escape" && app.query) {
      app.query = "";
      search?.blur();
    }
  }

  // ---------- нижняя панель ----------
  const cableOk = $derived(!!app.cable?.installed);
  const micMuted = $derived(!!app.audio?.micMuted);
  const passthrough = $derived(!!app.audio?.micPassthrough);

  const tileLabels = $derived({ play: t("tile.play"), stop: t("tile.stop"), menu: t("tile.menu"), broken: t("tile.broken") });
</script>

<svelte:window onkeydown={keydown} />
<svelte:head><title>Boomly</title></svelte:head>

<div class="shell">
  <!-- шапка -->
  <Panel as="header" cut accentEdge padding="none" class="top">
    <BoomlyLogo size={22} />
    <SearchField bind:value={app.query} bind:ref={search} placeholder={t("top.search")} clearLabel={t("top.clearSearch")} class="search" />
    <div class="ml-auto flex items-center gap-1">
      <IconButton icon="settings" label={t("top.settings")} onclick={() => (settingsOpen = true)} />
      <IconButton icon="info" label={t("top.info")} onclick={() => (aboutOpen = true)} />
      <Button variant="danger" icon="octagon-x" cut class="ml-2" disabled={!app.anyPlaying} onclick={app.stopAll} title={app.hotkeyFor("stopAll") ?? undefined}>
        {t("top.stopAll")}
      </Button>
    </div>
  </Panel>

  <!-- доски -->
  <aside class="side">
    <BoardList
      items={boardItems}
      activeId={app.query.trim() ? null : app.activeBoard}
      label={t("boards.label")}
      addLabel={t("boards.new")}
      onselect={(id) => {
        app.query = "";
        app.activeBoard = id;
      }}
      onmenu={boardMenu}
      onadd={() => {
        boardDialogId = null;
        boardDialogOpen = true;
      }}
    />
  </aside>

  <!-- сетка -->
  <main class="main">
    <AppUpdateBanner />
    <div class="main-scroll">
      {#if app.library.sounds.length || !app.ready}
        <div class="head">
          <h1>
            {#if titleEmoji}<span>{titleEmoji}</span>{:else if app.activeBoard === FAVORITES && !app.query.trim()}<Icon name="star" size={18} class="text-playing" />{/if}
            {title}
          </h1>
          <Button size="sm" variant="secondary" icon="plus" onclick={addFiles} title="Ctrl+O">{t("tile.add")}</Button>
        </div>
      {/if}

      {#if !app.ready}
        <!-- загрузка: ничего не мигаем -->
      {:else if !app.library.sounds.length}
        <div class="empty-wrap">
          <EmptyState icon="audio-lines" title={t("empty.library.title")} text={t("empty.library.text")}>
            {#snippet actions()}
              <Button variant="primary" icon="plus" onclick={addFiles}>{t("empty.addFiles")}</Button>
              <Button variant="secondary" icon="folder-open" onclick={addFolder}>{t("empty.addFolder")}</Button>
            {/snippet}
          </EmptyState>
        </div>
      {:else if !app.visibleSounds.length}
        <div class="empty-wrap">
          {#if app.query.trim()}
            <EmptyState icon="search" title={t("empty.search.title")} text={t("empty.search.text", { query: app.query.trim() })} />
          {:else if app.activeBoard === FAVORITES}
            <EmptyState icon="star" title={t("empty.favorites.title")} text={t("empty.favorites.text")} />
          {:else}
            <EmptyState icon="layout-grid" title={t("empty.board.title")} text={t("empty.board.text")}>
              {#snippet actions()}
                <Button variant="primary" icon="plus" onclick={addFiles}>{t("empty.addFiles")}</Button>
              {/snippet}
            </EmptyState>
          {/if}
        </div>
      {:else}
        <SoundGrid minTile={156} gap={12} label={title}>
          {#each app.visibleSounds as s (s.id)}
            <SoundTile
              name={s.name}
              emoji={s.emoji}
              color={s.color || "cyan"}
              hotkey={app.hotkeyBySound.get(s.id) ?? null}
              favorite={s.favorite}
              durationMs={s.durationMs}
              playing={s.id in app.playing}
              progress={app.playing[s.id]?.progress ?? null}
              onplay={() => app.play(s.id)}
              onmenu={(e) => soundMenu(s.id, e)}
              ondblclick={() => openEditor(s.id)}
              onkeydown={(e) => {
                if (e.key === "Delete") askDeleteSound(s.id);
                else if (e.key === "F2") openEditor(s.id);
              }}
              labels={tileLabels}
            />
          {/each}
          {#if !app.query.trim()}
            <AddTile label={t("tile.add")} hint={t("tile.addHint")} dragover={dragging} onclick={addFiles} />
          {/if}
        </SoundGrid>
      {/if}
    </div>
  </main>

  <!-- громкости и статус -->
  <footer class="bottom">
    {#if app.audio}
      <VolumeSlider label={t("bottom.monitor")} icon="headphones" value={app.audio.monitorVolume} onchange={(v) => app.setVolume("monitor", v)} compact />
      <VolumeSlider label={t("bottom.cable")} icon="volume-2" value={app.audio.cableVolume} onchange={(v) => app.setVolume("cable", v)} disabled={!cableOk} compact />
      <VolumeSlider
        label={t("bottom.mic")}
        icon="mic"
        value={app.audio.micVolume}
        onchange={(v) => app.setVolume("mic", v)}
        muted={micMuted}
        onmute={() => app.toggleMicMute()}
        muteLabel={t("bottom.micMute")}
        unmuteLabel={t("bottom.micUnmute")}
        level={passthrough && !micMuted ? app.micLevel : null}
        disabled={!passthrough}
        compact
      />
    {/if}
    <div class="ml-auto flex items-center gap-2">
      {#if app.cable}
        {#if cableOk}
          <StatusIndicator
            tone="ok"
            icon="cable"
            label={t("bottom.cableOk")}
            detail={t("bottom.cableOkDetail", { name: app.cable.outputDeviceName ?? "CABLE Input" })}
            onclick={() => {
              settingsTab = "sound";
              settingsOpen = true;
            }}
          />
        {:else}
          <StatusIndicator
            tone="error"
            icon="cable"
            label={t("bottom.cableMissing")}
            detail={t("bottom.cableMissingDetail")}
            pulse
            onclick={() => (wizardOpen = true)}
          />
        {/if}
      {/if}
      {#if app.audio}
        <StatusIndicator
          tone={!passthrough ? "off" : micMuted ? "off" : "live"}
          icon={micMuted || !passthrough ? "mic-off" : "mic"}
          label={!passthrough ? t("bottom.passthroughOff") : micMuted ? t("bottom.voiceOff") : t("bottom.voiceOn")}
          detail={app.hotkeyFor("toggleMicMute") ?? undefined}
          onclick={() => (passthrough ? app.toggleMicMute() : ((settingsTab = "sound"), (settingsOpen = true)))}
        />
      {/if}
    </div>
  </footer>

  {#if dragging}
    <div class="drop" aria-hidden="true">
      <div class="drop-card">
        <Icon name="upload" size={36} />
        <strong>{t("drop.title")}</strong>
        <span>{app.currentBoard ? t("drop.toBoard", { name: app.currentBoard.name }) : t("drop.toLibrary")}</span>
        <small>{t("drop.formats")}</small>
      </div>
    </div>
  {/if}
</div>

<Menu bind:open={menu.open} x={menu.x} y={menu.y} items={menu.items} onselect={onMenu} label={t("tile.menu")} />
<SoundEditor bind:open={editorOpen} soundId={editorId} ondelete={askDeleteSound} />
<BoardDialog bind:open={boardDialogOpen} boardId={boardDialogId} />
<ConfirmDialog bind:open={confirm.open} title={confirm.title} text={confirm.text} onconfirm={confirm.run} />
<SettingsDialog
  bind:open={settingsOpen}
  bind:tab={settingsTab}
  onwizard={() => {
    settingsOpen = false;
    wizardOpen = true;
  }}
/>
<AboutDialog bind:open={aboutOpen} />
<FirstRunWizard bind:open={wizardOpen} onadd={addFiles} />

<style>
  .shell {
    display: grid;
    grid-template-columns: 232px 1fr;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
    overflow: hidden;
  }
  :global(.top) {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 18px;
    margin: 8px 8px 0;
    padding: 10px 12px 10px 16px !important;
  }
  :global(.top .search) {
    width: min(360px, 40vw);
  }
  .side {
    padding: 12px 8px;
    overflow: auto;
  }
  .main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    margin: 8px 8px 0 0;
    border-radius: var(--radius-lg);
    background: var(--surface-1);
    border: 1px solid var(--line);
  }
  .main-scroll {
    flex: 1;
    overflow: auto;
    padding: 16px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
  }
  h1 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 20px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty-wrap {
    display: grid;
    place-items: center;
    min-height: 60%;
    padding-top: 6vh;
  }
  .bottom {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 10px 16px;
  }
  .bottom > :global(.vs) {
    width: 200px;
    flex: 0 1 200px;
  }
  .drop {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    background: rgb(3 3 6 / 0.7);
    pointer-events: none;
    animation: fade var(--dur) var(--ease);
  }
  .drop-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 36px 56px;
    border-radius: var(--radius-lg);
    border: 2px dashed var(--accent);
    background: var(--glass);
    color: var(--accent);
    --glow-color: var(--accent);
    box-shadow: var(--glow-md);
  }
  .drop-card strong {
    font-family: var(--font-display);
    font-weight: 400;
    font-size: 20px;
    color: var(--text);
  }
  .drop-card span {
    color: var(--text-muted);
  }
  .drop-card small {
    font-family: var(--font-mono);
    color: var(--text-faint);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @media (max-width: 860px) {
    .shell {
      grid-template-columns: 180px 1fr;
    }
    .bottom {
      gap: 14px;
      flex-wrap: wrap;
    }
  }
</style>
