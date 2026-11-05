<script lang="ts">
  // Участок C — витрина дизайн-системы (открыть: /ui в `pnpm dev`).
  // Все компоненты и их состояния + пример сборки главного экрана (план §3.4) — для участка D.
  import {
    AddTile,
    BoardList,
    BoomlyLogo,
    Button,
    ColorSwatches,
    EmptyState,
    Icon,
    IconButton,
    Kbd,
    Menu,
    Modal,
    Panel,
    SearchField,
    Select,
    TextField,
    Segmented,
    SoundGrid,
    SoundTile,
    StatusIndicator,
    Switch,
    Toast,
    Toaster,

    UpdateBanner,
    VolumeSlider,
    ACCENTS,
    TILE_COLORS,
    applyTheme,
    icons,
    toast,
    type BoardItem,
    type IconName,
    type MenuItem,
  } from "$lib/ui";

  // ---------- тема ----------
  let accent = $state("cyan");
  let effects = $state<"minimal" | "normal">("normal");
  $effect(() => applyTheme({ accent, effects }));

  // ---------- демо-данные ----------
  const demo = [
    { id: "1", name: "Ба-дум-тсс", emoji: "🥁", color: "cyan", hotkey: "Ctrl+1", durationMs: 2100 },
    { id: "2", name: "Аплодисменты", emoji: "👏", color: "green", hotkey: "Ctrl+2", durationMs: 5400, favorite: true },
    { id: "3", name: "Воздушная тревога (длинное название звука)", emoji: "🚨", color: "red", durationMs: 31000 },
    { id: "4", name: "Bruh", emoji: "😐", color: "purple", hotkey: "F8", durationMs: 900 },
    { id: "5", name: "Сверчки", emoji: "🦗", color: "lime", durationMs: 4000 },
    { id: "6", name: "Вау", emoji: null, color: "magenta", hotkey: "Ctrl+Shift+F1", durationMs: 1500 },
    { id: "7", name: "Rimshot", emoji: "🎺", color: "orange", durationMs: 1800, favorite: true },
    { id: "8", name: "Тишина", emoji: "🤫", color: "slate", durationMs: 3000 },
    { id: "9", name: "Лазер", emoji: "⚡", color: "blue", durationMs: 700 },
    { id: "10", name: "Синтвейв", emoji: "🌆", color: "teal", durationMs: 12000 },
  ];

  // Имитация воспроизведения: клик — старт/стоп, прогресс тикает сам.
  let playing = $state<Record<string, number>>({ "1": 0.35 });
  $effect(() => {
    const t = setInterval(() => {
      for (const id of Object.keys(playing)) {
        const s = demo.find((d) => d.id === id);
        const next = playing[id] + 100 / (s?.durationMs ?? 3000);
        if (next >= 1 && id !== "1") delete playing[id];
        else playing[id] = next >= 1 ? 0 : next;
      }
    }, 100);
    return () => clearInterval(t);
  });
  function toggle(id: string) {
    if (id in playing) delete playing[id];
    else playing[id] = 0;
  }

  // ---------- доски ----------
  const boards: BoardItem[] = [
    { id: "all", name: "Все звуки", icon: "layout-grid", count: 42 },
    { id: "fav", name: "Избранное", icon: "star", count: 7, tone: "favorite" },
    { id: "b1", name: "Мемы", emoji: "😂", count: 18, playing: true },
    { id: "b2", name: "Стрим", emoji: "🎮", count: 9 },
    { id: "b3", name: "Очень длинное название доски для проверки", emoji: "🎵", count: 8 },
  ];
  let activeBoard = $state("b1");

  // ---------- громкости и переключатели ----------
  let monitor = $state(0.8);
  let cable = $state(0.65);
  let mic = $state(1);
  let micMuted = $state(false);
  let soundVol = $state(1.3);
  let micLevel = $state(0.4);
  $effect(() => {
    const t = setInterval(() => (micLevel = micMuted ? 0 : 0.15 + Math.random() * 0.6), 120);
    return () => clearInterval(t);
  });
  let sw1 = $state(true);
  let sw2 = $state(false);
  let mode = $state("toggle");
  let color = $state("magenta");
  let query = $state("");

  // ---------- меню ----------
  let menuOpen = $state(false);
  let menuPos = $state({ x: 0, y: 0 });
  const menuItems: MenuItem[] = [
    { id: "edit", label: "Изменить…", icon: "pencil" },
    { id: "hotkey", label: "Назначить хоткей", icon: "keyboard", shortcut: "Ctrl+K" },
    { id: "fav", label: "В избранное", icon: "star" },
    { separator: true },
    { id: "m-toggle", label: "Играть / стоп", checked: true },
    { id: "m-restart", label: "Сначала", checked: false },
    { id: "m-overlay", label: "Поверх", checked: false },
    { separator: true },
    { id: "folder", label: "Показать файл", icon: "folder-open", disabled: true },
    { id: "delete", label: "Удалить", icon: "trash-2", danger: true, shortcut: "Del" },
  ];
  function openMenu(e: MouseEvent) {
    menuPos = { x: e.clientX, y: e.clientY };
    if (e.clientX === 0 && e.clientY === 0) {
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      menuPos = { x: r.left, y: r.bottom + 4 };
    }
    menuOpen = true;
  }

  // ---------- модал ----------
  let modalOpen = $state(false);
  let confirmOpen = $state(false);

  const iconNames = Object.keys(icons) as IconName[];

  const swatches = [
    { name: "--bg", v: "#07070a" },
    { name: "--surface-1", v: "#0e0f13" },
    { name: "--surface-2", v: "#15171d" },
    { name: "--surface-3", v: "#1d2028" },
    { name: "--cyan", v: "#00f0ff" },
    { name: "--purple", v: "#9d00ff" },
    { name: "--magenta", v: "#ff00c1" },
    { name: "--yellow", v: "#fcee0a", note: "только «играет» и «избранное»" },
    { name: "--ok", v: "#39ff88" },
    { name: "--warn", v: "#ffa53a" },
    { name: "--danger", v: "#ff3355" },
  ];
  const textTokens = [
    { name: "--text", v: "#ececf2", ratio: "17.1:1" },
    { name: "--text-muted", v: "#a8b0c0", ratio: "9.2:1" },
    { name: "--text-faint", v: "#7f889a", ratio: "5.6:1" },
    { name: "--purple-text", v: "#c78bff", ratio: "8.2:1" },
  ];
