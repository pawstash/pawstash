<script lang="ts">
  interface Props {
    value: number;
    min?: number;
    max?: number;
    step?: number;
    class?: string;
    oninput?: (val: number) => void;
  }

  let {
    value = $bindable(),
    min = 0,
    max = 100,
    step = 1,
    class: extraClass = '',
    oninput
  }: Props = $props();

  let percent = $derived(((value - min) / (max - min)) * 100);
  let isDragging = $state(false);
</script>

<div class="m3-slider {extraClass}" class:dragging={isDragging}>
  <div class="slider-track">
    <div class="track-active" style="width: {percent}%;"></div>
    <div class="track-inactive" style="width: {100 - percent}%;"></div>
  </div>

  <input
    type="range"
    {min}
    {max}
    {step}
    bind:value
    oninput={() => oninput?.(value)}
    onpointerdown={() => (isDragging = true)}
    onpointerup={() => (isDragging = false)}
    onlostpointercapture={() => (isDragging = false)}
    class="slider-native"
  />

  <div
    class="slider-thumb"
    class:active={isDragging}
    style="left: {percent}%;"
  >
    <div class="thumb-ripple"></div>
    <div class="thumb-dot"></div>
  </div>
</div>

<style>
  .m3-slider {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
    height: 46px;
    cursor: pointer;
    touch-action: none;
  }

  .slider-track {
    position: absolute;
    left: 0;
    right: 0;
    height: 6px;
    border-radius: 9999px;
    display: flex;
    overflow: hidden;
    pointer-events: none;
  }

  .track-active {
    height: 100%;
    background: #ffffff;
    border-radius: 9999px 0 0 9999px;
    margin-right: 1px;
  }

  .track-inactive {
    height: 100%;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 0 9999px 9999px 0;
    margin-left: 1px;
  }

  .slider-native {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: pointer;
    z-index: 3;
    -webkit-appearance: none;
    appearance: none;
  }

  .slider-thumb {
    position: absolute;
    top: 50%;
    width: 0;
    height: 0;
    transform: translate(-50%, -50%);
    pointer-events: none;
    z-index: 2;
  }

  .thumb-dot {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #ffffff;
    transform: translate(-50%, -50%) scale(1);
    transition: transform 200ms cubic-bezier(0.2, 0, 0, 1),
                box-shadow 200ms cubic-bezier(0.2, 0, 0, 1);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  .m3-slider:hover .thumb-dot {
    transform: translate(-50%, -50%) scale(1.1);
  }

  .slider-thumb.active .thumb-dot {
    transform: translate(-50%, -50%) scale(1.05);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  }

  .thumb-ripple {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0);
    transform: translate(-50%, -50%);
    transition: background 200ms ease;
    pointer-events: none;
  }

  .m3-slider:hover .thumb-ripple {
    background: rgba(255, 255, 255, 0.08);
  }

  .slider-thumb.active .thumb-ripple {
    background: rgba(255, 255, 255, 0.14);
  }

  .slider-native:focus-visible ~ .slider-thumb .thumb-dot {
    box-shadow: 0 0 0 2px #14161a, 0 0 0 4px var(--accent-primary, #ffffff);
  }
</style>
