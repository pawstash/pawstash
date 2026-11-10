<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { ripple } from '$lib/motion';
  import StableWeightLabel from './StableWeightLabel.svelte';

  export interface ChoiceOption<T> {
    value: T;
    label: string;
    icon?: Component;
    count?: number | string | null;
    showZero?: boolean;
    title?: string;
  }

  interface Props<T> {
    options: ChoiceOption<T>[];
    value: T;
    onchange: (value: T) => void;
    ariaLabel?: string;
    class?: string;
    align?: 'left' | 'right' | 'center';
    disabled?: boolean;
    addon?: Snippet;
    hasAddon?: (option: ChoiceOption<T>) => boolean;
  }

  let {
    options,
    value,
    onchange,
    ariaLabel,
    class: extraClass = '',
    align = 'right',
    disabled = false,
    addon,
    hasAddon
  }: Props<any> = $props();

  let buttons: HTMLButtonElement[] = [];
  let group: HTMLDivElement;
  let indicator: HTMLSpanElement;
  let ready = $state(false);
  let suppressUntil = 0;

  let activeIndex = $derived(options.findIndex((option) => option.value === value));
  let accessibleLabel = $derived(ariaLabel || options.map((option) => option.label).join(', '));
  let activeHasAddon = $derived.by(() => {
    if (!addon) return false;
    if (!hasAddon) return true;
    const option = options[activeIndex];
    return option ? hasAddon(option) : false;
  });

  function measure() {
    const button = buttons[activeIndex];
    if (!group || !indicator || !button) return;
    const groupRect = group.getBoundingClientRect();
    const buttonRect = button.getBoundingClientRect();

    let shift = 0;
    const addonEl = group.querySelector<HTMLElement>('.choice-group__addon');
    const addonSegment = addonEl?.closest('.choice-group__segment');
    const activeSegment = button.closest('.choice-group__segment');
    if (addonEl && addonSegment && activeSegment) {
      const segments = Array.from(group.children);
      if (segments.indexOf(addonSegment) < segments.indexOf(activeSegment)) {
        shift = -addonEl.getBoundingClientRect().width;
      }
    }

    indicator.style.transform = `translate(${buttonRect.left - groupRect.left + shift}px, ${buttonRect.top - groupRect.top}px)`;
    indicator.style.width = `${buttonRect.width}px`;
    indicator.style.height = `${buttonRect.height}px`;
    if (!ready) requestAnimationFrame(() => (ready = true));
  }

  function scheduleMeasure(force = false) {
    requestAnimationFrame(() => {
      if (!force && performance.now() < suppressUntil) return;
      measure();
    });
  }

  $effect(() => {
    void activeIndex;
    void options.length;
    void activeHasAddon;
    suppressUntil = performance.now() + 260;
    scheduleMeasure(true);
    const timer = setTimeout(() => {
      suppressUntil = 0;
      measure();
    }, 280);
    return () => clearTimeout(timer);
  });

  onMount(() => {
    scheduleMeasure(true);
    const observer = new ResizeObserver(() => scheduleMeasure());
    observer.observe(group);
    for (const button of buttons) observer.observe(button);
    return () => observer.disconnect();
  });

  function select(index: number) {
    const option = options[index];
    if (!option) return;
    onchange(option.value);
    buttons[index]?.focus();
  }

  function handleKeydown(event: KeyboardEvent, index: number) {
    let next: number;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = (index + 1) % options.length;
    else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = (index - 1 + options.length) % options.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = options.length - 1;
    else return;
    event.preventDefault();
    select(next);
  }

  function showCount(option: ChoiceOption<unknown>): boolean {
    if (option.count === undefined || option.count === null || option.count === '') return false;
    return option.showZero || option.count !== 0;
  }
</script>

<div
  class="choice-group align-{align} {extraClass}"
  class:is-disabled={disabled}
  role="radiogroup"
  aria-label={accessibleLabel}
  bind:this={group}
