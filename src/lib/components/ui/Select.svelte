<script lang="ts">
  import { onMount, onDestroy, tick, type Snippet } from 'svelte';
  import { OverlayScrollbars } from 'overlayscrollbars';
  import { computePosition, autoUpdate, flip, shift, offset, size } from '@floating-ui/dom';
  import { portal } from '$lib/actions/portal';
  import { ripple } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import { logger } from '$lib/utils/logger';
  import IconChevronDown from '~icons/fluent/chevron-down-24-regular';
  import IconAdd from '~icons/fluent/add-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';

  export interface Option {
    value: string | number;
    label: string;
    icon?: any;
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
    trigger
  }: Props = $props();

  let isOpen = $state(false);
  let containerEl = $state<HTMLDivElement | null>(null);
  let triggerEl = $state<HTMLButtonElement | HTMLDivElement | null>(null);
  let dropdownEl = $state<HTMLDivElement | null>(null);

  // In-place Creation state
  let isCreating = $state(false);
  let newOptionName = $state('');
  let createInputEl = $state<HTMLInputElement | null>(null);
  let creatingPending = $state(false);

  let effectiveVariant = $derived.by(() => {
    if (variant === 'accent') return 'accent';
    if (selectedValues && selectedValues.length > 0) return 'accent';
    return variant;
  });

  let selectedLabel = $derived.by(() => {
    if (selectedValues && selectedValues.length > 0) {
      if (selectedValues.length === 1) {
        const found = options.find((opt) => opt.value === selectedValues[0]);
        if (found) return found.label;
      }
      return i18n.t('library.in_stashes_count', { count: selectedValues.length }) || `${selectedValues.length} stashes`;
    }
    return options.find((opt) => opt.value === value)?.label || placeholder || String(value);
  });

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
        offset(6),
        flip({
          fallbackPlacements: ['top-start', 'bottom-end', 'top-end', 'bottom', 'top'],
          padding: 12
        }),
        shift({
          padding: 12
        }),
        size({
          padding: 12,
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
      logger.error('Failed to create item in Select', e);
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
      class="select-trigger variant-{effectiveVariant}"
      class:is-open={isOpen}
      class:is-disabled={disabled}
      class:has-selected={selectedValues ? selectedValues.length > 0 : Boolean(value)}
      class:icon-only={iconOnly}
      aria-haspopup="listbox"
      aria-expanded={isOpen}
      aria-label={ariaLabel || selectedLabel}
      title={ariaLabel || selectedLabel}
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
      class="select-dropdown-portal"
    >
      <div
        class="select-dropdown-viewport"
        use:initScrollbars
      >
        <div class="select-options-list" role="listbox">
          {#if options.length === 0}
            <div class="select-empty-msg">{placeholder || i18n.t('library.no_stashes') || 'No stashes yet'}</div>
          {/if}

          {#each options as opt (opt.value)}
            {@const active = isOptSelected(opt.value)}

            <button
              type="button"
              role="option"
              aria-selected={active}
              class="select-option"
              class:is-selected={active}
              onclick={() => selectOption(opt.value)}
              use:ripple
            >
              <span class="option-label">{opt.label}</span>
              {#if active}
                <IconCheckmark class="w-[15px] h-[15px] option-check" />
              {/if}
            </button>
          {/each}
        </div>
      </div>

      {#if onCreate}
        <div class="select-footer">
          {#if isCreating}
            <form class="select-create-inline" onsubmit={submitCreate}>
              <IconAdd class="w-[16px] h-[16px] create-inline-icon" />
              <input
                bind:this={createInputEl}
                bind:value={newOptionName}
                placeholder={i18n.t('library.stash_name') || 'Stash name...'}
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
                  title={i18n.t('common.confirm') || 'Create'}
                  aria-label="Create stash"
                >
                  <IconCheckmark class="w-[15px] h-[15px]" />
                </button>
                <button
                  type="button"
                  class="create-action-btn cancel"
                  use:ripple
                  onclick={cancelCreating}
                  disabled={creatingPending}
                  title={i18n.t('common.cancel') || 'Cancel'}
                  aria-label="Cancel"
                >
                  <IconDismiss class="w-[15px] h-[15px]" />
                </button>
              </div>
            </form>
          {:else}
            <button
              type="button"
              class="select-create-trigger"
              onclick={startCreating}
              use:ripple
            >
              <IconAdd class="w-[16px] h-[16px] create-inline-icon" />
              <span class="create-trigger-text">{createLabel || i18n.t('library.new_stash') || 'New stash'}</span>
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
    gap: 8px;
    width: 100%;
    height: 46px;
    padding: 0 16px;
    background: var(--bg-card);
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    font-size: 14px;
    font-family: var(--font-sans);
    cursor: pointer;
    text-align: left;
    outline: none;
    box-sizing: border-box;
    transition: background var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo);
  }

  .select-trigger.icon-only {
    width: 46px !important;
    min-width: 46px !important;
    height: 46px !important;
    padding: 0 !important;
    justify-content: center !important;
    border-radius: var(--radius-full) !important;
  }

  .select-trigger:hover {
    background: var(--bg-card-hover);
    border-color: var(--border-color-hover);
  }

  .select-trigger:focus-visible {
    border-color: var(--border-color-focus, var(--accent-primary));
    box-shadow: 0 0 0 2px var(--accent-glow, rgba(255, 255, 255, 0.15));
  }

  .select-trigger.variant-ghost {
    background: transparent;
    border-color: transparent;
    color: var(--text-secondary);
  }

  .select-trigger.variant-ghost:hover {
    background: var(--bg-card-hover);
    border-color: var(--border-color-hover);
    color: var(--text-primary);
  }

  .select-trigger.variant-accent {
    background: color-mix(in srgb, var(--accent-primary) 14%, transparent);
    border-color: color-mix(in srgb, var(--accent-primary) 28%, transparent);
    color: var(--accent-primary);
  }

  .select-trigger.variant-accent:hover {
    background: color-mix(in srgb, var(--accent-primary) 22%, transparent);
    border-color: color-mix(in srgb, var(--accent-primary) 40%, transparent);
  }

  .trigger-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    opacity: 0.8;
  }

  .trigger-label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 500;
  }

  .trigger-chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    opacity: 0.45;
    transition: transform var(--duration-normal) var(--ease-expo),
                opacity var(--duration-normal) var(--ease-expo);
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
    background: var(--floating-bg);
    border: var(--floating-border);
    border-radius: var(--floating-radius, 18px);
    box-shadow: var(--floating-shadow);
    backdrop-filter: var(--floating-backdrop);
    -webkit-backdrop-filter: var(--floating-backdrop);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    animation: selectDropdownIn 0.16s var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1));
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
    padding: var(--floating-padding, 6px);
    width: 100%;
    box-sizing: border-box;
    gap: var(--floating-gap, 2px);
  }

  .select-empty-msg {
    padding: 12px 14px;
    font-size: 13px;
    color: var(--text-muted);
    text-align: center;
  }

  .select-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--floating-item-gap, 10px);
    width: 100%;
    height: var(--floating-item-height, 36px);
    padding: 0 var(--floating-item-px, 12px);
    background: transparent;
    border: none;
    border-radius: var(--floating-item-radius, 12px);
    color: var(--text-secondary);
    font-size: var(--floating-item-font-size, 13.5px);
    font-family: var(--font-sans);
    cursor: pointer;
    text-align: left;
    outline: none;
    box-sizing: border-box;
    white-space: nowrap;
    transition: background var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo);
  }

  .select-option:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .select-option.is-selected {
    background: color-mix(in srgb, var(--accent-primary) 14%, transparent);
    color: var(--accent-primary);
    font-weight: 550;
  }

  .select-option.is-selected:hover {
    background: color-mix(in srgb, var(--accent-primary) 20%, transparent);
  }

  .option-label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  :global(.option-check) {
    color: var(--accent-primary);
    flex-shrink: 0;
  }

  /* Footer Creation */
  .select-footer {
    border-top: 1px solid var(--floating-divider-color, rgba(255, 255, 255, 0.035));
    padding: var(--floating-padding, 6px);
    box-sizing: border-box;
    width: 100%;
    flex-shrink: 0;
  }

  .select-create-trigger,
  .select-create-inline {
    display: flex;
    align-items: center;
    gap: var(--floating-item-gap, 10px);
    width: 100%;
    height: var(--floating-item-height, 36px);
    padding: 0 var(--floating-item-px, 12px);
    border-radius: var(--floating-item-radius, 12px);
    background: transparent;
    border: none;
    box-sizing: border-box;
    transition: background var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo);
  }

  .select-create-trigger {
    color: var(--text-secondary);
    font-size: var(--floating-item-font-size, 13.5px);
    font-family: var(--font-sans);
    cursor: pointer;
    text-align: left;
    outline: none;
  }

  .select-create-trigger:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .select-create-inline {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .create-inline-icon {
    opacity: 0.6;
    flex-shrink: 0;
  }

  .create-trigger-text {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .create-inline-input {
    flex: 1;
    min-width: 0;
    height: 100%;
    background: transparent !important;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
    padding: 0;
    color: var(--text-primary);
    font-size: 13.5px;
    font-family: var(--font-sans);
    box-sizing: border-box;
  }

  .create-inline-input::placeholder {
    color: var(--text-muted);
    opacity: 0.55;
    font-size: 13px;
  }

  .create-inline-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
    flex-shrink: 0;
  }

  .create-action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-secondary);
    opacity: 0.6;
    border-radius: var(--radius-sm, 6px);
    cursor: pointer;
    padding: 0;
    transition: opacity var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo),
                background var(--duration-fast) var(--ease-expo);
  }

  .create-action-btn:hover {
    opacity: 1;
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
  }

  .create-action-btn.confirm:hover {
    color: var(--accent-primary);
  }

  .create-action-btn:disabled {
    opacity: 0.25;
    cursor: not-allowed;
    background: transparent;
  }
</style>
