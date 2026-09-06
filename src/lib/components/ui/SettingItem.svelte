<script lang="ts">
  import type { Snippet } from 'svelte';
  import { fade } from 'svelte/transition';
  import { ripple, tooltip } from '$lib/motion';
  import { i18n } from '$lib/i18n/i18nState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import IconUndo from '~icons/fluent/arrow-undo-24-regular';
  import IconQuestion from '~icons/fluent/question-circle-24-regular';

  interface Props {
    title: string;
    description?: string;
    icon?: any;
    value?: unknown;
    defaultValue?: unknown;
    onReset?: () => void;
    children?: Snippet;
    class?: string;
    align?: 'left' | 'right';
  }

  let {
    title,
    description = '',
    icon: IconComponent,
    value,
    defaultValue,
    onReset,
    children,
    class: extraClass = '',
    align = 'right'
  }: Props = $props();

  function isValuesEqual(a: unknown, b: unknown): boolean {
    if (a === b) return true;
    if (typeof a !== typeof b) return false;
    if (typeof a === 'object' && a !== null && b !== null) {
      return JSON.stringify(a) === JSON.stringify(b);
    }
    return false;
  }

  let showReset = $derived(
    value !== undefined && defaultValue !== undefined && !isValuesEqual(value, defaultValue)
  );
  let isHelpOpen = $state(false);

  function handleHelpClick() {
    if (layoutState.isMobile) {
      isHelpOpen = true;
    }
  }
</script>

