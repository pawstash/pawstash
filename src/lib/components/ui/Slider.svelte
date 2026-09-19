<script lang="ts">
  import { tooltip } from '$lib/motion/tooltip';

  interface Props {
    size?: 'sm' | 'base' | 'md' | 'lg';
    value: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    stops?: boolean;
    ticks?: boolean;
    ariaLabel?: string;
    class?: string;
    formatValue?: (val: number) => string;
    oninput?: (val: number) => void;
    onchange?: (val: number) => void;
  }

  let {
    value = $bindable(),
    min = 0,
    max = 100,
    step = 1,
    size,
    disabled = false,
    stops = true,
    ticks = false,
    ariaLabel,
    class: extraClass = '',
    formatValue,
    oninput,
    onchange
  }: Props = $props();

  let rootEl: HTMLDivElement | undefined = $state();
  let travelEl: HTMLDivElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();
  let travelWidth = $state(0);
  let trackHeight = $state(0);
  let isPressed = $state(false);
  let isScrubbing = $state(false);
  let isHovered = $state(false);
  let isKeyboardFocused = $state(false);

  let stepDecimals = $derived.by(() => {
    const decimals = String(step).split('.')[1];
    return decimals ? decimals.length : 0;
  });

  let fraction = $derived(max > min ? Math.min(1, Math.max(0, (value - min) / (max - min))) : 0);

  let tickFractions = $derived.by(() => {
    if (!ticks || !(step > 0) || max <= min || travelWidth <= 0) return [];
    const count = Math.round((max - min) / step) + 1;
    if (count < 2 || (count - 1) * (trackHeight * 0.75) > travelWidth) return [];
    return Array.from({ length: count }, (_, index) => index / (count - 1));
  });

  let showStartStop = $derived(
    stops && !tickFractions.length && fraction * travelWidth > trackHeight
  );
  let showEndStop = $derived(
    stops && !tickFractions.length && (1 - fraction) * travelWidth > trackHeight
  );

  let indicator = $derived({
    text: formatValue?.(value),
    placement: 'top' as const,
    variant: 'accent' as const,
    delay: isPressed || isKeyboardFocused ? 0 : 120,
    open: !disabled && (isPressed || isHovered || isKeyboardFocused)
  });

  function snap(raw: number): number {
    const bounded = Math.min(max, Math.max(min, raw));
    if (!(step > 0)) return bounded;
    const stepped = min + Math.round((bounded - min) / step) * step;
    return Number(Math.min(max, Math.max(min, stepped)).toFixed(stepDecimals));
  }

  function emit(next: number): boolean {
    if (next === value) return false;
    value = next;
    oninput?.(next);
    return true;
  }

  function trackPointer(clientX: number) {
    const rect = travelEl?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return;
    emit(snap(min + ((clientX - rect.left) / rect.width) * (max - min)));
  }

  function onPointerDown(event: PointerEvent) {
    if (disabled || !event.isPrimary || event.button !== 0) return;
    event.preventDefault();
    isPressed = true;
    rootEl?.setPointerCapture(event.pointerId);
    inputEl?.focus({ preventScroll: true });
    trackPointer(event.clientX);
  }

  function onPointerMove(event: PointerEvent) {
    if (!isPressed) return;
    isScrubbing = true;
    trackPointer(event.clientX);
  }

  function onPointerEnd(event: PointerEvent) {
    if (!isPressed) return;
    isPressed = false;
    isScrubbing = false;
    if (rootEl?.hasPointerCapture(event.pointerId)) rootEl.releasePointerCapture(event.pointerId);
    onchange?.(value);
  }

  function onNativeInput(event: Event) {
    const el = event.currentTarget as HTMLInputElement;
    if (!emit(snap(Number(el.value)))) el.value = String(value);
  }

  function onNativeChange() {
    if (isPressed) return;
    onchange?.(value);
  }

  function onNativeFocus(event: FocusEvent) {
    isKeyboardFocused = (event.currentTarget as HTMLElement).matches(':focus-visible');
  }
</script>

