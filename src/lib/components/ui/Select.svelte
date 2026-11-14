<script lang="ts">
  import { onMount, onDestroy, tick, type Snippet } from 'svelte';
  import { OverlayScrollbars } from 'overlayscrollbars';
  import { computePosition, autoUpdate, flip, shift, offset, size as floatingSize } from '@floating-ui/dom';
  import { FLOATING_EDGE, FLOATING_OFFSET } from '$lib/utils/floating';
  import { portal } from '$lib/actions/portal';
  import { ripple, tooltip } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import IconChevronDown from '~icons/fluent/chevron-down-24-regular';
  import IconAdd from '~icons/fluent/add-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import CountBadge from './CountBadge.svelte';

  export interface Option {
    value: string | number;
    label: string;
    icon?: any;
    ariaLabel?: string;
    color?: string;
    count?: number | string | null;
    showZero?: boolean;
  }

  interface Props {
    options: Option[];
    value?: string | number;
    selectedValues?: (string | number)[];
    onchange: (val: any) => void;
    class?: string;
    variant?: 'default' | 'ghost' | 'accent';
    style?: string;
    placeholder?: string;
    icon?: any;
    createLabel?: string;
    onCreate?: (name: string) => void | Promise<void>;
    multi?: boolean;
    closeOnChange?: boolean;
    iconOnly?: boolean;
    ariaLabel?: string;
    disabled?: boolean;
    align?: 'left' | 'right';
    open?: boolean;
    size?: 'sm' | 'base' | 'md' | 'lg';
    trigger?: Snippet<[{ toggle: () => void; open: boolean; selectedLabel: string }]>;
  }

  let {
    options,
    value = '',
    selectedValues,
    onchange,
    class: extraClass = '',
    variant = 'default',
    style = '',
    placeholder = '',
    icon,
    createLabel = '',
    onCreate,
    multi = false,
    closeOnChange,
    iconOnly = false,
    ariaLabel,
    disabled = false,
    align = 'left',
    open = $bindable(false),
    size,
    trigger
  }: Props = $props();

  let isOpen = $state(false);

  $effect(() => {
    isOpen = open;
  });

  $effect(() => {
    open = isOpen;
  });
  let containerEl = $state<HTMLDivElement | null>(null);
  let triggerEl = $state<HTMLButtonElement | HTMLDivElement | null>(null);
  let dropdownEl = $state<HTMLDivElement | null>(null);

  let isCreating = $state(false);
  let newOptionName = $state('');
  let createInputEl = $state<HTMLInputElement | null>(null);
  let creatingPending = $state(false);

  let effectiveVariant = $derived.by(() => {
    if (variant === 'accent') return 'accent';
    if (variant === 'ghost' && iconOnly) return 'ghost';
    if (selectedValues && selectedValues.length > 0) return 'accent';
    return variant;
  });

  let selectedOption = $derived.by(() => {
    if (selectedValues && selectedValues.length > 0) {
      if (selectedValues.length === 1) {
        return options.find((opt) => opt.value === selectedValues[0]);
      }
      return null;
    }
    return options.find((opt) => opt.value === value);
  });

  let selectedLabel = $derived.by(() => {
    if (selectedValues && selectedValues.length > 0) {
      if (selectedValues.length === 1) {
        const found = options.find((opt) => opt.value === selectedValues[0]);
        if (found) return found.label;
      }
      return i18n.t('library.in_stashes_count', { count: selectedValues.length });
    }
    return options.find((opt) => opt.value === value)?.label || placeholder || String(value);
  });

  let selectedCount = $derived(selectedOption?.count);

  function isOptSelected(val: string | number) {
    if (selectedValues) {
      return selectedValues.includes(val);
    }
    return value === val;
  }

  async function updatePosition() {
    if (!triggerEl || !dropdownEl) return;

    const triggerRect = triggerEl.getBoundingClientRect();
    const targetWidth = Math.max(triggerRect.width, onCreate ? 260 : 200);
    dropdownEl.style.width = `${targetWidth}px`;
    dropdownEl.style.minWidth = `${targetWidth}px`;
    dropdownEl.style.maxWidth = `min(${Math.max(targetWidth, 340)}px, calc(100vw - 24px))`;

    const { x, y } = await computePosition(triggerEl, dropdownEl, {
      placement: align === 'right' ? 'bottom-end' : 'bottom-start',
      strategy: 'fixed',
      middleware: [
        offset(FLOATING_OFFSET),
        flip({
          fallbackPlacements: ['top-start', 'bottom-end', 'top-end', 'bottom', 'top'],
          padding: FLOATING_EDGE
        }),
        shift({
          padding: FLOATING_EDGE
        }),
        floatingSize({
          padding: FLOATING_EDGE,
          apply({ availableHeight, elements }) {
            Object.assign(elements.floating.style, {
              maxHeight: `${Math.min(availableHeight, 360)}px`
            });
          }
        })
      ]
    });

    if (!dropdownEl) return;
    dropdownEl.style.left = `${x}px`;
    dropdownEl.style.top = `${y}px`;
    dropdownEl.style.visibility = 'visible';
  }

  let cleanupAutoUpdate: (() => void) | null = null;

  $effect(() => {
    if (isOpen && triggerEl && dropdownEl) {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = autoUpdate(triggerEl, dropdownEl, () => {
        void updatePosition();
      });
    } else {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = null;
    }

    return () => {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = null;
    };
  });

  function toggle() {
    isOpen = !isOpen;
    if (isOpen) {
      isCreating = false;
      newOptionName = '';
    }
  }

  function selectOption(val: string | number) {
    onchange(val);
    const shouldClose = closeOnChange !== undefined ? closeOnChange : !multi;
    if (shouldClose) {
      isOpen = false;
    }
  }

  function handleDocumentClick(e: MouseEvent) {
    const target = e.target as Node;
    if (isOpen) {
      const clickedInsideTrigger = containerEl?.contains(target);
      const clickedInsideDropdown = dropdownEl?.contains(target);
      if (!clickedInsideTrigger && !clickedInsideDropdown) {
        isOpen = false;
        isCreating = false;
      }
    }
  }

  async function submitCreate(e: SubmitEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (!newOptionName.trim() || creatingPending || !onCreate) return;
    creatingPending = true;
    try {
      await onCreate(newOptionName.trim());
      newOptionName = '';
      isCreating = false;
      isOpen = false;
    } catch (e) {
      notify.error(i18n.t('library.save_error'), e);
    } finally {
      creatingPending = false;
    }
  }

  function startCreating(e: MouseEvent) {
    e.stopPropagation();
    isCreating = true;
    newOptionName = '';
    tick().then(() => {
      createInputEl?.focus();
      void updatePosition();
    });
  }

  function cancelCreating(e: MouseEvent) {
    e.stopPropagation();
    isCreating = false;
    newOptionName = '';
    tick().then(() => void updatePosition());
  }

  let osInstance: ReturnType<typeof OverlayScrollbars> | null = null;

  function initScrollbars(node: HTMLElement) {
    osInstance = OverlayScrollbars(node, {
      scrollbars: {
        theme: 'os-theme-pawstash',
        autoHide: 'leave',
        autoHideDelay: 400,
        clickScroll: true
      }
    });
    return {
      destroy() {
        osInstance?.destroy();
        osInstance = null;
      }
    };
  }

  onMount(() => {
    document.addEventListener('click', handleDocumentClick, true);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleDocumentClick, true);
    cleanupAutoUpdate?.();
  });
