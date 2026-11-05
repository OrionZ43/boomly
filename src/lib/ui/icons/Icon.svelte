<script lang="ts">
  // Единственный набор иконок Boomly — Lucide (контур 2px, 24×24). Список: ./paths.ts.
  import { icons, type IconName } from "./paths";

  interface Props {
    name: IconName;
    size?: number;
    strokeWidth?: number;
    /** Подпись для скринридера. Без неё иконка декоративная (aria-hidden). */
    label?: string;
    class?: string;
  }
  let { name, size = 18, strokeWidth = 2, label, class: klass = "" }: Props = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  class="shrink-0 {klass}"
  role={label ? "img" : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : "true"}
>
  {@html icons[name]}
</svg>
