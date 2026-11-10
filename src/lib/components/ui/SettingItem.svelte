<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { ripple } from '$lib/motion';
  import { marquee } from '$lib/actions/marquee';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import MaterialShape from '$lib/components/ui/MaterialShape.svelte';
  import SwapIcon from '$lib/components/ui/SwapIcon.svelte';
  import { activeMaterialShapeFor } from '$lib/utils/materialShapes';
  import IconOpen from '~icons/fluent/open-24-regular';

  interface Props {
    title: string;
    description?: string;
    summary?: string;
    icon?: Component;
    iconActive?: Component;
    shapeSeed?: string;
    active?: boolean;
    checked?: boolean;
    onCheckedChange?: (checked: boolean) => void;
    onclick?: () => void;
    external?: boolean;
    action?: boolean;
    danger?: boolean;
    stacked?: boolean;
    disabled?: boolean;
    children?: Snippet;
    class?: string;
  }

  let {
    title,
    description = '',
    summary,
    icon: Icon,
    iconActive: IconActive,
    shapeSeed,
    active = false,
    checked,
    onCheckedChange,
    onclick,
    external = false,
    action = false,
    danger = false,
    stacked = false,
    disabled = false,
    children,
    class: extraClass = ''
  }: Props = $props();

  let isSwitch = $derived(checked !== undefined && onCheckedChange !== undefined);
  let isButton = $derived(!isSwitch && onclick !== undefined);
  let isStacked = $derived(Boolean(children) && (stacked || layoutState.isMobile));
  let supporting = $derived(summary ?? description);
  let activeShape = $derived(activeMaterialShapeFor(shapeSeed ?? title));

  function activate() {
    if (disabled) return;
    if (isSwitch) onCheckedChange?.(!checked);
    else onclick?.();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.target !== event.currentTarget) return;
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      activate();
    }
  }
</script>

