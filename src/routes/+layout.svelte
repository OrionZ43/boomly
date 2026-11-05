<script lang="ts">
  // Владелец: A. Общая обёртка: стили и автопроверка обновлений (участок E).
  // D: загрузка состояния приложения и тосты — общие для всех экранов.
  import { onMount } from "svelte";
  import { startAutoCheck } from "$lib/updater";
  import { app } from "$lib/stores/app.svelte";
  import { Toaster } from "$lib/ui";
  import "../app.css";

  let { children } = $props();

  onMount(() => {
    app.init();
    const stop = startAutoCheck();
    return () => {
      stop();
      app.destroy();
    };
  });
</script>

{@render children()}
<Toaster offset={84} />
