<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    cardWidth: number;
    gap: number;
    ratio: number;
    rows?: number;
    fading?: boolean;
  }

  let { cardWidth, gap, ratio, rows, fading = false }: Props = $props();

  const SK_TILT = Math.tan(Math.PI / 12);
  const SK_SIN = Math.sin((105 * Math.PI) / 180);
  const MAX_ROWS = 12;

  let root = $state<HTMLDivElement>();
  let width = $state(0);
  let available = $state(0);
  let observer: ResizeObserver | undefined;

  function measure() {
    if (!root) return;
    width = root.clientWidth;
    available = Math.max(240, window.innerHeight - root.getBoundingClientRect().top);
  }

  onMount(() => {
    measure();
    observer = new ResizeObserver(measure);
    if (root) observer.observe(root);
    window.addEventListener('resize', measure);
  });

  onDestroy(() => {
    observer?.disconnect();
    if (typeof window !== 'undefined') window.removeEventListener('resize', measure);
  });

  let columns = $derived(Math.max(1, Math.floor((width + gap) / (cardWidth + gap))));
  let cellWidth = $derived(columns > 0 ? (width - gap * (columns - 1)) / columns : width);
  let cellHeight = $derived(Math.round(cellWidth / ratio));
  let stride = $derived(cellHeight + gap);
  let rowCount = $derived(
    rows ?? (stride > 0 ? Math.min(MAX_ROWS, Math.max(2, Math.ceil(available / stride) + 1)) : 0)
  );
  let canvasHeight = $derived(Math.max(0, rowCount * cellHeight + (rowCount - 1) * gap));
  let band = $derived(Math.max(90, width * 0.2));
  let reach = $derived(width / 2 + (canvasHeight * SK_TILT) / 2 + band / SK_SIN);

  let cells = $derived.by(() => {
    if (width <= 0 || stride <= 0 || cellHeight <= 0) return [];
    return Array.from({ length: rowCount * columns }, (_, index) => ({
      index,
      x: (index % columns) * (cellWidth + gap),
      y: Math.floor(index / columns) * stride
    }));
  });
</script>

<div
  bind:this={root}
  class="skeleton-grid"
  class:is-fading={fading}
  aria-hidden="true"
  style={`--sk-cols: ${columns}; --sk-w: ${width}px; --sk-h: ${canvasHeight}px; --sk-band: ${band}px; --sk-reach: ${reach}px; --sk-gap: ${gap}px; --sk-ratio: ${ratio};`}
>
  {#each cells as cell (cell.index)}
    <div class="skeleton-card" style={`--sk-x: ${cell.x}; --sk-y: ${cell.y};`}>
      <div class="sk-footer">
        <span class="sk-line sk-title"></span>
        <span class="sk-line sk-title sk-title-tail"></span>
        <span class="sk-line sk-author"></span>
        <span class="sk-line sk-meta"></span>
      </div>
    </div>
  {/each}
</div>

<style>
  .skeleton-grid {
    display: grid;
    grid-template-columns: repeat(var(--sk-cols, 1), minmax(0, 1fr));
    gap: var(--sk-gap);
    align-items: start;
    width: 100%;
    animation:
      sk-sweep 2.4s cubic-bezier(.42, 0, .32, 1) infinite,
      sk-appear var(--duration-normal) var(--ease-expo);
  }

  .skeleton-grid.is-fading {
    position: absolute;
    inset: 0 0 auto 0;
    opacity: 0;
    pointer-events: none;
    transition: opacity 220ms var(--ease-expo);
  }

  @keyframes sk-sweep {
    0% { --sk-pos: calc(-1 * var(--sk-reach)); }
    72% { --sk-pos: var(--sk-reach); }
    100% { --sk-pos: var(--sk-reach); }
  }

  @keyframes sk-appear {
    from { opacity: 0; }
  }

  .skeleton-card {
    position: relative;
    aspect-ratio: var(--sk-ratio);
    background-color: rgba(var(--surface-tint-rgb), .035);
    background-image: linear-gradient(
      105deg,
      transparent calc(50% - var(--sk-band)),
      rgba(var(--surface-tint-rgb), .012) calc(50% - var(--sk-band) * .7),
      rgba(var(--surface-tint-rgb), .038) calc(50% - var(--sk-band) * .35),
      rgba(var(--surface-tint-rgb), .072) 50%,
      rgba(var(--surface-tint-rgb), .038) calc(50% + var(--sk-band) * .35),
      rgba(var(--surface-tint-rgb), .012) calc(50% + var(--sk-band) * .7),
      transparent calc(50% + var(--sk-band))
    );
    background-repeat: no-repeat;
    background-size: var(--sk-w) var(--sk-h);
    background-position:
      calc(var(--sk-pos, -99999px) - var(--sk-x, 0) * 1px)
      calc(var(--sk-y, 0) * -1px);
  }

  .sk-footer {
    position: absolute;
    right: calc(12px * var(--grid-scale, 1));
    bottom: calc(12px * var(--grid-scale, 1));
    left: calc(12px * var(--grid-scale, 1));
    display: flex;
    flex-direction: column;
    gap: calc(5px * var(--grid-scale, 1));
  }

  .sk-line { display: block; border-radius: calc(3px * var(--grid-scale, 1)); background: rgba(var(--surface-tint-rgb), .06); }
  .sk-title { width: 86%; height: calc(10px * var(--grid-scale, 1)); }
  .sk-title-tail { width: 54%; }
  .sk-author { width: 42%; height: calc(8px * var(--grid-scale, 1)); margin-top: calc(2px * var(--grid-scale, 1)); }
  .sk-meta { width: 28%; height: calc(7px * var(--grid-scale, 1)); background: rgba(var(--surface-tint-rgb), .045); }

  @media (prefers-reduced-motion: reduce) {
    .skeleton-grid { animation: none; }
  }
</style>
