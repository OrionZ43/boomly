<script lang="ts">
  // Владелец: A. Общая обёртка: стили и автопроверка обновлений (участок E).
  // Оболочку экранов (навигация, UpdateBanner, Toaster) делает D в своих компонентах.
  import { onMount } from "svelte";
  import { startAutoCheck } from "$lib/updater";
  import "../app.css";

  let { children } = $props();

  onMount(() => startAutoCheck());
</script>

{@render children()}
