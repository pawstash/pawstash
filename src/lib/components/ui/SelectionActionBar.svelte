<script lang="ts">
  import type { Snippet } from 'svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import { portal } from '$lib/actions/portal';
  import IconCheckmarkCircle from '~icons/fluent/checkmark-circle-24-filled';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconSelectAll from '~icons/fluent/select-all-on-24-regular';

  interface Props {
    totalCount?: number;
    onSelectAll?: () => void;
    children?: Snippet;
  }

  let { totalCount = 0, onSelectAll, children }: Props = $props();

  let selectedCount = $derived(selectionState.count);
  let allSelected = $derived(totalCount > 0 && selectedCount >= totalCount);

  function handleToggleAll() {
    if (allSelected) {
      selectionState.clear();
    } else if (onSelectAll) {
      onSelectAll();
    }
  }

  function handleClose() {
    selectionState.exit();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && selectionState.active) {
      selectionState.exit();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if selectionState.active}
  <aside
    use:portal
    class="selection-floating-dock-portal"
    class:is-mobile={layoutState.isMobile}
    aria-label="Selection actions"
  >
    <div class="selection-dock" role="toolbar">
      <div class="selection-counter">
        <span class="selection-count-badge" class:active={selectedCount > 0}>
          {selectedCount}
        </span>
        <span class="selection-count-label">
          {selectedCount === 0
            ? (i18n.t('selection.none_selected') || 'No items')
            : (i18n.t('selection.items_count') || 'selected')}
        </span>
      </div>

      <div class="selection-dock-divider"></div>

      {#if onSelectAll && totalCount > 0}
        <button
          type="button"
          class="selection-dock-btn"
          onclick={handleToggleAll}
          title={i18n.t(allSelected ? 'selection.deselect_all' : 'selection.select_all')}
        >
          <IconSelectAll class="w-[17px] h-[17px]" />
          <span>{i18n.t(allSelected ? 'selection.deselect_all' : 'selection.select_all') || (allSelected ? 'Deselect all' : 'Select all')}</span>
        </button>

        <div class="selection-dock-divider"></div>
      {/if}

      {#if children}
        <div class="selection-actions-container">
          {@render children()}
        </div>

        <div class="selection-dock-divider"></div>
      {/if}

      <button
        type="button"
        class="selection-dock-close-btn"
        onclick={handleClose}
        title="{i18n.t('selection.cancel') || 'Cancel'} (Esc)"
        aria-label="Close selection"
      >
        <IconDismiss class="w-[18px] h-[18px]" />
      </button>
    </div>
  </aside>
{/if}

<style>
  .selection-floating-dock-portal {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 10000;
    pointer-events: none;
    display: flex;
    justify-content: center;
    max-width: calc(100vw - 28px);
    transition: bottom 0.22s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .selection-floating-dock-portal.is-mobile {
    bottom: calc(max(14px, env(safe-area-inset-bottom, 14px)) + 64px + 12px);
    width: min(calc(100vw - 28px), 520px);
  }

  @media (max-width: 768px) {
    .selection-floating-dock-portal {
      bottom: calc(max(14px, env(safe-area-inset-bottom, 14px)) + 64px + 12px);
      width: min(calc(100vw - 28px), 520px);
    }
  }

  .selection-dock {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 56px;
    padding: 0 10px;
    border-radius: 22px;
    background: rgba(16, 17, 22, 0.88);
    border: none !important;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(24px) saturate(1.6);
    -webkit-backdrop-filter: blur(24px) saturate(1.6);
    animation: floatingDockIn 220ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
    white-space: nowrap;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
    max-width: 100%;
  }

  .selection-dock::-webkit-scrollbar {
    display: none;
  }

  @keyframes floatingDockIn {
    0% {
      opacity: 0;
      transform: translateY(20px) scale(0.96);
    }
    100% {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .selection-counter {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 12px 0 8px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.05);
    flex-shrink: 0;
  }

  .selection-count-badge {
    min-width: 24px;
    height: 24px;
    padding: 0 7px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.12);
    color: rgba(255, 255, 255, 0.7);
    font-size: 12px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .selection-count-badge.active {
    background: var(--accent-primary);
    color: var(--accent-text, #ffffff);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
  }

  .selection-count-label {
    font-size: 13px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.75);
    letter-spacing: -0.01em;
  }

  .selection-dock-divider {
    width: 1px;
    height: 22px;
    background: rgba(255, 255, 255, 0.08);
    margin: 0 2px;
    flex-shrink: 0;
  }

  .selection-dock-btn {
    height: 38px;
    padding: 0 12px;
    border-radius: 9999px;
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.7);
    font-size: 13px;
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    transition: color 0.18s ease, background 0.18s ease, transform 0.1s ease;
    flex-shrink: 0;
  }

  .selection-dock-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #ffffff;
  }

  .selection-dock-btn:active {
    transform: scale(0.97);
  }

  .selection-actions-container {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  :global(.selection-dock .selection-btn) {
    height: 38px !important;
    padding: 0 14px !important;
    font-size: 13px !important;
    font-weight: 500 !important;
    gap: 7px !important;
    border-radius: 9999px !important;
    white-space: nowrap !important;
    background: rgba(255, 255, 255, 0.05) !important;
    border: none !important;
    color: rgba(255, 255, 255, 0.8) !important;
    transition: all 0.18s ease !important;
    flex-shrink: 0 !important;
  }

  :global(.selection-dock .selection-btn:hover) {
    background: rgba(255, 255, 255, 0.12) !important;
    color: #ffffff !important;
  }

  :global(.selection-dock .selection-btn:active) {
    transform: scale(0.97) !important;
  }

  :global(.selection-dock .selection-btn.btn-danger),
  :global(.selection-dock .btn-danger.selection-btn) {
    background: rgba(239, 68, 68, 0.15) !important;
    color: #fca5a5 !important;
  }

  :global(.selection-dock .selection-btn.btn-danger:hover),
  :global(.selection-dock .btn-danger.selection-btn:hover) {
    background: rgba(239, 68, 68, 0.28) !important;
    color: #ffffff !important;
  }

  :global(.selection-dock .selection-stash-select) {
    width: auto !important;
    min-width: 150px;
    flex-shrink: 0 !important;
  }

  :global(.selection-dock .selection-stash-select .select-trigger) {
    height: 38px !important;
    padding: 0 14px !important;
    font-size: 13px !important;
    font-weight: 500 !important;
    gap: 8px !important;
    border-radius: 9999px !important;
    background: var(--bg-card) !important;
    border: var(--border-width) solid var(--border-color) !important;
    color: var(--text-primary) !important;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2) !important;
    transition: background var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo) !important;
  }

  :global(.selection-dock .selection-stash-select .select-trigger:hover),
  :global(.selection-dock .selection-stash-select .select-trigger.is-open) {
    background: var(--bg-card-hover) !important;
    border-color: var(--border-color-hover) !important;
    color: var(--text-primary) !important;
  }

  .selection-dock-close-btn {
    width: 38px;
    height: 38px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.05);
    border: none;
    color: rgba(255, 255, 255, 0.5);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.18s ease;
    flex-shrink: 0;
  }

  .selection-dock-close-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #ffffff;
  }

  .selection-dock-close-btn:active {
    transform: scale(0.95);
  }
</style>
