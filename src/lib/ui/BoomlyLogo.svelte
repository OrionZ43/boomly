<script lang="ts">
  // Логотип Boomly: знак (B + звуковые волны + жёлтая искра) и надпись Russo One с градиентом бренда.
  // Исходники: ./brand/boomly-icon.svg (иконка приложения), ./brand/boomly-mark.svg (знак).
  interface Props {
    /** Высота знака, px. */
    size?: number;
    /** Показать надпись BOOMLY. */
    wordmark?: boolean;
    /** Знак на срезанной подложке, как иконка приложения. */
    badge?: boolean;
    class?: string;
  }
  let { size = 28, wordmark = true, badge = false, class: klass = "" }: Props = $props();
  const uid = $props.id();
  const g = `bm-g-${uid}`;
</script>

<span class="logo {klass}" style:--h="{size}px">
  <svg
    viewBox={badge ? "0 0 512 512" : "96 84 376 300"}
    height={size}
    width={badge ? size : (size * 376) / 300}
    aria-hidden={wordmark ? "true" : undefined}
    role={wordmark ? undefined : "img"}
    aria-label={wordmark ? undefined : "Boomly"}
  >
    <defs>
      <linearGradient id={g} x1="80" y1="80" x2="440" y2="440" gradientUnits="userSpaceOnUse">
        <stop offset="0" stop-color="#00f0ff" />
        <stop offset="1" stop-color="#ff00c1" />
      </linearGradient>
    </defs>
    {#if badge}
      <path d="M120 20H492V392L392 492H20V120Z" fill="#0d0e16" />
      <path d="M124 32H480V387L387 480H32V124Z" fill="none" stroke="url(#{g})" stroke-width="18" />
    {/if}
    <g transform="translate(-22 0)">
      <path
        fill="url(#{g})"
        fill-rule="evenodd"
        d="M156 140H240C284 140 310 163 310 198C310 222 298 240 278 249C306 259 322 281 322 313C322 352 292 376 248 376H128V168ZM180 184H232C248 184 258 193 258 206C258 219 248 228 232 228H180ZM180 270H240C258 270 270 281 270 298C270 315 258 330 240 330H180Z"
      />
      <path fill="none" stroke="url(#{g})" stroke-width="28" stroke-linecap="round" d="M356 214A62 62 0 0 1 356 298" />
      <path fill="none" stroke="url(#{g})" stroke-width="28" stroke-linecap="round" d="M396 172A118 118 0 0 1 396 340" />
    </g>
    <path fill="#fcee0a" d="M424 92L434 122L464 132L434 142L424 172L414 142L384 132L414 122Z" />
  </svg>
  {#if wordmark}<span class="word">BOOMLY</span>{/if}
</span>

<style>
  .logo {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--h) * 0.32);
  }
  .word {
    font-family: var(--font-display);
    font-size: calc(var(--h) * 0.78);
    line-height: 1;
    letter-spacing: 0.04em;
    background: var(--gradient-brand);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
</style>
