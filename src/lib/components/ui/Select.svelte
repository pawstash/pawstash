<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { OverlayScrollbars } from 'overlayscrollbars';
  import { computePosition, autoUpdate, flip, shift, offset, size } from '@floating-ui/dom';
  import { portal } from '$lib/actions/portal';
  import { ripple } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import IconChevronDown from '~icons/fluent/chevron-down-24-regular';
  import IconAdd from '~icons/fluent/add-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';

  interface Option {
    value: string | number;
    label: string;
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
    ariaLabel
  }: Props = $props();

  let isOpen = $state(false);
  let containerEl = $state<HTMLDivElement | null>(null);
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let dropdownEl = $state<HTMLDivElement | null>(null);
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
    dropdownEl.style.minWidth = `${Math.max(triggerRect.width, 200)}px`;

    const { x, y } = await computePosition(triggerEl, dropdownEl, {
      placement: 'bottom-start',
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
      if (!multi) {
        isOpen = false;
      }
    } catch (err) {
      console.error(err);
    } finally {
      creatingPending = false;
    }
  }

  function startCreating(e: MouseEvent) {
    e.stopPropagation();
    isCreating = true;
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
    if (typeof document !== 'undefined') {
      document.removeEventListener('click', handleDocumentClick, true);
    }
    cleanupAutoUpdate?.();
  });
</script>

<div
  bind:this={containerEl}
  class="select-root {extraClass}"
>
  <button
    bind:this={triggerEl}
    type="button"
    use:ripple
    onclick={toggle}
    class="select-trigger variant-{effectiveVariant}"
    class:is-open={isOpen}
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
      {:else}
        <IconChevronDown class="w-[20px] h-[20px]" />
      {/if}
    {:else}
      {#if icon}
        {@const IconComp = icon}
        <span class="trigger-icon">
          <IconComp class="w-[16px] h-[16px]" />
        </span>
      {/if}
      <span class="trigger-label">{selectedLabel}</span>
      <span class="trigger-chevron" class:flipped={isOpen}>
        <IconChevronDown />
      </span>
    {/if}
  </button>

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

          {#each options as opt}
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
            <form class="select-create-form" onsubmit={submitCreate}>
              <input
                bind:this={createInputEl}
                bind:value={newOptionName}
                placeholder={i18n.t('library.stash_name') || 'Stash name...'}
                class="select-create-input"
                disabled={creatingPending}
              />
              <button
                type="submit"
                class="select-create-btn-confirm"
                disabled={!newOptionName.trim() || creatingPending}
                title="Create"
              >
                <IconCheckmark class="w-[14px] h-[14px]" />
              </button>
              <button
                type="button"
                class="select-create-btn-cancel"
                onclick={cancelCreating}
                disabled={creatingPending}
                title="Cancel"
              >
                <IconDismiss class="w-[14px] h-[14px]" />
              </button>
            </form>
          {:else}
            <button
              type="button"
              class="select-create-trigger"
              onclick={startCreating}
              use:ripple
            >
              <IconAdd class="w-[15px] h-[15px]" />
              <span>{createLabel || i18n.t('library.new_stash') || 'New stash'}</span>
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
    width: 100%;
    user-select: none;
  }

  .select-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    height: 46px;
    padding: 0 14px;
    background: var(--bg-card);
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    font-size: 14px;
    font-family: var(--font-sans);
    cursor: pointer;
    text-align: left;
    outline: none;
    transition: background var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo);
  }

  .select-trigger.icon-only {
    width: 44px !important;
    min-width: 44px !important;
    height: 44px !important;
    padding: 0 !important;
    justify-content: center !important;
    border-radius: var(--radius-full) !important;
  }

  .select-trigger:hover {
    background: var(--bg-card-hover);
    border-color: var(--border-color-hover);
  }

  .select-trigger.is-open {
    background: var(--bg-card-hover);
    border-color: var(--border-color-focus);
  }

  .select-trigger.variant-ghost {
    background: transparent;
    border-color: transparent;
    color: var(--text-secondary);
  }

  .select-trigger.variant-ghost:hover {
    background: rgba(255, 255, 255, 0.05);
    border-color: transparent;
    color: var(--text-primary);
  }

  .select-trigger.variant-ghost.is-open {
    background: rgba(255, 255, 255, 0.05);
    border-color: transparent;
    color: var(--text-primary);
  }

  .select-trigger.variant-accent {
    background: color-mix(in srgb, var(--accent-primary) 12%, transparent);
    border-color: color-mix(in srgb, var(--accent-primary) 35%, transparent);
    color: var(--accent-primary);
    font-weight: 550;
  }

  .select-trigger.variant-accent:hover {
    background: color-mix(in srgb, var(--accent-primary) 18%, transparent);
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  .select-trigger.variant-accent.is-open {
    background: color-mix(in srgb, var(--accent-primary) 22%, transparent);
    border-color: var(--accent-primary);
  }

  .trigger-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    opacity: 0.85;
    flex-shrink: 0;
  }

  .trigger-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .trigger-chevron {
    display: flex;
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
    background: var(--bg-dropdown);
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-lg);
    box-shadow: 0 14px 44px rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(var(--backdrop-blur));
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .select-dropdown-viewport {
    max-height: 240px;
    width: 100%;
  }

  .select-options-list {
    display: flex;
    flex-direction: column;
    padding: 6px;
    width: 100%;
    box-sizing: border-box;
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
    gap: 8px;
    width: 100%;
    height: 38px;
    padding: 0 12px;
    background: transparent;
    border: none;
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 13.5px;
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
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-primary);
  }

  .select-option.is-selected {
    background: color-mix(in srgb, var(--accent-primary) 14%, transparent);
    color: var(--accent-primary);
    font-weight: 550;
  }

  :global(.option-check) {
    color: var(--accent-primary);
    flex-shrink: 0;
  }

  .select-footer {
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    padding: 6px;
    background: rgba(0, 0, 0, 0.15);
  }

  .select-create-trigger {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 36px;
    padding: 0 10px;
    border-radius: 8px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 13px;
    font-family: var(--font-sans);
    font-weight: 500;
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-expo);
  }

  .select-create-trigger:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .select-create-form {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
  }

  .select-create-input {
    flex: 1;
    min-width: 0;
    height: 34px;
    padding: 0 10px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-sans);
    outline: none;
    transition: border-color 0.15s ease;
  }

  .select-create-input:focus {
    border-color: var(--accent-primary);
  }

  .select-create-btn-confirm,
  .select-create-btn-cancel {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    border: none;
    cursor: pointer;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .select-create-btn-confirm {
    background: var(--accent-primary);
    color: var(--accent-text, #ffffff);
  }

  .select-create-btn-confirm:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .select-create-btn-cancel {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-muted);
  }

  .select-create-btn-cancel:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #ffffff;
  }
</style>
