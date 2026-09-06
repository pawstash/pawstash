<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ripple } from '$lib/motion';
  import StableWeightLabel from './StableWeightLabel.svelte';
  import CountBadge from './CountBadge.svelte';
  import IconChevronDown from '~icons/fluent/chevron-down-20-regular';

  export interface ChoiceOption<T> {
    value: T;
    label: string;
    icon?: any;
    count?: number | string | null;
    showZero?: boolean;
    chevron?: boolean;
    chevronOpen?: boolean;
    title?: string;
  }

  type Option<T> = ChoiceOption<T>;

  interface Props<T> {
    size?: 'sm' | 'base' | 'md' | 'lg';
    options: ChoiceOption<T>[];
    value: T;
    onchange: (value: T) => void;
    onclick?: (value: T, index: number, buttonEl: HTMLButtonElement, wasActive: boolean, event?: MouseEvent | KeyboardEvent) => void;
    ariaLabel?: string;
    class?: string;
    compact?: boolean;
    tabWidth?: number;
    align?: 'left' | 'right' | 'center';
    wrap?: boolean;
    buttons?: HTMLButtonElement[];
    activeAddon?: Snippet<[{ option: ChoiceOption<T> }]>;
    hasActiveAddon?: (option: ChoiceOption<T>) => boolean;
  }

  let {
    options,
    value,
    onchange,
    onclick,
    ariaLabel,
    class: extraClass = '',
    compact = false,
    tabWidth,
    size,
    align = 'right',
    wrap = false,
    buttons = $bindable([]),
    activeAddon,
    hasActiveAddon
  }: Props<any> = $props();

  let activeIndex = $derived(options.findIndex((option) => option.value === value));
  let accessibleLabel = $derived(ariaLabel || options.map((option) => option.label).join(', '));

  function select(index: number, event?: MouseEvent | KeyboardEvent) {
    const option = options[index];
    if (!option) return;
    const wasActive = option.value === value;
    onchange(option.value);
    onclick?.(option.value, index, buttons[index], wasActive, event);
    buttons[index]?.focus();
  }

  function handleKeydown(event: KeyboardEvent, index: number) {
    let nextIndex = index;

    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
      nextIndex = (index + 1) % options.length;
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
      nextIndex = (index - 1 + options.length) % options.length;
    } else if (event.key === 'Home') {
      nextIndex = 0;
    } else if (event.key === 'End') {
      nextIndex = options.length - 1;
    } else {
      return;
    }

    event.preventDefault();
    select(nextIndex);
  }
</script>

<div
  class="choice-group {size ? `size-${size}` : ''} align-{align} {extraClass}"
  class:compact
  class:is-wrap={wrap}
  role="radiogroup"
  aria-label={accessibleLabel}
  style:--choice-tab-width={tabWidth ? `${tabWidth}px` : undefined}