{#snippet text()}
  {#if Icon}
    <span class="setting-item__icon" class:is-active={active}>
      <MaterialShape
        name={active ? activeShape : 'circle'}
        size={40}
        class="setting-item__shape"
      />
      <span class="setting-item__glyph">
        <SwapIcon icon={Icon} iconActive={IconActive} active={active} size={24} />
      </span>
    </span>
  {/if}
  <span class="setting-item__text">
    {#if isButton}
      <span class="setting-item__title" use:marquee={{ active, trigger: '.setting-item' }}>{title}</span>
      {#if supporting}
        <span class="setting-item__description" use:marquee={{ active, trigger: '.setting-item' }}>{supporting}</span>
      {/if}
    {:else}
      <span class="setting-item__title">{title}</span>
      {#if supporting}
        <span class="setting-item__description">{supporting}</span>
      {/if}
    {/if}
  </span>
{/snippet}

{#if isSwitch}
  <div
    class="setting-item is-interactive {extraClass}"
    class:is-disabled={disabled}
    role="switch"
    aria-checked={checked}
    aria-disabled={disabled || undefined}
    tabindex={disabled ? -1 : 0}
    use:ripple
    onclick={activate}
    onkeydown={handleKeydown}
  >
    {@render text()}
    <span class="setting-item__trailing">
      <Toggle checked={checked ?? false} {disabled} tabindex={-1} class="pointer-events-none" />
    </span>
  </div>
{:else if isButton}
  <div
    class="setting-item is-interactive {extraClass}"
    class:is-active={active}
    class:is-disabled={disabled}
    class:is-danger={danger}
    role="button"
    aria-current={active ? 'page' : undefined}
    aria-disabled={disabled || undefined}
    tabindex={disabled ? -1 : 0}
    use:ripple
    onclick={activate}
    onkeydown={handleKeydown}
  >
    {@render text()}
    {#if !action && external}
      <span class="setting-item__trailing">
        <IconOpen />
      </span>
    {/if}
  </div>
{:else}
  <div
    class="setting-item {extraClass}"
    class:is-stacked={isStacked}
    class:is-disabled={disabled}
    aria-disabled={disabled || undefined}
  >
    {@render text()}
    {#if children}
      <div class="setting-item__control">
        {@render children()}
      </div>
    {/if}
  </div>
{/if}

<style>
  .setting-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 16px;
    width: 100%;
    min-width: 0;
    min-height: 52px;
    padding: 10px 16px;
    box-sizing: border-box;
    border-radius: var(--radius-xs);
    background: var(--settings-tile, var(--content-surface));
    overflow: hidden;
    transition:
      background-color var(--duration-normal) var(--ease-standard),
      border-radius var(--duration-normal) var(--ease-standard);
  }

  .setting-item:global(.is-revealed) {
    isolation: isolate;
  }

  .setting-item:global(.is-revealed)::after {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    background: var(--accent-container);
    pointer-events: none;
    animation: setting-reveal 1600ms var(--ease-standard) forwards;
  }

  @keyframes setting-reveal {
    0%,
    35% {
      opacity: 0.9;
    }
    100% {
      opacity: 0;
    }
  }

  .setting-item.is-stacked {
    flex-wrap: wrap;
  }

  .setting-item.is-stacked .setting-item__control {
    flex: 1 0 100%;
    max-width: none;
    justify-content: stretch;
  }

  .setting-item.is-interactive {
    cursor: default;
    outline: none;
  }

  .setting-item.is-active {
    border-radius: var(--radius-xl);
    background: var(--content-surface-high);
  }

  .setting-item.is-interactive::before {
    content: '';
    position: absolute;
    inset: 0;
    background: transparent;
    pointer-events: none;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  @media (hover: hover) {
    .setting-item.is-interactive:not(.is-disabled):hover::before {
      background: var(--state-layer-hover);
    }
  }

  .setting-item.is-interactive:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .setting-item.is-disabled {
    opacity: 0.38;
    pointer-events: none;
  }

  .setting-item__icon {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 40px;
    height: 40px;
    color: var(--text-secondary);
  }

  .setting-item__icon :global(.setting-item__shape) {
    position: absolute;
    inset: 0;
    color: var(--content-surface-high);
    transition: color var(--duration-normal) var(--ease-standard);
  }

  .setting-item__icon.is-active {
    color: var(--accent-container);
  }

  .setting-item__icon.is-active :global(.setting-item__shape) {
    color: var(--accent-primary);
  }

  .setting-item__glyph {
    position: relative;
    display: grid;
    place-items: center;
  }

  .setting-item__glyph :global(svg) {
    width: 24px;
    height: 24px;
  }

  .setting-item__text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1 1 0;
    min-width: 0;
  }

  .setting-item__title {
    color: var(--text-primary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
    overflow-wrap: break-word;
  }

  .setting-item.is-active .setting-item__title {
    font-weight: 500;
  }

  .setting-item.is-danger .setting-item__title,
  .setting-item.is-danger .setting-item__icon {
    color: var(--status-error);
  }

  .setting-item__description {
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
    overflow-wrap: break-word;
  }

  .setting-item__trailing {
    display: grid;
    place-items: center;
    flex: none;
    color: var(--text-secondary);
  }

  .setting-item__trailing :global(svg) {
    width: 20px;
    height: 20px;
  }

  .setting-item__control {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex: 0 1 var(--settings-control-max-width);
    min-width: 0;
    max-width: var(--settings-control-max-width);
  }

  .setting-item__control > :global(.input-box),
  .setting-item__control > :global(.select-root),
  .setting-item__control > :global(.template-input-wrapper),
  .setting-item__control > :global(.shortcut-box),
  .setting-item__control > :global(.m3-slider),
  .setting-item__control > :global(div.w-full) {
    flex: 1 1 auto;
    width: 100%;
    min-width: 0;
  }

  .setting-item__control > :global(.choice-group) {
    flex: none;
  }

  .setting-item.is-stacked .setting-item__control > :global(.choice-group) {
    flex: 1 1 auto;
  }

  .setting-item__control :global(.settings-action-group) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    width: 100%;
  }

  .setting-item.is-stacked .setting-item__control :global(.settings-action-group) {
    justify-content: flex-start;
  }

  :global([data-layout='mobile']) .setting-item {
    min-height: 56px;
  }

  :global([data-layout='mobile']) .setting-item.is-interactive:has(.setting-item__description) {
    min-height: 72px;
  }

  :global([data-layout='mobile']) .setting-item__title {
    font-size: var(--type-body-lg-size);
    line-height: var(--type-body-lg-line);
  }

  :global([data-layout='mobile']) .setting-item__description {
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
  }
</style>