</script>

<svelte:head><title>Boomly · UI kit</title></svelte:head>

<div class="page">
  <!-- Шапка витрины: срезанный угол — как у шапки приложения. -->
  <Panel as="header" cut accentEdge class="top">
    <BoomlyLogo size={30} />
    <span class="kit">UI kit</span>
    <div class="ml-auto flex flex-wrap items-center gap-4">
      <div class="flex items-center gap-2">
        <span class="lbl">Акцент</span>
        <ColorSwatches bind:value={accent} colors={ACCENTS} label="Акцент" />
      </div>
      <Segmented
        label="Эффекты"
        bind:value={effects}
        options={[
          { value: "minimal", label: "Эффекты: мин" },
          { value: "normal", label: "Обычные" },
        ]}
        size="sm"
      />
    </div>
  </Panel>

  <nav class="toc" aria-label="Разделы">
    {#each [["brand", "Бренд"], ["tokens", "Токены"], ["type", "Шрифты"], ["buttons", "Кнопки"], ["icons", "Иконки"], ["tiles", "Плитки"], ["boards", "Доски"], ["controls", "Контролы"], ["status", "Статусы"], ["feedback", "Модал, меню, тосты"], ["screen", "Главный экран"]] as [id, t] (id)}
      <a href="#{id}">{t}</a>
    {/each}
  </nav>

  <!-- ===================== Бренд ===================== -->
  <section id="brand">
    <h2>Бренд</h2>
    <div class="row items-end gap-10">
      <figure>
        <BoomlyLogo size={96} wordmark={false} badge />
        <figcaption>Иконка приложения · brand/boomly-icon.svg</figcaption>
      </figure>
      <figure>
        <BoomlyLogo size={40} badge wordmark={false} />
        <BoomlyLogo size={24} badge wordmark={false} />
        <BoomlyLogo size={16} badge wordmark={false} />
        <figcaption>Мелкие размеры</figcaption>
      </figure>
      <figure>
        <BoomlyLogo size={48} />
        <figcaption>Логотип: знак + Russo One, градиент cyan → magenta</figcaption>
      </figure>
      <figure>
        <BoomlyLogo size={28} />
        <figcaption>В шапке</figcaption>
      </figure>
    </div>
    <p class="note">
      Знак — буква B из «пэда» со звуковыми волнами; жёлтая искра — «бум» (жёлтый в бренде = звук играет). Подложка с
      двумя срезанными углами — фирменная форма Z43 (CutCornerShape).
    </p>
  </section>

  <!-- ===================== Токены ===================== -->
  <section id="tokens">
    <h2>Цвета и токены</h2>
    <div class="swatches">
      {#each swatches as s (s.name)}
        <div class="sw">
          <span class="chip" style:background={s.v}></span>
          <code>{s.name}</code>
          <span class="hex">{s.v}</span>
          {#if s.note}<span class="hint">{s.note}</span>{/if}
        </div>
      {/each}
    </div>
    <h3>Текст на фоне (WCAG AA ≥ 4.5:1)</h3>
    <div class="swatches">
      {#each textTokens as s (s.name)}
        <div class="sw">
          <span class="sample" style:color={s.v}>Аа Звук 123</span>
          <code>{s.name}</code>
          <span class="hex">{s.v} · {s.ratio}</span>
        </div>
      {/each}
    </div>
    <h3>Палитра плиток (Sound.color)</h3>
    <div class="row gap-2">
      {#each TILE_COLORS as c (c.key)}
        <div class="sw small">
          <span class="chip" style:background={c.hex}></span>
          <code>{c.key}</code>
        </div>
      {/each}
    </div>
    <h3>Формы</h3>
    <div class="row gap-6">
      <div class="shape r-tile">плитка 12px</div>
      <div class="shape r-md">кнопка/поле 8px</div>
      <div class="shape r-lg">модал/панель 16px</div>
      <div class="shape cut-corners cut">срез — только «Стоп всё» и шапка</div>
    </div>
  </section>

  <!-- ===================== Шрифты ===================== -->
  <section id="type">
    <h2>Шрифты</h2>
    <div class="type">
      <div><span class="tag">Russo One · заголовки</span><p class="t-display">Мои доски · Boomly</p></div>
      <div>
        <span class="tag">Inter · текст</span>
        <p class="t-body">Перетащите звуки в окно или нажмите «Добавить». Клик по плитке — играть, ещё раз — стоп.</p>
      </div>
      <div>
        <span class="tag">Chakra Petch · цифры, хоткеи (только латиница)</span>
        <p class="t-mono">0:42 · 85% · Ctrl+Shift+F1 · v0.1.0</p>
      </div>
    </div>
  </section>

  <!-- ===================== Кнопки ===================== -->
  <section id="buttons">
    <h2>Кнопки</h2>
    <div class="row gap-3">
      <Button variant="primary" icon="plus">Добавить звуки</Button>
      <Button variant="secondary" icon="folder-open">Открыть папку</Button>
      <Button variant="ghost" icon="settings">Настройки</Button>
      <Button variant="danger" icon="trash-2">Удалить</Button>
      <Button variant="primary" loading>Импорт…</Button>
      <Button variant="secondary" disabled>Недоступно</Button>
    </div>
    <div class="row gap-3">
      <Button variant="primary" size="sm">Small</Button>
      <Button variant="primary" size="md">Medium</Button>
      <Button variant="primary" size="lg">Large</Button>
      <Button variant="danger" size="lg" icon="octagon-x" cut>Стоп всё</Button>
    </div>
    <div class="row gap-1">
      <IconButton icon="settings" label="Настройки" />
      <IconButton icon="info" label="Инфо" />
      <IconButton icon="mic" label="Микрофон" active />
      <IconButton icon="mic-off" label="Микрофон выключен" tone="danger" active />
      <IconButton icon="star" label="Избранное" tone="playing" active />
      <IconButton icon="pencil" label="Изменить" size="sm" />
      <IconButton icon="trash-2" label="Удалить" size="sm" tone="danger" />
      <IconButton icon="x" label="Закрыть" size="lg" />
      <IconButton icon="plus" label="Нельзя" disabled />
    </div>
    <div class="row gap-3">
      <Kbd keys="Ctrl+Shift+F1" />
      <Kbd keys="F8" />
      <Kbd keys="Alt+Num5" size="sm" />
    </div>
  </section>

  <!-- ===================== Иконки ===================== -->
  <section id="icons">
    <h2>Иконки · Lucide ({iconNames.length})</h2>
    <p class="note">Один набор: Lucide, контур 2px, сетка 24. Новая иконка — строкой в src/lib/ui/icons/paths.ts.</p>
    <div class="icons">
      {#each iconNames as n (n)}
        <div class="ic" title={n}><Icon name={n} size={20} /><span>{n}</span></div>
      {/each}
    </div>
  </section>

  <!-- ===================== Плитки ===================== -->
  <section id="tiles">
    <h2>Плитки звуков</h2>
    <p class="note">Клик — играть/стоп (здесь имитация). ПКМ или «⋯» — меню.</p>
    <h3>Состояния</h3>
    <div class="states">
      <div><span class="tag">обычная</span><SoundTile name="Ба-дум-тсс" emoji="🥁" color="cyan" hotkey="Ctrl+1" durationMs={2100} /></div>
      <div><span class="tag">играет</span><SoundTile name="Аплодисменты" emoji="👏" color="green" playing progress={0.62} durationMs={5400} /></div>
      <div><span class="tag">играет, длина неизвестна</span><SoundTile name="Стрим-заставка" emoji="📺" color="purple" playing /></div>
      <div><span class="tag">избранное</span><SoundTile name="Rimshot" emoji="🎺" color="orange" favorite hotkey="F8" /></div>
      <div><span class="tag">без эмодзи, #hex</span><SoundTile name="Свой цвет" color="#ff6ad5" /></div>
      <div><span class="tag">выделена</span><SoundTile name="Bruh" emoji="😐" color="purple" selected /></div>
      <div><span class="tag">файл не найден</span><SoundTile name="old_sound.mp3" color="red" broken /></div>
      <div><span class="tag">добавить</span><AddTile /></div>
      <div><span class="tag">над ней тащат файлы</span><AddTile dragover /></div>
    </div>

    <h3>Сетка</h3>
    <SoundGrid label="Звуки">
      {#each demo as s (s.id)}
        <SoundTile
          name={s.name}
          emoji={s.emoji}
          color={s.color}
          hotkey={s.hotkey}
          durationMs={s.durationMs}
          favorite={s.favorite}
          playing={s.id in playing}
          progress={playing[s.id] ?? null}
          onplay={() => toggle(s.id)}
          onmenu={openMenu}
        />
      {/each}
      <AddTile />
    </SoundGrid>

    <h3>Пустая доска</h3>
    <SoundGrid isEmpty>
      {#snippet empty()}
        <EmptyState
          icon="audio-lines"
          title="Здесь пока тихо"
          text="Перетащите mp3, wav, ogg или flac прямо в окно — или выберите файлы."
        >
          {#snippet actions()}
            <Button variant="primary" icon="plus">Добавить звуки</Button>
          {/snippet}
        </EmptyState>
      {/snippet}
    </SoundGrid>
  </section>

  <!-- ===================== Доски ===================== -->
  <section id="boards">
    <h2>Список досок</h2>
    <Panel class="w-64" padding="sm">
      <BoardList items={boards} activeId={activeBoard} onselect={(id) => (activeBoard = id)} onadd={() => {}} onmenu={(_, e) => openMenu(e)} />
    </Panel>
  </section>

  <!-- ===================== Контролы ===================== -->
  <section id="controls">
    <h2>Громкость, переключатели, поиск</h2>
    <div class="grid max-w-3xl grid-cols-1 gap-6 md:grid-cols-2">
      <VolumeSlider label="Я слышу" icon="headphones" bind:value={monitor} />
      <VolumeSlider label="В микрофон" icon="volume-2" bind:value={cable} />
      <VolumeSlider
        label="Голос"
        icon="mic"
        bind:value={mic}
        muted={micMuted}
        onmute={() => (micMuted = !micMuted)}
        level={micLevel}
      />
      <VolumeSlider label="Громкость звука (до 200%)" bind:value={soundVol} max={2} />
      <VolumeSlider label="Выключено" value={0.5} disabled />
    </div>
    <div class="mt-6 grid max-w-3xl grid-cols-1 gap-5 md:grid-cols-2">
      <Switch label="Запускать вместе с Windows" bind:checked={sw1} />
      <Switch label="Сворачивать в трей" description="Крестик прячет окно, хоткеи продолжают работать." bind:checked={sw2} />
      <Switch label="Недоступно" disabled />
      <div class="flex flex-col gap-2">
        <span class="lbl">Повторный клик</span>
        <Segmented
          label="Повторный клик"
          bind:value={mode}
          options={[
            { value: "toggle", label: "Стоп", icon: "square" },
            { value: "restart", label: "Сначала", icon: "rotate-ccw" },
            { value: "overlay", label: "Поверх", icon: "layers" },
          ]}
        />
      </div>
      <div class="flex flex-col gap-2">
        <span class="lbl">Цвет плитки</span>
        <ColorSwatches bind:value={color} />
      </div>
      <div class="flex flex-col gap-2">
        <span class="lbl">Поиск</span>
        <SearchField bind:value={query} />
      </div>
      <Select label="Устройство" hint="Наушники или колонки." value="a" options={[{ value: "a", label: "По умолчанию" }, { value: "b", label: "Наушники (Realtek)" }]} />
      <TextField label="Название" placeholder="Например, Мемы" />
      <TextField label="С ошибкой" value="Ctrl+1" error="Это сочетание уже занято" />
    </div>
  </section>

  <!-- ===================== Статусы ===================== -->
  <section id="status">
    <h2>Индикаторы и баннер обновления</h2>
    <div class="row gap-3">
      <StatusIndicator tone="ok" icon="cable" label="VB-CABLE" detail="Кабель найден: CABLE Input" />
      <StatusIndicator tone="error" icon="cable" label="Нет VB-CABLE" detail="Установите драйвер" pulse onclick={() => {}} />
      <StatusIndicator tone="warn" icon="triangle-alert" label="Устройство пропало" />
      <StatusIndicator tone="live" icon="mic" label="Голос в эфире" />
      <StatusIndicator tone="off" icon="mic-off" label="Микрофон выключен" />
    </div>
    <div class="mt-5 flex max-w-3xl flex-col gap-3">
      <UpdateBanner
        state="available"
        info={{ version: "0.2.0", notes: "• Сквозной микрофон\n• Перетаскивание плиток\n• Исправлены хоткеи на F13–F24" }}
        oninstall={() => {}}
        ondismiss={() => {}}
      />
      <UpdateBanner state="downloading" info={{ version: "0.2.0" }} progress={{ downloaded: 7_340_032, total: 12_582_912 }} />
      <UpdateBanner state="downloading" info={{ version: "0.2.0" }} progress={{ downloaded: 1_048_576, total: null }} />
      <UpdateBanner state="ready" onrestart={() => {}} ondismiss={() => {}} />
      <UpdateBanner state="error" error="Нет соединения с GitHub" oncheck={() => {}} ondismiss={() => {}} />
    </div>
  </section>

  <!-- ===================== Модал, меню, тосты ===================== -->
  <section id="feedback">
    <h2>Модал, меню, тосты</h2>
    <div class="row gap-3">
      <Button onclick={() => (modalOpen = true)} icon="pencil">Открыть модал</Button>
      <Button variant="danger" onclick={() => (confirmOpen = true)} icon="trash-2">Подтверждение</Button>
      <Button variant="ghost" icon="ellipsis-vertical" onclick={openMenu}>Меню</Button>
    </div>
    <div class="row gap-3">
      <Button size="sm" onclick={() => toast.success({ title: "Добавлено 5 звуков", text: "Доска «Мемы»" })}>Тост: успех</Button>
      <Button size="sm" onclick={() => toast.info("Хоткеи на паузе, пока вы записываете сочетание")}>Тост: инфо</Button>
      <Button size="sm" onclick={() => toast.warning({ title: "Сочетание занято", text: "Ctrl+1 уже у «Ба-дум-тсс»" })}>Тост: внимание</Button>
      <Button
        size="sm"
        onclick={() =>
          toast.error({ title: "Не удалось открыть файл", text: "boom.mp3: неподдерживаемый формат", action: { label: "Удалить", run: () => {} } })}
        >Тост: ошибка</Button
      >
    </div>
    <div class="row items-start gap-3">
      <Toast kind="success" title="Импортировано 12 звуков" />
      <Toast kind="error" title="VB-CABLE пропал" text="Звук в микрофон не идёт" action={{ label: "Что делать", run: () => {} }} ondismiss={() => {}} />
    </div>
  </section>

  <!-- ===================== Главный экран (пример сборки) ===================== -->
  <section id="screen">
    <h2>Пример сборки главного экрана</h2>
    <p class="note">
      Для участка D: только раскладка из компонентов (§3.4) — доски слева, сверху поиск и «Стоп всё», по центру сетка,
      снизу громкости и статус кабеля.
    </p>
    <div class="app">
      <Panel as="header" cut accentEdge padding="none" class="app-top">
        <BoomlyLogo size={22} />
        <SearchField bind:value={query} class="w-80" />
        <div class="ml-auto flex items-center gap-1">
          <UpdateBanner state="idle" />
          <IconButton icon="settings" label="Настройки" />
          <IconButton icon="info" label="Инфо" />
          <Button variant="danger" icon="octagon-x" cut onclick={() => (playing = {})}>Стоп всё</Button>
        </div>
      </Panel>
      <aside class="app-side">
        <BoardList items={boards} activeId={activeBoard} onselect={(id) => (activeBoard = id)} onadd={() => {}} />
      </aside>
      <main class="app-main">
        <div class="mb-3 flex items-center justify-between">
          <h3 class="m-0 text-lg">😂 Мемы</h3>
          <Button size="sm" variant="secondary" icon="plus">Добавить</Button>
        </div>
        <SoundGrid minTile={148} gap={10}>
          {#each demo.slice(0, 8) as s (s.id)}
            <SoundTile
              name={s.name}
              emoji={s.emoji}
              color={s.color}
              hotkey={s.hotkey}
              favorite={s.favorite}
              playing={s.id in playing}
              progress={playing[s.id] ?? null}
              onplay={() => toggle(s.id)}
              onmenu={openMenu}
            />
          {/each}
          <AddTile />
        </SoundGrid>
      </main>
      <footer class="app-bottom">
        <VolumeSlider label="Я слышу" icon="headphones" bind:value={monitor} compact />
        <VolumeSlider label="В микрофон" icon="volume-2" bind:value={cable} compact />
        <VolumeSlider
          label="Голос"
          icon="mic"
          bind:value={mic}
          muted={micMuted}
          onmute={() => (micMuted = !micMuted)}
          level={micLevel}
          compact
        />
        <div class="ml-auto flex gap-2">
          <StatusIndicator tone="ok" icon="cable" label="VB-CABLE" />
          <StatusIndicator tone={micMuted ? "off" : "live"} icon={micMuted ? "mic-off" : "mic"} label={micMuted ? "Голос выкл" : "Голос"} />
        </div>
      </footer>
    </div>
  </section>

  <footer class="credits">СДЕЛАНО С <span class="heart">❤</span> Z43 STUDIOS</footer>
</div>

<Menu bind:open={menuOpen} x={menuPos.x} y={menuPos.y} items={menuItems} onselect={(id) => toast.info(`Меню: ${id}`)} />

<Modal bind:open={modalOpen} title="Звук" description="Название, цвет, эмодзи и хоткей">
  <div class="flex flex-col gap-4">
    <div class="flex flex-col gap-2">
      <span class="lbl">Цвет</span>
      <ColorSwatches bind:value={color} />
    </div>
    <VolumeSlider label="Громкость звука" bind:value={soundVol} max={2} />
    <Segmented
      label="Повторный клик"
      bind:value={mode}
      options={[
        { value: "toggle", label: "Стоп" },
        { value: "restart", label: "Сначала" },
        { value: "overlay", label: "Поверх" },
      ]}
    />
  </div>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (modalOpen = false)}>Отмена</Button>
    <Button variant="primary" onclick={() => (modalOpen = false)}>Сохранить</Button>
  {/snippet}
</Modal>

<Modal bind:open={confirmOpen} title="Удалить 3 звука?" description="Файлы в папке Boomly тоже удалятся." size="sm">
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (confirmOpen = false)}>Отмена</Button>
    <Button variant="danger" icon="trash-2" onclick={() => (confirmOpen = false)}>Удалить</Button>
  {/snippet}
</Modal>


<style>
  .page {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px 28px 64px;
  }
  :global(.top) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
    position: sticky;
    top: 12px;
    z-index: 20;
  }
  .kit {
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--text-faint);
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .toc {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    margin: 16px 2px 8px;
  }
  .toc a {
    color: var(--text-muted);
    text-decoration: none;
    font-size: 13px;
  }
  .toc a:hover {
    color: var(--accent);
  }
  section {
    padding: 28px 0;
    border-bottom: 1px solid var(--line);
    scroll-margin-top: 90px;
  }
  h2 {
    margin: 0 0 16px;
    font-size: 22px;
  }
  h3 {
    margin: 22px 0 10px;
    font-size: 14px;
    color: var(--text-muted);
  }
  .note {
    color: var(--text-muted);
    max-width: 720px;
    margin: 0 0 14px;
  }
  .lbl,
  .tag {
    font-size: 12px;
    color: var(--text-faint);
  }
  .tag {
    display: block;
    margin-bottom: 6px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    margin-bottom: 14px;
  }
  figure {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
  }
  figcaption {
    flex-basis: 100%;
    font-size: 12px;
    color: var(--text-faint);
  }
  .swatches {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 12px;
  }
  .sw {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .sw.small {
    width: 72px;
  }
  .chip {
    height: 44px;
    border-radius: var(--radius-md);
    border: 1px solid var(--line-strong);
  }
  .sample {
    font-size: 18px;
    font-weight: 600;
  }
  code {
    font-family: var(--font-mono);
    font-size: 12.5px;
    color: var(--text);
  }
  .hex,
  .hint {
    font-size: 12px;
    color: var(--text-faint);
  }
  .shape {
    display: grid;
    place-items: center;
    width: 160px;
    height: 72px;
    padding: 8px;
    text-align: center;
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    font-size: 12px;
    color: var(--text-muted);
  }
  .r-tile {
    border-radius: var(--radius-tile);
  }
  .r-md {
    border-radius: var(--radius-md);
  }
  .r-lg {
    border-radius: var(--radius-lg);
  }
  .shape.cut {
    border: 0;
    background: color-mix(in oklab, var(--accent) 18%, var(--surface-2));
  }
  .type {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .type p {
    margin: 0;
  }
  .t-display {
    font-family: var(--font-display);
    font-size: 30px;
  }
  .t-body {
    font-size: 15px;
    max-width: 640px;
  }
  .t-mono {
    font-family: var(--font-mono);
    font-weight: 600;
    font-size: 22px;
    color: var(--accent);
  }
  .icons {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(118px, 1fr));
    gap: 6px;
  }
  .ic {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px;
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: 11.5px;
  }
  .ic span {
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ic:hover {
    background: var(--surface-2);
  }
  .states {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 14px;
  }

  /* Пример главного экрана */
  .app {
    display: grid;
    grid-template-columns: 220px 1fr;
    grid-template-rows: auto 1fr auto;
    height: 640px;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: var(--bg);
  }
  :global(.app-top) {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 10px 12px 10px 16px;
  }
  .app-side {
    padding: 12px 8px;
    background: var(--surface-1);
    border-right: 1px solid var(--line);
    overflow: auto;
  }
  .app-main {
    padding: 16px;
    overflow: auto;
  }
  .app-bottom {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 10px 16px;
    background: var(--surface-1);
    border-top: 1px solid var(--line);
  }
  .credits {
    margin-top: 32px;
    text-align: center;
    font-family: var(--font-display);
    font-size: 12px;
    letter-spacing: 0.12em;
    color: var(--text-faint);
  }
  .heart {
    color: var(--magenta);
  }
</style>
<Toaster offset={24} />
