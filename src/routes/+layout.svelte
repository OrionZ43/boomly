<script lang="ts">
  // Владелец: A. Общая обёртка: стили. Оболочку экранов (навигация) делает D в своих компонентах.
  import "../app.css";
  let { children } = $props();
</script>

{@render children()}
