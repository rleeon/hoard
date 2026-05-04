<script lang="ts">
  /**
   * App wordmark — a stylised treasure-chest glyph next to "Hoard".
   * Single SVG, no external assets, scales cleanly. Honours `currentColor`
   * for the chest body so callers can tint it amber on dark backgrounds.
   */
  type Props = {
    size?: number;
    showWordmark?: boolean;
    class?: string;
  };

  let {
    size = 56,
    showWordmark = true,
    class: extraClass = "",
  }: Props = $props();
</script>

<div class={`flex items-center gap-3 ${extraClass}`}>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 64 64"
    width={size}
    height={size}
    aria-hidden="true"
    role="img"
  >
    <!-- Chest base -->
    <rect
      x="8"
      y="26"
      width="48"
      height="28"
      rx="3"
      fill="currentColor"
      opacity="0.18"
    />
    <rect
      x="8"
      y="26"
      width="48"
      height="28"
      rx="3"
      fill="none"
      stroke="currentColor"
      stroke-width="2.5"
    />
    <!-- Lid -->
    <path
      d="M8 30 Q8 14 32 14 Q56 14 56 30"
      fill="currentColor"
      opacity="0.28"
    />
    <path
      d="M8 30 Q8 14 32 14 Q56 14 56 30"
      fill="none"
      stroke="currentColor"
      stroke-width="2.5"
    />
    <!-- Lid seam -->
    <line
      x1="8"
      y1="30"
      x2="56"
      y2="30"
      stroke="currentColor"
      stroke-width="2"
    />
    <!-- Lock -->
    <rect
      x="28"
      y="34"
      width="8"
      height="10"
      rx="1.5"
      fill="currentColor"
      opacity="0.7"
    />
    <circle cx="32" cy="38" r="1.6" fill="#18181b" />
  </svg>

  {#if showWordmark}
    <div class="flex flex-col leading-none">
      <span class="text-2xl font-semibold tracking-tight text-zinc-50">
        Hoard
      </span>
      <span class="text-xs uppercase tracking-[0.18em] text-zinc-500">
        Save sync
      </span>
    </div>
  {/if}
</div>
