<script lang="ts">
  import { untrack } from 'svelte';
  import {
    interpolatePoints,
    materialShapePath,
    pointsToPath,
    shapePoints,
    type MaterialShapeName
  } from '$lib/utils/materialShapes';

  interface Props {
    name?: MaterialShapeName;
    size?: number;
    duration?: number;
    cycle?: MaterialShapeName[];
    cycleInterval?: number;
    spin?: boolean;
    spinDuration?: number;
    class?: string;
  }

  let {
    name = 'circle',
    size = 40,
    duration = 320,
    cycle,
    cycleInterval = 4200,
    spin = false,
    spinDuration = 28000,
    class: extraClass = ''
  }: Props = $props();

  let displayName = $state(untrack(() => name));
  let path = $state(untrack(() => materialShapePath(name)));
  let current = untrack(() => shapePoints(name));
  let frame = 0;
  let first = true;
  const reduced =
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  $effect(() => {
    const targetName = displayName;
    const target = shapePoints(targetName);

    if (first || reduced) {
      first = false;
      current = target;
      path = materialShapePath(targetName);
      return;
    }

    const from = current;
    const start = performance.now();
    cancelAnimationFrame(frame);
    const step = (now: number) => {
      const progress = Math.min(1, (now - start) / duration);
      const eased = 1 - (1 - progress) ** 3;
      current = interpolatePoints(from, target, eased);
      path = pointsToPath(current);
      if (progress < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  });

  $effect(() => {
    if (cycle && cycle.length > 0 && !reduced) return;
    displayName = name;
  });

  $effect(() => {
    if (!cycle || cycle.length === 0 || reduced) return;
    let index = cycle.indexOf(name);
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const tick = () => {
      timer = setTimeout(() => {
        if (cancelled) return;
        index = (index + 1) % cycle.length;
        displayName = cycle[index];
        tick();
      }, cycleInterval);
    };
    tick();
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  });
</script>

<svg
  class="material-shape {extraClass}"
  class:is-spinning={spin && !reduced}
  width={size}
  height={size}
  viewBox="0 0 100 100"
  style="--material-shape-spin-duration: {spinDuration}ms"
  aria-hidden="true"
  focusable="false"
>
  <path d={path} fill="currentColor" />
</svg>

<style>
  .material-shape {
    display: block;
    flex: none;
    pointer-events: none;
  }

  .material-shape.is-spinning {
    animation: material-shape-spin var(--material-shape-spin-duration) linear infinite;
    transform-box: view-box;
    transform-origin: center;
  }

  @keyframes material-shape-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .material-shape.is-spinning {
      animation: none;
    }
  }
</style>
