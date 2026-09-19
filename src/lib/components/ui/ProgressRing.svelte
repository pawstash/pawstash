<script lang="ts">
  interface Props {
    value?: number | null;
    size?: number;
    stroke?: number;
    variant?: 'standard' | 'segmented';
    class?: string;
  }

  let {
    value = null,
    size = 30,
    stroke = 2,
    variant = 'standard',
    class: extraClass = ''
  }: Props = $props();

  const RADIUS = 10.5;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

  let isDeterminate = $derived(value !== null && value !== undefined);
  let offset = $derived(
    !isDeterminate
      ? CIRCUMFERENCE * 0.75
      : CIRCUMFERENCE * (1 - Math.min(1, Math.max(0, value!)))
  );
</script>

<svg
  class="progress-ring {extraClass}"
  class:indeterminate={!isDeterminate}
  class:segmented={!isDeterminate && variant === 'segmented'}
  viewBox="0 0 24 24"
  style="--ring-size: {size}px; --ring-stroke: {stroke};"
  aria-hidden="true"
>
  <circle class="ring-track" cx="12" cy="12" r={RADIUS} />
  <circle
    class="ring-value"
    cx="12"
    cy="12"
    r={RADIUS}
    stroke-dasharray={!isDeterminate && variant === 'segmented' ? '3.8 2.8' : CIRCUMFERENCE}
    stroke-dashoffset={!isDeterminate && variant === 'segmented' ? 0 : offset}
  />
</svg>

<style>
  .progress-ring {
    position: absolute;
    top: 50%;
    left: 50%;
    width: calc(var(--ring-size) * var(--ui-scale, 1));
    height: calc(var(--ring-size) * var(--ui-scale, 1));
    transform: translate(-50%, -50%);
    pointer-events: none;
    overflow: visible;
  }

  circle {
    fill: none;
    stroke-width: var(--ring-stroke);
    stroke-linecap: round;
  }

  .ring-track {
    stroke: var(--accent-container);
  }

  .ring-value {
    stroke: var(--accent-primary);
    transform: rotate(-90deg);
    transform-origin: center;
    transition: stroke-dashoffset var(--duration-normal) var(--ease-expo);
  }

  .indeterminate:not(.segmented) .ring-value {
    transition: none;
    animation: ring-spin 1.4s linear infinite;
  }

  .segmented .ring-track {
    stroke: rgba(var(--surface-tint-rgb), 0.08);
  }

  .segmented .ring-value {
    transition: none;
    animation: ring-spin 1.2s linear infinite;
  }

  @keyframes ring-spin {
    from {
      transform: rotate(-90deg);
    }
    to {
      transform: rotate(270deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .indeterminate .ring-value {
      animation: none;
      stroke-dashoffset: 0;
      opacity: 0.45;
    }
  }
</style>