>
  {#each options as option, index}
    {@const isActive = option.value === value}
    {@const canHaveAddon = Boolean(activeAddon) && (hasActiveAddon ? hasActiveAddon(option) : true)}
    <div class="choice-group__segment">
      <button
        bind:this={buttons[index]}
        type="button"
        use:ripple
        class="choice-group__option"
        class:is-active={isActive}
        class:has-active-addon={isActive && canHaveAddon}
        role="radio"
        aria-checked={isActive}
        title={option.title}
        tabindex={isActive || (activeIndex === -1 && index === 0) ? 0 : -1}
        data-choice-value={String(option.value)}
        onclick={(event) => select(index, event)}
        onkeydown={(event) => handleKeydown(event, index)}
      >
        {#if option.icon}
          <option.icon />
        {/if}
        <StableWeightLabel text={option.label} reserveWeight="var(--font-weight-semibold)" />
        {#if option.count !== undefined && option.count !== null}
          <CountBadge count={option.count} showZero={option.showZero} />
        {/if}
        {#if option.chevron}
          <IconChevronDown class="choice-group__chevron {option.chevronOpen ? 'is-open' : ''}" />
        {/if}
      </button>

      {#if canHaveAddon && activeAddon}
        <div
          class="choice-group__addon-slot"
          class:is-expanded={isActive}
          inert={!isActive}
          aria-hidden={!isActive}
        >
          <div class="choice-group__addon-inner">
            {@render activeAddon({ option })}
          </div>
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .choice-group {
    --choice-option-height: calc(var(--control-height, 34px) * var(--ui-scale, 1));
    --choice-option-font-size: calc(var(--control-font-size, 13px) * var(--ui-scale, 1));
    --choice-option-icon-size: calc(var(--control-icon-size, 16px) * var(--ui-scale, 1));
    --choice-option-padding: calc(var(--control-padding-x, 14px) * var(--ui-scale, 1));

    display: inline-flex;
    align-items: stretch;
    justify-content: flex-end;
    flex-wrap: nowrap;
    min-width: 0;
    gap: calc(4px * var(--ui-scale, 1)) calc(2px * var(--ui-scale, 1));
    box-sizing: border-box;
    user-select: none;
    white-space: nowrap;
  }

  .choice-group.is-wrap {
    flex-wrap: wrap;
    max-width: 100%;
  }

  .choice-group.align-left {
    justify-content: flex-start;
  }

  .choice-group.align-center {
    justify-content: center;
  }

  .choice-group.align-right {
    justify-content: flex-end;
  }

  .choice-group.size-sm {
    --control-height: var(--control-height-sm, 34px);
    --control-font-size: var(--control-font-sm, 12.5px);
    --control-icon-size: var(--control-icon-sm, 16px);
    --control-padding-x: var(--control-padding-sm, 12px);
  }

  .choice-group.size-base {
    --control-height: var(--control-height-base, 40px);
    --control-font-size: var(--control-font-base, 13.5px);
    --control-icon-size: var(--control-icon-base, 18px);
    --control-padding-x: var(--control-padding-base, 14px);
  }

  .choice-group.size-md {
    --control-height: var(--control-height-md, 46px);
    --control-font-size: var(--control-font-md, 14px);
    --control-icon-size: var(--control-icon-md, 20px);
    --control-padding-x: var(--control-padding-md, 16px);
  }

  .choice-group.size-lg {
    --control-height: var(--control-height-lg, 52px);
    --control-font-size: var(--control-font-lg, 15px);
    --control-icon-size: var(--control-icon-lg, 22px);
    --control-padding-x: var(--control-padding-lg, 20px);
  }

  .choice-group__option {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: var(--choice-option-height);
    width: var(--choice-tab-width, auto);
    height: var(--choice-option-height);
    gap: calc(6px * var(--ui-scale, 1));
    padding: 0 var(--choice-option-padding);
    border: 0;
    border-radius: calc(var(--radius-sm) * var(--ui-scale, 1));
    background: var(--input-bg, rgba(255, 255, 255, 0.06));
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-size: var(--choice-option-font-size);
    font-weight: var(--font-weight-normal);
    line-height: normal;
    white-space: nowrap;
    cursor: pointer;
    flex-shrink: 0;
    transition:
      background var(--duration-fast) var(--ease-expo),
      color var(--duration-fast) var(--ease-expo),
      border-radius var(--duration-normal, 240ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
      transform var(--duration-fast) var(--ease-expo);
  }

  .choice-group__segment {
    display: inline-flex;
    align-items: stretch;
    flex-shrink: 0;
    max-width: 100%;
  }

  .choice-group__addon-slot {
    display: inline-flex;
    align-items: stretch;
    overflow: hidden;
    max-width: 0;
    margin-left: 0;
    opacity: 0;
    transform: scale(0.85);
    transform-origin: left center;
    pointer-events: none;
    visibility: hidden;
    transition:
      max-width var(--duration-normal, 240ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
      margin-left var(--duration-normal, 240ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
      opacity var(--duration-fast, 150ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
      transform var(--duration-normal, 240ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
      visibility 0s linear var(--duration-normal, 240ms);
    will-change: max-width, margin-left, opacity, transform;
  }

  .choice-group__addon-slot.is-expanded {
    max-width: calc(var(--choice-option-height, 46px) + 16px);
    margin-left: calc(2px * var(--ui-scale, 1));
    opacity: 1;
    transform: scale(1);
    pointer-events: auto;
    visibility: visible;
    transition-delay: 0s;
  }

  .choice-group__addon-inner {
    display: inline-flex;
    align-items: stretch;
    height: 100%;
    flex-shrink: 0;
  }


  .choice-group__option:hover:not(.is-active) {
    background: var(--input-bg-hover, rgba(255, 255, 255, 0.095));
    color: var(--text-primary);
  }

  .choice-group__option:active {
    transform: scale(0.98);
  }

  .choice-group__option:focus-visible {
    outline: calc(1.5px * var(--ui-scale, 1)) solid var(--accent-primary) !important;
    outline-offset: calc(-1.5px * var(--ui-scale, 1)) !important;
    box-shadow: none !important;
    z-index: 1;
  }

  .choice-group__option.is-active {
    border-radius: min(
      calc(var(--radius-full) * var(--ui-scale, 1)),
      calc(var(--choice-option-height) / 2)
    );
    background: var(--choice-active-bg, var(--accent-primary));
    color: var(--choice-active-text, var(--text-on-accent, #ffffff));
    font-weight: var(--font-weight-semibold);
  }

  .choice-group__option.is-active.has-active-addon {
    border-radius: min(
      calc(var(--radius-full) * var(--ui-scale, 1)),
      calc(var(--choice-option-height) / 2)
    ) calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) min(
      calc(var(--radius-full) * var(--ui-scale, 1)),
      calc(var(--choice-option-height) / 2)
    ) !important;
    border-top-right-radius: calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) !important;
    border-bottom-right-radius: calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) !important;
  }

  .choice-group__option :global(svg) {
    width: var(--choice-option-icon-size);
    height: var(--choice-option-icon-size);
    flex: none;
    opacity: 0.72;
    transition:
      opacity var(--duration-fast) var(--ease-expo),
      transform var(--duration-fast) var(--ease-expo);
  }

  .choice-group__option:hover:not(.is-active) :global(svg) {
    opacity: 1;
  }

  .choice-group__option.is-active :global(svg) {
    color: var(--choice-active-text, currentColor);
    opacity: 1;
  }

  .choice-group__chevron {
    width: calc(var(--choice-option-icon-size, 16px) * 0.85) !important;
    height: calc(var(--choice-option-icon-size, 16px) * 0.85) !important;
    margin-left: calc(-2px * var(--ui-scale, 1));
    flex-shrink: 0;
    opacity: 0.75;
  }

  .choice-group__chevron.is-open {
    transform: rotate(180deg);
  }

  .choice-group__option :global(.count-badge) {
    font-size: calc(12px * var(--ui-scale, 1));
    transition:
      background var(--duration-fast) var(--ease-expo),
      color var(--duration-fast) var(--ease-expo);
  }

  .choice-group__option.is-active :global(.count-badge) {
    background: color-mix(in srgb, currentColor 22%, transparent);
    color: inherit;
  }

  .choice-group.compact .choice-group__option {
    padding-inline: 14px;
  }

  :global([data-layout='mobile']) .choice-group:not(.align-left) {
    width: 100%;
    justify-content: stretch;
  }

  :global([data-layout='mobile']) .choice-group:not(.align-left) .choice-group__segment {
    flex: 1 0 auto;
  }

  :global([data-layout='mobile']) .choice-group:not(.align-left) .choice-group__option {
    flex: 1 0 auto;
    width: 100%;
  }

  @media (max-width: 640px), (hover: none) {
    .choice-group:not(.align-left) {
      width: 100%;
      justify-content: stretch;
    }

    .choice-group:not(.align-left) .choice-group__segment {
      flex: 1 0 auto;
    }

    .choice-group:not(.align-left) .choice-group__option {
      flex: 1 0 auto;
      width: 100%;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .choice-group__option,
    .choice-group__addon-slot {
      transition: none !important;
    }
  }
</style>