<div class="setting-item {extraClass}">
  <div class="setting-item__header">
    <div class="setting-item__title-row">
      {#if IconComponent}
        <div class="setting-item__icon">
          <IconComponent />
        </div>
      {/if}
      <span class="setting-item__title">{title}</span>
      {#if description}
        <button
          type="button"
          use:tooltip={layoutState.isMobile ? undefined : { text: description, placement: 'top' }}
          onclick={handleHelpClick}
          class="setting-item__help"
          aria-label={description}
          aria-haspopup={layoutState.isMobile ? 'dialog' : undefined}
          aria-expanded={layoutState.isMobile ? isHelpOpen : undefined}
        >
          <IconQuestion />
        </button>
      {/if}
      {#if onReset && showReset}
        <button
          type="button"
          transition:fade={{ duration: 150 }}
          use:ripple
          use:tooltip={layoutState.isMobile ? undefined : { text: i18n.t('settings.reset_setting_to_default') || 'Reset to default', placement: 'top' }}
          onclick={(e) => {
            e.stopPropagation();
            onReset();
          }}
          class="setting-item__reset"
          aria-label={i18n.t('settings.reset_setting_to_default') || 'Reset to default'}
        >
          <IconUndo />
        </button>
      {/if}
    </div>
  </div>

  <div
    class="setting-item__control"
    class:is-right={align === 'right'}
    class:is-left={align === 'left'}
  >
    {#if children}
      {@render children()}
    {/if}
  </div>
</div>

<BottomSheet
  isOpen={isHelpOpen && layoutState.isMobile}
  title={title}
  closeLabel={i18n.t('common.close')}
  actionLabel={i18n.t('common.got_it')}
  onclose={() => (isHelpOpen = false)}
>
  <p class="setting-item__help-text">{description}</p>
</BottomSheet>

<style>
  .setting-item {
    display: flex;
    align-items: center;
    width: 100%;
    min-height: calc(var(--settings-row-min-height, 58px) * var(--ui-scale, 1));
    min-width: 0;
    max-width: 100%;
    gap: calc(16px * var(--ui-scale, 1));
    padding: calc(9px * var(--ui-scale, 1)) calc(16px * var(--ui-scale, 1));
    border-radius: calc(var(--radius-sm) * var(--ui-scale, 1));
    background: var(--settings-row-bg, var(--bg-card));
    box-sizing: border-box;
  }

  .setting-item__header {
    flex: 1 1 auto;
    min-width: 0;
  }

  .setting-item__title-row {
    display: flex;
    align-items: center;
    min-width: 0;
    gap: calc(10px * var(--ui-scale, 1));
  }

  .setting-item__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: calc(21.5px * var(--ui-scale, 1));
    height: calc(21.5px * var(--ui-scale, 1));
    color: rgba(255, 255, 255, 0.5);
    flex: none;
  }

  .setting-item__icon :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }

  .setting-item__title {
    min-width: 0;
    color: rgba(255, 255, 255, 0.85);
    font-family: var(--font-sans);
    font-size: calc(13.5px * var(--ui-scale, 1));
    font-weight: var(--font-weight-normal);
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .setting-item__help,
  .setting-item__reset {
    position: relative;
    display: inline-grid;
    place-items: center;
    flex: none;
    width: calc(28px * var(--ui-scale, 1));
    height: calc(28px * var(--ui-scale, 1));
    padding: 0;
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--text-secondary);
    transition:
      background var(--duration-fast) var(--ease-expo),
      color var(--duration-fast) var(--ease-expo);
  }

  .setting-item__help {
    cursor: help;
  }

  .setting-item__reset {
    cursor: pointer;
  }

  .setting-item__help:hover,
  .setting-item__reset:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
  }

  .setting-item__help:focus-visible,
  .setting-item__reset:focus-visible {
    outline: calc(1.5px * var(--ui-scale, 1)) solid var(--accent-primary) !important;
    outline-offset: calc(-1.5px * var(--ui-scale, 1)) !important;
    box-shadow: none !important;
  }

  .setting-item__help :global(svg) {
    width: calc(20px * var(--ui-scale, 1));
    height: calc(20px * var(--ui-scale, 1));
  }

  .setting-item__reset :global(svg) {
    width: calc(17px * var(--ui-scale, 1));
    height: calc(17px * var(--ui-scale, 1));
  }

  .setting-item__control {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex: 0 1 calc(var(--settings-control-max-width, 360px) * var(--ui-scale, 1));
    width: 100%;
    min-width: 0;
    max-width: calc(var(--settings-control-max-width, 360px) * var(--ui-scale, 1));
    margin-left: auto;
  }

  .setting-item__control.is-right {
    justify-content: flex-end;
  }

  .setting-item__control.is-left {
    justify-content: flex-start;
  }

  .setting-item__help-text {
    margin: 0;
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-size: calc(var(--text-sm) * var(--ui-scale, 1));
    line-height: var(--leading-normal);
    overflow-wrap: anywhere;
    user-select: text;
  }

  :global([data-layout='mobile']) .setting-item__help {
    cursor: pointer;
  }

  :global([data-layout='mobile']) .setting-item__help + .setting-item__reset {
    margin-left: calc(var(--space-2) * var(--ui-scale, 1));
  }

  :global([data-layout='mobile']) .setting-item__help::after,
  :global([data-layout='mobile']) .setting-item__reset::after {
    content: '';
    position: absolute;
    inset: calc(var(--space-2) * var(--ui-scale, 1) * -1);
  }

  @media (max-width: 768px) {
    .setting-item__help {
      cursor: pointer;
    }

    .setting-item__help + .setting-item__reset {
      margin-left: calc(var(--space-2) * var(--ui-scale, 1));
    }

    .setting-item__help::after,
    .setting-item__reset::after {
      content: '';
      position: absolute;
      inset: calc(var(--space-2) * var(--ui-scale, 1) * -1);
    }
  }

  .setting-item.col-span-full .setting-item__control {
    max-width: none;
  }

  .setting-item__control :global(.settings-action-group) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: calc(8px * var(--ui-scale, 1));
    width: 100%;
  }

  .setting-item__control > :global(.input-box),
  .setting-item__control > :global(.select-root),
  .setting-item__control > :global(.template-input-container),
  .setting-item__control > :global(.shortcut-input-container),
  .setting-item__control > :global(div.w-full) {
    width: 100%;
    flex: 1 1 auto;
    min-width: 0;
  }

  .setting-item__control > :global(.choice-group) {
    width: auto;
    flex: 0 0 auto;
    margin-left: auto;
  }

  .setting-item__control.is-left > :global(.choice-group) {
    margin-left: 0;
  }

  :global([data-layout='mobile']) .setting-item {
    flex-direction: column;
    align-items: stretch;
    gap: calc(12px * var(--ui-scale, 1));
    padding: calc(12px * var(--ui-scale, 1));
  }

  :global([data-layout='mobile']) .setting-item__control,
  :global([data-layout='mobile']) .setting-item__control.is-right,
  :global([data-layout='mobile']) .setting-item__control.is-left {
    flex: none;
    width: 100%;
    max-width: none;
    margin-left: 0;
    justify-content: flex-start;
  }

  :global([data-layout='mobile']) .setting-item__control > :global(*),
  :global([data-layout='mobile']) .setting-item__control > :global(.input-box),
  :global([data-layout='mobile']) .setting-item__control > :global(.select-root),
  :global([data-layout='mobile']) .setting-item__control > :global(.choice-group),
  :global([data-layout='mobile']) .setting-item__control > :global(.template-input-container),
  :global([data-layout='mobile']) .setting-item__control > :global(.shortcut-input-container),
  :global([data-layout='mobile']) .setting-item__control > :global(div.w-full) {
    width: 100% !important;
    max-width: 100% !important;
    margin-left: 0 !important;
    flex: 1 1 auto !important;
  }

  :global([data-layout='mobile']) .setting-item__control > :global(.btn),
  :global([data-layout='mobile']) .setting-item__control > :global(button.btn) {
    width: 100%;
    justify-content: center;
  }

  :global([data-layout='mobile']) .setting-item__control :global(.settings-action-group) {
    width: 100%;
    justify-content: stretch;
  }

  :global([data-layout='mobile']) .setting-item__control :global(.settings-action-group > .btn),
  :global([data-layout='mobile']) .setting-item__control :global(.settings-action-group > button) {
    flex: 1 1 calc(50% - 6px);
    min-width: 110px;
    justify-content: center;
    text-align: center;
  }

  @media (max-width: 640px) {
    .setting-item {
      flex-direction: column;
      align-items: stretch;
      gap: calc(12px * var(--ui-scale, 1));
      padding: calc(12px * var(--ui-scale, 1));
    }

    .setting-item__control,
    .setting-item__control.is-right,
    .setting-item__control.is-left {
      flex: none;
      width: 100%;
      max-width: none;
      margin-left: 0;
      justify-content: flex-start;
    }

    .setting-item__control > :global(*),
    .setting-item__control > :global(.input-box),
    .setting-item__control > :global(.select-root),
    .setting-item__control > :global(.choice-group),
    .setting-item__control > :global(.template-input-container),
    .setting-item__control > :global(.shortcut-input-container),
    .setting-item__control > :global(div.w-full) {
      width: 100% !important;
      max-width: 100% !important;
      margin-left: 0 !important;
      flex: 1 1 auto !important;
    }

    .setting-item__control > :global(.btn),
    .setting-item__control > :global(button.btn) {
      width: 100%;
      justify-content: center;
    }

    .setting-item__control :global(.settings-action-group) {
      width: 100%;
      justify-content: stretch;
    }

    .setting-item__control :global(.settings-action-group > .btn),
    .setting-item__control :global(.settings-action-group > button) {
      flex: 1 1 calc(50% - 6px);
      min-width: 110px;
      justify-content: center;
      text-align: center;
    }
  }
</style>
