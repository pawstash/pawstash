<script lang="ts">
  import { ripple } from '$lib/motion/ripple';
  import type { Snippet } from 'svelte';

  interface Props {
    quadrants: [string, string, string, string];
    active?: boolean;
    label: string;
    size?: number;
    indicatorColor?: string;
    onclick?: () => void;
    children?: Snippet;
  }

  let {
    quadrants,
    active = false,
    label,
    size = 26,
    indicatorColor,
    onclick,
    children
  }: Props = $props();

  const ringColor = $derived(indicatorColor || quadrants[0]);
</script>

<button
  type="button"
  use:ripple
  {onclick}
  class="palette-circle relative rounded-full transition-all duration-300 flex items-center justify-center cursor-pointer select-none p-0 overflow-visible hover:scale-115 active:scale-90"
  style="
    width: {size}px;
    height: {size}px;
    transform: {active ? 'scale(1.15)' : 'scale(1)'};
    box-shadow: {active ? `0 0 0 2px ${ringColor}` : 'none'};
  "
  aria-label={label}
  aria-pressed={active}
>
  <div
    class="w-full h-full rounded-full overflow-hidden grid grid-cols-2 grid-rows-2 gap-[0.5px] bg-[#111215]/40 transition-transform duration-200"
    style="
      border: {active ? '2px solid #111215' : '1px solid rgba(255, 255, 255, 0.12)'};
    "
  >
    <div style="background-color: {quadrants[0]};"></div>
    <div style="background-color: {quadrants[1]};"></div>
    <div style="background-color: {quadrants[2]};"></div>
    <div style="background-color: {quadrants[3]};"></div>
  </div>

  {#if children}
    <div class="absolute inset-0 flex items-center justify-center pointer-events-none z-10">
      {@render children()}
    </div>
  {/if}
</button>

<style>
  .palette-circle {
    outline: none;
    background: transparent;
    border: none;
  }
</style>
