<script lang="ts">
  import { ripple } from '$lib/motion';

  interface Option<T> {
    value: T;
    label: string;
    icon?: any;
  }

  interface Props<T> {
    options: Option<T>[];
    value: T;
    onchange: (val: T) => void;
    class?: string;
    compact?: boolean;
    tabWidth?: number;
  }

  let {
    options,
    value,
    onchange,
    class: extraClass = '',
    compact = false
  }: Props<any> = $props();

  let buttons = $state<HTMLButtonElement[]>([]);
  let activeIndex = $derived(options.findIndex((opt) => opt.value === value));

  let sliderStyle = $derived.by(() => {
    if (activeIndex === -1 || !buttons[activeIndex]) {
      return '';
    }
    const btn = buttons[activeIndex];
    return `left: ${btn.offsetLeft}px; width: ${btn.offsetWidth}px;`;
  });
</script>

<div
  class="segmented-control-container {extraClass}"
  class:compact
>
  {#if activeIndex !== -1 && buttons[activeIndex]}
    <div
      class="active-slider"
      style={sliderStyle}
    >
      <div class="active-pill"></div>
    </div>
  {/if}

  {#each options as opt, idx}
    {@const isActive = opt.value === value}
    <button
      bind:this={buttons[idx]}
      type="button"
      use:ripple
      class="tab-btn"
      class:active={isActive}
      onclick={() => onchange(opt.value)}
      aria-pressed={isActive}
    >
      {#if opt.icon}
        <opt.icon class="w-[18px] h-[18px] shrink-0" />
      {/if}
      <span>{opt.label}</span>
    </button>
  {/each}
</div>

<style>
  .segmented-control-container {
    position: relative;
    display: inline-flex;
    align-items: center;
    background: var(--bg-card);
    border: var(--border-width) solid var(--border-color);
    padding: 2px;
    border-radius: var(--radius-full);
    height: 46px;
    user-select: none;
    overflow: hidden;
    max-width: 100%;
    width: max-content;
    min-width: 0;
  }

  .active-slider {
    position: absolute;
    top: 2px;
    bottom: 2px;
    z-index: 1;
    transition: left 240ms cubic-bezier(0.16, 1, 0.3, 1), width 240ms cubic-bezier(0.16, 1, 0.3, 1);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    padding: 0 2px;
    box-sizing: border-box;
  }

  .active-pill {
    width: 100%;
    height: 100%;
    border-radius: var(--radius-full);
    background: rgba(255, 255, 255, 0.1);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
  }

  .tab-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    border: none;
    outline: none;
    border-radius: var(--radius-full);
    background: transparent;
    font-size: 13.5px;
    font-weight: var(--font-weight-normal);
    font-family: var(--font-sans);
    color: var(--text-secondary);
    cursor: pointer;
    z-index: 2;
    transition: color var(--duration-fast) var(--ease-expo);
    min-width: 0;
    padding-inline: 18px;
    padding-bottom: 2px;
    line-height: normal;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .tab-btn span {
    min-width: 0;
    white-space: nowrap;
    line-height: normal;
  }

  .tab-btn :global(svg) {
    opacity: 0.6;
    transition: opacity var(--duration-fast) var(--ease-expo);
  }

  .tab-btn:hover {
    color: var(--text-primary);
  }

  .tab-btn:hover :global(svg) {
    opacity: 0.9;
  }

  .tab-btn.active {
    color: var(--text-primary);
    font-weight: var(--font-weight-medium);
  }

  .tab-btn.active :global(svg) {
    opacity: 1;
  }
</style>