{#snippet tickRow()}
  {#if tickFractions.length}
    <div class="tick-row">
      {#each tickFractions as tickFraction (tickFraction)}
        <span class="stop-dot tick" style="left: {tickFraction * 100}%;"></span>
      {/each}
    </div>
  {/if}
{/snippet}

<div
  bind:this={rootEl}
  role="presentation"
  class="m3-slider {size ? `size-${size}` : ''} {extraClass}"
  class:pressed={isPressed}
  class:scrubbing={isScrubbing}
  class:disabled
  style="--slider-fraction: {fraction}; --travel-width: {travelWidth}px;"
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerEnd}
  onpointercancel={onPointerEnd}
  onpointerenter={() => (isHovered = true)}
  onpointerleave={() => (isHovered = false)}
>
  <input
    bind:this={inputEl}
    class="slider-native"
    type="range"
    {min}
    {max}
    {step}
    {disabled}
    {value}
    aria-label={ariaLabel}
    oninput={onNativeInput}
    onchange={onNativeChange}
    onfocus={onNativeFocus}
    onblur={() => (isKeyboardFocused = false)}
  />

  <div class="slider-track" bind:clientHeight={trackHeight}>
    <div class="track-segment track-active">
      <span class="stop-dot cap" class:visible={showStartStop}></span>
      {@render tickRow()}
    </div>
    <div class="track-segment track-inactive">
      <span class="stop-dot cap" class:visible={showEndStop}></span>
      {@render tickRow()}
    </div>

    <div class="slider-travel" bind:this={travelEl} bind:clientWidth={travelWidth}>
      <div class="slider-handle" use:tooltip={indicator}></div>
    </div>
  </div>
</div>

<style>
  .m3-slider {
    --track-height: calc(
      var(--control-height, 46px) * var(--slider-track-scale) * var(--ui-scale, 1)
    );
    --handle-width: calc(var(--slider-handle-width) * var(--ui-scale, 1));
    --handle-height: calc(
      var(--track-height) + var(--slider-handle-overhang) * 2 * var(--ui-scale, 1)
    );
    --track-gap: calc(var(--slider-gap) * var(--ui-scale, 1));
    --travel-inset: calc(var(--handle-width) / 2 + var(--track-gap));
    --inner-radius: calc(var(--slider-inner-radius) * var(--ui-scale, 1));
    --stop-size: calc(var(--slider-stop-size) * var(--ui-scale, 1));
    --stop-inset: calc((var(--track-height) - var(--stop-size)) / 2);

    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1));
    cursor: pointer;
    touch-action: none;
  }

  .m3-slider.size-sm {
    --control-height: var(--control-height-sm);
  }

  .m3-slider.size-base {
    --control-height: var(--control-height-base);
  }

  .m3-slider.size-md {
    --control-height: var(--control-height-md);
  }

  .m3-slider.size-lg {
    --control-height: var(--control-height-lg);
  }

  .m3-slider.disabled {
    cursor: not-allowed;
    opacity: var(--opacity-disabled);
  }

  .slider-track {
    position: relative;
    flex: 1;
    height: var(--track-height);
  }

  .track-segment {
    position: absolute;
    top: 0;
    bottom: 0;
    overflow: hidden;
    transition:
      left var(--duration-normal) var(--ease-expo),
      width var(--duration-normal) var(--ease-expo),
      background var(--duration-fast) linear;
  }

  .track-active {
    left: 0;
    width: max(0px, (100% - var(--travel-inset) * 2) * var(--slider-fraction));
    background: var(--slider-active);
    border-radius: var(--radius-full) var(--inner-radius) var(--inner-radius) var(--radius-full);
  }

  .track-inactive {
    left: calc(
      (100% - var(--travel-inset) * 2) * var(--slider-fraction) + var(--travel-inset) * 2
    );
    right: 0;
    background: var(--slider-inactive);
    border-radius: var(--inner-radius) var(--radius-full) var(--radius-full) var(--inner-radius);
  }

  .m3-slider:hover:not(.disabled) .track-inactive {
    background: var(--slider-inactive-hover);
  }

  .stop-dot {
    position: absolute;
    top: 50%;
    width: var(--stop-size);
    height: var(--stop-size);
    border-radius: var(--radius-full);
    transform: translateY(-50%);
  }

  .stop-dot.cap {
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-expo);
  }

  .stop-dot.cap.visible {
    opacity: 1;
  }

  .tick-row {
    position: absolute;
    top: 0;
    bottom: 0;
    width: var(--travel-width);
  }

  .track-active .tick-row {
    left: var(--travel-inset);
  }

  .track-inactive .tick-row {
    right: var(--travel-inset);
  }

  .stop-dot.tick {
    transform: translate(-50%, -50%);
  }

  .track-active .stop-dot.cap {
    left: var(--stop-inset);
  }

  .track-active .stop-dot {
    background: var(--slider-stop-on-active);
  }

  .track-inactive .stop-dot.cap {
    right: var(--stop-inset);
  }

  .track-inactive .stop-dot {
    background: var(--slider-stop-on-inactive);
  }

  .slider-travel {
    position: absolute;
    top: 0;
    bottom: 0;
    left: var(--travel-inset);
    right: var(--travel-inset);
    pointer-events: none;
  }

  .slider-handle {
    position: absolute;
    top: 50%;
    left: calc(var(--slider-fraction) * 100%);
    width: var(--handle-width);
    height: var(--handle-height);
    border-radius: var(--radius-full);
    background: var(--slider-handle);
    transform: translate(-50%, -50%);
    transition:
      left var(--duration-normal) var(--ease-expo),
      transform var(--duration-fast) var(--ease-expo);
  }

  .m3-slider.pressed .slider-handle {
    transform: translate(-50%, -50%) scaleX(var(--slider-handle-pressed-scale));
  }

  .m3-slider.scrubbing .track-segment,
  .m3-slider.scrubbing .slider-handle {
    transition: transform var(--duration-fast) var(--ease-expo);
  }

  .slider-native:focus-visible ~ .slider-track .slider-handle {
    outline: calc(var(--slider-focus-ring) * var(--ui-scale, 1)) solid var(--accent-primary);
    outline-offset: calc(var(--slider-focus-ring) * var(--ui-scale, 1));
  }

  .slider-native {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    pointer-events: none;
    -webkit-appearance: none;
    appearance: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .track-segment,
    .slider-handle,
    .stop-dot {
      transition: none;
    }
  }
</style>