</script>

<div
  bind:this={containerEl}
  class="select-root {extraClass}"
  class:is-open={isOpen}
  class:is-active={selectedValues ? selectedValues.length > 0 : Boolean(value)}
  class:is-disabled={disabled}
  class:is-compact={iconOnly || Boolean(trigger)}
>
  {#if trigger}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div bind:this={triggerEl} class="select-custom-trigger" onclick={(e) => e.stopPropagation()}>
      {@render trigger({ toggle, open: isOpen, selectedLabel })}
    </div>
  {:else}
    <button
      bind:this={triggerEl}
      type="button"
      use:ripple
      onclick={toggle}
      disabled={disabled}
      class="select-trigger variant-{effectiveVariant} {size ? `size-${size}` : ''}"
      class:is-open={isOpen}
      class:is-disabled={disabled}
      class:has-selected={selectedValues ? selectedValues.length > 0 : Boolean(value)}
      class:icon-only={iconOnly}
      aria-haspopup="listbox"
      aria-expanded={isOpen}
      aria-label={ariaLabel || selectedLabel}
      use:tooltip={ariaLabel || selectedLabel}
      {style}
    >
      {#if iconOnly}
        {#if icon}
          {@const IconComp = icon}
          <IconComp class="w-[20px] h-[20px]" />
        {:else if !disabled}
          <IconChevronDown class="w-[20px] h-[20px]" />
        {/if}
      {:else}
        {#if icon}
          {@const IconComp = icon}
          <span class="trigger-icon">
            <IconComp class="w-[18px] h-[18px]" />
          </span>
        {/if}
        <span class="trigger-label">{selectedLabel}</span>
        {#if selectedCount !== undefined && selectedCount !== null}
          <CountBadge count={selectedCount} showZero={selectedOption?.showZero ?? true} variant="tab" class="trigger-count" />
        {/if}
        {#if !disabled}
          <span class="trigger-chevron" class:flipped={isOpen}>
            <IconChevronDown />
          </span>
        {/if}
      {/if}
    </button>
  {/if}

  {#if isOpen}
    <div
      bind:this={dropdownEl}
      use:portal
      class="select-dropdown-portal floating-surface"
    >
      <div
        class="select-dropdown-viewport"
        use:initScrollbars
      >
        <div class="select-options-list" role="listbox">
          {#if options.length === 0}
            <div class="select-empty-msg">{placeholder || i18n.t('library.no_stashes')}</div>
          {/if}

          {#each options as opt (opt.value)}
            {@const active = isOptSelected(opt.value)}

            <button
              type="button"
              role="option"
              aria-selected={active}
              aria-label={opt.ariaLabel}
              class="floating-item select-option"
              onclick={() => selectOption(opt.value)}
              use:ripple
            >
              {#if opt.color}
                <span class="option-color-dot" style:background={opt.color}></span>
              {:else if opt.icon}
                <opt.icon />
              {/if}
              <span class="floating-item-label">{opt.label}</span>
              {#if opt.count !== undefined && opt.count !== null && (opt.count !== 0 || (opt.showZero ?? true))}
                <span class="floating-item-detail">{typeof opt.count === 'number' ? i18n.number(opt.count) : opt.count}</span>
              {/if}
              {#if active}
                <IconCheckmark />
              {/if}
            </button>
          {/each}
        </div>
      </div>

      {#if onCreate}
        <div class="select-footer">
          <div class="floating-divider" role="separator"></div>
          {#if isCreating}
            <form class="floating-item select-create-inline" onsubmit={submitCreate}>
              <input
                bind:this={createInputEl}
                bind:value={newOptionName}
                placeholder={i18n.t('library.stash_name')}
                disabled={creatingPending}
                class="create-inline-input"
                onkeydown={(e) => {
                  if (e.key === 'Escape') {
                    e.preventDefault();
                    e.stopPropagation();
                    cancelCreating(e as any);
                  }
                }}
              />
              <div class="create-inline-actions">
                <button
                  type="submit"
                  class="create-action-btn confirm"
                  use:ripple
                  disabled={!newOptionName.trim() || creatingPending}
                  use:tooltip={i18n.t('common.confirm')}
                  aria-label={createLabel || i18n.t('common.confirm')}
                >
                  <IconCheckmark />
                </button>
                <button
                  type="button"
                  class="create-action-btn cancel"
                  use:ripple
                  onclick={cancelCreating}
                  disabled={creatingPending}
                  use:tooltip={i18n.t('common.cancel')}
                  aria-label={i18n.t('common.cancel')}
                >
                  <IconDismiss />
                </button>
              </div>
            </form>
          {:else}
            <button
              type="button"
              class="floating-item"
              onclick={startCreating}
              use:ripple
            >
              <IconAdd />
              <span class="floating-item-label">{createLabel || i18n.t('library.new_stash')}</span>
            </button>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .select-root {
    position: relative;
    display: inline-flex;
    width: 100%;
    max-width: 100%;
    min-width: 0;
    box-sizing: border-box;
    flex-shrink: 0;
  }

  .select-root.is-compact {
    width: auto;
  }

  .select-custom-trigger {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    position: relative;
    z-index: 1;
  }

  .select-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(8px * var(--ui-scale, 1));
    width: 100%;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1));
    padding: 0 calc(var(--control-padding-x, 16px) * var(--ui-scale, 1));
    background: var(--select-bg);
    border: none;
    border-radius: var(--radius-full);
    color: var(--text-primary);
    font-size: calc(var(--control-font-size, 14px) * var(--ui-scale, 1));
    font-family: var(--font-sans);
    line-height: normal;
    cursor: pointer;
    text-align: left;
    outline: none;
    box-sizing: border-box;
    transition: background var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo);
  }

  .select-trigger.size-sm {
    --control-height: var(--control-height-sm, 34px);
    --control-font-size: var(--control-font-sm, 12.5px);
    --control-icon-size: var(--control-icon-sm, 16px);
    --control-padding-x: var(--control-padding-sm, 12px);
  }

  .select-trigger.size-base {
    --control-height: var(--control-height-base, 40px);
    --control-font-size: var(--control-font-base, 13.5px);
    --control-icon-size: var(--control-icon-base, 18px);
    --control-padding-x: var(--control-padding-base, 14px);
  }

  .select-trigger.size-md {
    --control-height: var(--control-height-md, 46px);
    --control-font-size: var(--control-font-md, 14px);
    --control-icon-size: var(--control-icon-md, 20px);
    --control-padding-x: var(--control-padding-md, 16px);
  }

  .select-trigger.size-lg {
    --control-height: var(--control-height-lg, 52px);
    --control-font-size: var(--control-font-lg, 15px);
    --control-icon-size: var(--control-icon-lg, 22px);
    --control-padding-x: var(--control-padding-lg, 20px);
  }

  .select-trigger :global(.trigger-icon svg),
  .select-trigger :global(.trigger-chevron svg) {
    width: calc(var(--control-icon-size, 18px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-icon-size, 18px) * var(--ui-scale, 1)) !important;
  }

  .select-trigger.icon-only {
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 !important;
    justify-content: center !important;
    border-radius: var(--radius-full) !important;
  }

  .select-trigger:hover {
    background: var(--select-bg-hover);
  }

  .select-trigger:focus-visible {
    outline: calc(1.5px * var(--ui-scale, 1)) solid var(--accent-primary);
    outline-offset: calc(-1.5px * var(--ui-scale, 1));
  }

  .select-trigger.variant-ghost {
    background: transparent;
    color: var(--text-secondary);
  }

  .select-trigger.variant-ghost:hover {
    background: var(--state-layer-hover);
    color: var(--text-primary);
  }

  .select-trigger.variant-ghost.icon-only.has-selected,
  .select-trigger.variant-ghost.icon-only.has-selected:hover {
    color: var(--accent-primary);
  }

  .select-trigger.variant-accent {
    background: var(--accent-container);
    color: var(--accent-on-container);
  }

  .select-trigger.variant-accent:hover {
    background: color-mix(in srgb, var(--accent-container) 80%, var(--accent-primary));
  }

  .trigger-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    opacity: 0.8;
  }

  .trigger-label {
    flex: 0 1 auto;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 500;
    line-height: normal;
  }

  .trigger-chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    opacity: 0.45;
    transition: transform var(--duration-normal) var(--ease-expo),
                opacity var(--duration-normal) var(--ease-expo);
    flex-shrink: 0;
    margin-left: auto;
  }

  :global(.select-trigger .trigger-count) {
    flex-shrink: 0;
  }

  .select-trigger:hover .trigger-chevron {
    opacity: 0.8;
  }

  .trigger-chevron.flipped {
    transform: scaleY(-1);
    opacity: 0.8;
  }

  :global(.select-dropdown-portal) {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 99999;
    visibility: hidden;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding: 0 !important;
    animation: selectDropdownIn var(--duration-fast) var(--ease-standard);
  }

  @keyframes selectDropdownIn {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .select-dropdown-viewport {
    max-height: 280px;
    width: 100%;
  }

  .select-options-list {
    display: flex;
    flex-direction: column;
    width: 100%;
    padding: var(--floating-padding);
    box-sizing: border-box;
  }

  .select-empty-msg {
    padding: 12px var(--floating-item-px);
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
    text-align: center;
  }

  .option-color-dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
  }

  .select-option[aria-selected='true'] .floating-item-detail {
    color: inherit;
  }

  .select-footer {
    flex-shrink: 0;
    width: 100%;
    padding: 0 var(--floating-padding) var(--floating-padding);
    box-sizing: border-box;
  }

  .select-create-inline {
    background: var(--state-layer-hover);
  }

  .create-inline-input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: 0 !important;
    outline: none !important;
    box-shadow: none !important;
    background: transparent !important;
    color: var(--text-primary);
    font: inherit;
  }

  .create-inline-input::placeholder {
    color: var(--text-secondary);
  }

  .create-inline-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-inline-start: auto;
    margin-inline-end: -8px;
    flex-shrink: 0;
  }

  .create-action-btn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--text-secondary);
    transition: background-color var(--duration-fast) var(--ease-standard),
                color var(--duration-fast) var(--ease-standard);
  }

  .create-action-btn :global(svg) {
    width: var(--floating-item-icon-size);
    height: var(--floating-item-icon-size);
  }

  @media (hover: hover) {
    .create-action-btn:hover {
      background: var(--state-layer-hover);
      color: var(--text-primary);
    }

    .create-action-btn.confirm:hover {
      color: var(--accent-primary);
    }
  }

  .create-action-btn:disabled {
    opacity: var(--opacity-disabled);
    background: transparent;
  }
</style>