>
  <span
    class="choice-group__indicator"
    class:is-ready={ready}
    class:has-addon={activeHasAddon}
    bind:this={indicator}
    aria-hidden="true"
  ></span>
  {#each options as option, index}
    {@const isActive = option.value === value}
    {@const withAddon = Boolean(addon) && (hasAddon ? hasAddon(option) : true)}
    <div class="choice-group__segment">
      <button
        bind:this={buttons[index]}
        type="button"
        use:ripple
        class="choice-group__option"
        class:is-active={isActive}
        class:has-addon={isActive && withAddon}
        role="radio"
        aria-checked={isActive}
        title={option.title}
        tabindex={isActive || (activeIndex === -1 && index === 0) ? 0 : -1}
        data-choice-value={String(option.value)}
        {disabled}
        onclick={() => select(index)}
        onkeydown={(event) => handleKeydown(event, index)}
      >
        {#if option.icon}
          <option.icon />
        {/if}
        <StableWeightLabel text={option.label} reserveWeight="500" />
        {#if showCount(option)}
          <span class="choice-group__count">{option.count}</span>
        {/if}
      </button>
      {#if withAddon && addon}
        <div class="choice-group__addon" class:is-open={isActive} inert={!isActive}>
          {@render addon()}
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .choice-group {
    --choice-inset: 3px;
    --choice-height: var(--control-height, 40px);

    position: relative;
    display: inline-flex;
    align-items: stretch;
    flex-shrink: 0;
    gap: 2px;
    min-width: 0;
    height: var(--choice-height);
    padding: var(--choice-inset);
    box-sizing: border-box;
    border-radius: var(--radius-full);
    background: var(--choice-track, var(--content-surface-high));
    user-select: none;
    white-space: nowrap;
  }

  .choice-group.is-disabled {
    opacity: 0.38;
    pointer-events: none;
  }

  .choice-group__indicator {
    position: absolute;
    top: 0;
    left: 0;
    z-index: 0;
    width: 0;
    height: 0;
    border-radius: var(--radius-full);
    background: var(--text-primary);
    pointer-events: none;
  }

  .choice-group__indicator.is-ready {
    transition:
      transform var(--duration-normal) var(--ease-expo),
      width var(--duration-normal) var(--ease-expo),
      height var(--duration-normal) var(--ease-expo),
      border-radius var(--duration-normal) var(--ease-expo),
      background-color var(--duration-normal) var(--ease-standard);
  }

  .choice-group__indicator.has-addon {
    border-start-end-radius: var(--radius-xs);
    border-end-end-radius: var(--radius-xs);
  }

  .choice-group__segment {
    display: flex;
    align-items: stretch;
    flex: none;
    min-width: 0;
  }

  .choice-group__option {
    position: relative;
    z-index: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    flex-shrink: 0;
    padding: 0 calc(var(--control-padding-x, 14px) - var(--choice-inset));
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-size: var(--control-font-size, 14px);
    font-weight: 500;
    line-height: normal;
    overflow: hidden;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard),
      border-radius var(--duration-normal) var(--ease-expo);
  }

  .choice-group__option.has-addon {
    border-start-end-radius: var(--radius-xs);
    border-end-end-radius: var(--radius-xs);
  }

  .choice-group__addon {
    --choice-addon-width: calc(var(--choice-height) - 2 * var(--choice-inset) + 6px);

    position: relative;
    z-index: 1;
    display: inline-flex;
    align-items: stretch;
    flex: none;
    max-width: 0;
    overflow: hidden;
    border-radius: var(--radius-xs) var(--radius-full) var(--radius-full) var(--radius-xs);
    background: var(--text-primary);
    color: var(--surface-container-low);
    --state-layer-hover: color-mix(in srgb, var(--surface-container-low) 12%, transparent);
    --ripple-color: color-mix(in srgb, var(--surface-container-low) 16%, transparent);
    opacity: 0;
    transition:
      max-width var(--duration-normal) var(--ease-expo),
      opacity var(--duration-fast) var(--ease-standard);
  }

  .choice-group__addon.is-open {
    max-width: var(--choice-addon-width);
    opacity: 1;
  }

  @media (hover: hover) {
    .choice-group__option:not(.is-active):hover {
      background: var(--state-layer-hover);
      color: var(--text-primary);
    }
  }

  .choice-group__option:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .choice-group__option.is-active {
    color: var(--surface-container-low);
  }

  .choice-group__option :global(svg) {
    width: var(--control-icon-size, 18px);
    height: var(--control-icon-size, 18px);
    flex: none;
  }

  .choice-group__count {
    font-variant-numeric: tabular-nums;
    font-weight: 400;
    opacity: 0.72;
  }

  .choice-group.align-left {
    align-self: flex-start;
  }

  :global([data-layout='mobile']) .choice-group:not(.align-left) {
    display: flex;
    width: 100%;
  }

  :global([data-layout='mobile']) .choice-group:not(.align-left) .choice-group__segment {
    flex: 1 1 0;
    min-width: 0;
  }

  :global([data-layout='mobile']) .choice-group:not(.align-left) .choice-group__option {
    flex: 1 1 0;
    min-width: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .choice-group__option,
    .choice-group__addon,
    .choice-group__indicator.is-ready {
      transition: none;
    }
  }

  .choice-group__option:disabled {
    cursor: default;
    pointer-events: none;
  }
</style>
