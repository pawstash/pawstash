<script lang="ts">
  import type { Snippet } from 'svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import { tooltip } from '$lib/motion';
  import { portal } from '$lib/actions/portal';
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
  {#if layoutState.isMobile}
    <!-- Mobile Adaptive Dock (1:1 identical to MobileBottomNav) -->
    <aside
      use:portal
      class="mobile-bottom-dock selection-mobile-dock"
      aria-label={i18n.t('selection.select_mode') || 'Selection actions'}
    >
      <div class="mobile-dock-capsule selection-dock-capsule" role="toolbar">
        <!-- 1. Select All / Deselect Toggle with Selected Count Badge -->
        {#if onSelectAll && totalCount > 0}
          <button
            type="button"
            class="mobile-dock-btn"
            class:active={selectedCount > 0}
            onclick={handleToggleAll}
            aria-label={i18n.t(allSelected ? 'selection.deselect_all' : 'selection.select_all')}
          >
            <div class="dock-icon-wrapper">
              <IconSelectAll class="dock-icon" />
              {#if selectedCount > 0}
                <span class="dock-badge">{selectedCount > 99 ? '99+' : selectedCount}</span>
              {/if}
            </div>
            <span class="dock-label">
              {i18n.t(allSelected ? 'selection.deselect_all' : 'selection.select_all') ||
                (allSelected ? 'Deselect' : 'Select all')}
            </span>
          </button>
        {/if}

        <!-- 2. Action Buttons (children) -->
        {#if children}
          {@render children()}
        {/if}

        <!-- 3. Close / Cancel Button -->
        <button
          type="button"
          class="mobile-dock-btn"
          onclick={handleClose}
          aria-label={i18n.t('selection.cancel') || 'Cancel'}
        >
          <div class="dock-icon-wrapper">
            <IconDismiss class="dock-icon" />
          </div>
          <span class="dock-label">
            {i18n.t('selection.cancel') || 'Cancel'}
          </span>
        </button>
      </div>
    </aside>
  {:else}
    <!-- Desktop Floating Glass Dock -->
    <aside
      use:portal
      class="selection-portal is-desktop"
      aria-label={i18n.t('selection.select_mode') || 'Selection actions'}
    >
      <div class="selection-desktop-dock" role="toolbar">
        <div class="selection-desktop-counter">
          <span class="selection-count-badge" class:active={selectedCount > 0}>
            {selectedCount}
          </span>
          <span class="selection-count-text">
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
            aria-label={i18n.t(allSelected ? 'selection.deselect_all' : 'selection.select_all')}
          >
            <IconSelectAll class="w-[17px] h-[17px]" />
            <span>
              {i18n.t(allSelected ? 'selection.deselect_all' : 'selection.select_all') ||
                (allSelected ? 'Deselect all' : 'Select all')}
            </span>
          </button>

          <div class="selection-dock-divider"></div>
        {/if}

        {#if children}
          <div class="selection-desktop-actions">
            {@render children()}
          </div>

          <div class="selection-dock-divider"></div>
        {/if}

        <button
          type="button"
          class="selection-desktop-close"
          onclick={handleClose}
          use:tooltip={`${i18n.t('selection.cancel') || 'Cancel'} (Esc)`}
          aria-label={i18n.t('selection.cancel') || 'Close selection'}
        >
          <IconDismiss class="w-[18px] h-[18px]" />
        </button>
      </div>
    </aside>
  {/if}
{/if}

<style>
  /* ---------------- Common Portal & Badges ---------------- */
  .selection-portal {
    position: fixed;
    left: 50%;
    transform: translateX(-50%);
    z-index: 10000;
    pointer-events: none;
    display: flex;
    justify-content: center;
  }

  /* ---------------- Mobile Adaptive Dock (1:1 identical to MobileBottomNav) ---------------- */
  .selection-mobile-dock {
    position: fixed;
    bottom: max(14px, env(safe-area-inset-bottom, 14px));
    left: 50%;
    transform: translateX(-50%);
    z-index: 10000;
    width: min(calc(100vw - 28px), 440px);
    pointer-events: auto;
  }

  .selection-dock-capsule {
    display: flex;
    align-items: center;
    justify-content: space-around;
    height: 64px;
    padding: 0 8px;
    background: rgba(16, 17, 22, 0.88);
    border: none !important;
    border-radius: 22px;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(24px) saturate(1.6);
    -webkit-backdrop-filter: blur(24px) saturate(1.6);
    animation: mobileDockIn 220ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  @keyframes mobileDockIn {
    0% {
      opacity: 0;
      transform: translateY(24px) scale(0.96);
    }
    100% {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .mobile-dock-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    height: 100%;
    background: transparent;
    color: rgba(255, 255, 255, 0.45);
    border: none;
    cursor: pointer;
    padding: 4px 0;
    transition: color 0.18s ease;
    min-width: 0;
  }

  .mobile-dock-btn:hover {
    color: rgba(255, 255, 255, 0.8);
  }

  .mobile-dock-btn.active {
    color: #ffffff;
  }

  .dock-icon-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  :global(.dock-icon) {
    width: 24px !important;
    height: 24px !important;
    flex-shrink: 0;
  }

  .dock-label {
    font-size: 11px;
    font-weight: 500;
    line-height: 1;
    margin-top: 4px;
    white-space: nowrap;
    letter-spacing: -0.01em;
    color: inherit;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .mobile-dock-btn.active .dock-label {
    font-weight: 700;
    color: #ffffff;
  }

  .dock-badge {
    position: absolute;
    top: -5px;
    right: -8px;
    background: var(--accent-primary, #ef4444);
    color: #ffffff;
    font-size: 9.5px;
    font-weight: 700;
    line-height: 1;
    padding: 2px 4.5px;
    border-radius: 9999px;
    border: 1.5px solid rgba(16, 17, 22, 0.95);
    font-variant-numeric: tabular-nums;
  }

  /* Map child buttons from views directly to mobile-dock-btn specifications */
  :global(.selection-dock-capsule .selection-btn) {
    display: flex !important;
    flex-direction: column !important;
    align-items: center !important;
    justify-content: center !important;
    flex: 1 !important;
    height: 100% !important;
    background: transparent !important;
    color: rgba(255, 255, 255, 0.45) !important;
    border: none !important;
    box-shadow: none !important;
    border-radius: 0 !important;
    cursor: pointer !important;
    padding: 4px 0 !important;
    transition: color 0.18s ease !important;
    min-width: 0 !important;
    gap: 0 !important;
  }

  :global(.selection-dock-capsule .selection-btn svg) {
    width: 24px !important;
    height: 24px !important;
    flex-shrink: 0 !important;
    color: inherit !important;
  }

  :global(.selection-dock-capsule .selection-btn span) {
    font-size: 11px !important;
    font-weight: 500 !important;
    line-height: 1 !important;
    margin-top: 4px !important;
    white-space: nowrap !important;
    letter-spacing: -0.01em !important;
    color: inherit !important;
    overflow: hidden !important;
    text-overflow: ellipsis !important;
    max-width: 100% !important;
  }

  :global(.selection-dock-capsule .selection-btn:hover) {
    color: rgba(255, 255, 255, 0.8) !important;
  }

  :global(.selection-dock-capsule .selection-btn:active) {
    color: #ffffff !important;
    transform: scale(0.96) !important;
  }

  :global(.selection-dock-capsule .selection-btn.btn-danger),
  :global(.selection-dock-capsule .btn-danger.selection-btn) {
    color: #fca5a5 !important;
  }

  :global(.selection-dock-capsule .selection-btn.btn-danger:hover),
  :global(.selection-dock-capsule .btn-danger.selection-btn:hover),
  :global(.selection-dock-capsule .selection-btn.btn-danger:active),
  :global(.selection-dock-capsule .btn-danger.selection-btn:active) {
    color: #ef4444 !important;
  }

  /* Stash select mapping in mobile dock */
  :global(.selection-dock-capsule .selection-stash-select) {
    display: flex !important;
    flex: 1 !important;
    height: 100% !important;
    min-width: 0 !important;
    width: auto !important;
  }

  :global(.selection-dock-capsule .selection-stash-select .select-trigger) {
    display: flex !important;
    flex-direction: column !important;
    align-items: center !important;
    justify-content: center !important;
    width: 100% !important;
    height: 100% !important;
    background: transparent !important;
    color: rgba(255, 255, 255, 0.45) !important;
    border: none !important;
    box-shadow: none !important;
    border-radius: 0 !important;
    cursor: pointer !important;
    padding: 4px 0 !important;
    transition: color 0.18s ease !important;
    gap: 0 !important;
  }

  :global(.selection-dock-capsule .selection-stash-select .select-trigger .trigger-icon svg) {
    width: 24px !important;
    height: 24px !important;
    flex-shrink: 0 !important;
    color: inherit !important;
  }

  :global(.selection-dock-capsule .selection-stash-select .select-trigger .trigger-label) {
    font-size: 11px !important;
    font-weight: 500 !important;
    line-height: 1 !important;
    margin-top: 4px !important;
    white-space: nowrap !important;
    letter-spacing: -0.01em !important;
    color: inherit !important;
    overflow: hidden !important;
    text-overflow: ellipsis !important;
    max-width: 100% !important;
  }

  :global(.selection-dock-capsule .selection-stash-select .select-trigger .trigger-chevron) {
    display: none !important;
  }

  :global(.selection-dock-capsule .selection-stash-select .select-trigger:hover) {
    color: rgba(255, 255, 255, 0.8) !important;
  }

  :global(.selection-dock-capsule .selection-stash-select .select-trigger:active),
  :global(.selection-dock-capsule .selection-stash-select .select-trigger.is-open) {
    color: #ffffff !important;
    transform: scale(0.96) !important;
  }

  :global(.selection-dock-capsule .selection-stash-select .select-trigger.variant-accent) {
    color: var(--accent-primary) !important;
  }

  /* ---------------- Desktop Floating Glass Dock ---------------- */
  .selection-portal.is-desktop {
    bottom: 28px;
    max-width: calc(100vw - 32px);
  }

  .selection-desktop-dock {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 52px;
    padding: 0 8px 0 12px;
    border-radius: var(--radius-full, 9999px);
    background: rgba(16, 17, 22, 0.88);
    border: none !important;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(24px) saturate(1.6);
    -webkit-backdrop-filter: blur(24px) saturate(1.6);
    animation: desktopDockIn 220ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
    white-space: nowrap;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
    max-width: 100%;
  }

  .selection-desktop-dock::-webkit-scrollbar {
    display: none;
  }

  @keyframes desktopDockIn {
    0% {
      opacity: 0;
      transform: translateY(22px) scale(0.96);
    }
    100% {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .selection-desktop-counter {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 10px 0 6px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.04);
    flex-shrink: 0;
  }

  .selection-count-badge {
    min-width: 22px;
    height: 22px;
    padding: 0 6px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.12);
    color: rgba(255, 255, 255, 0.7);
    font-size: 11.5px;
    font-weight: 700;
    font-family: var(--font-display, var(--font-sans));
    font-variant-numeric: tabular-nums;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    flex-shrink: 0;
  }

  .selection-count-badge.active {
    background: var(--accent-primary);
    color: var(--accent-text, #ffffff);
    box-shadow: 0 2px 10px var(--accent-glow, rgba(214, 144, 133, 0.35));
  }

  .selection-desktop-counter .selection-count-text {
    font-size: 13px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    letter-spacing: -0.01em;
  }

  .selection-dock-divider {
    width: 1px;
    height: 20px;
    background: rgba(255, 255, 255, 0.08);
    margin: 0 2px;
    flex-shrink: 0;
  }

  .selection-dock-btn {
    height: 36px;
    padding: 0 12px;
    border-radius: 9999px;
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.75);
    font-size: 13px;
    font-weight: 500;
    line-height: normal;
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

  .selection-desktop-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  :global(.selection-desktop-dock .selection-btn) {
    height: 36px !important;
    padding: 0 13px !important;
    font-size: 13px !important;
    font-weight: 500 !important;
    line-height: normal !important;
    gap: 7px !important;
    border-radius: 9999px !important;
    white-space: nowrap !important;
    background: rgba(255, 255, 255, 0.05) !important;
    border: 1px solid rgba(255, 255, 255, 0.05) !important;
    color: rgba(255, 255, 255, 0.8) !important;
    transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1) !important;
    flex-shrink: 0 !important;
  }

  :global(.selection-desktop-dock .selection-btn:hover) {
    background: rgba(255, 255, 255, 0.11) !important;
    border-color: rgba(255, 255, 255, 0.1) !important;
    color: #ffffff !important;
  }

  :global(.selection-desktop-dock .selection-btn:active) {
    transform: scale(0.97) !important;
  }

  :global(.selection-desktop-dock .selection-btn.btn-danger),
  :global(.selection-desktop-dock .btn-danger.selection-btn) {
    background: rgba(239, 68, 68, 0.12) !important;
    border-color: rgba(239, 68, 68, 0.15) !important;
    color: #fca5a5 !important;
  }

  :global(.selection-desktop-dock .selection-btn.btn-danger:hover),
  :global(.selection-desktop-dock .btn-danger.selection-btn:hover) {
    background: rgba(239, 68, 68, 0.22) !important;
    border-color: rgba(239, 68, 68, 0.3) !important;
    color: #ffffff !important;
  }

  :global(.selection-desktop-dock .selection-stash-select) {
    width: auto !important;
    min-width: 140px;
    flex-shrink: 0 !important;
  }

  :global(.selection-desktop-dock .selection-stash-select .select-trigger) {
    height: 36px !important;
    padding: 0 13px !important;
    font-size: 13px !important;
    font-weight: 500 !important;
    gap: 8px !important;
    border-radius: 9999px !important;
    background: rgba(255, 255, 255, 0.05) !important;
    border: 1px solid rgba(255, 255, 255, 0.06) !important;
    color: rgba(255, 255, 255, 0.85) !important;
    box-shadow: none !important;
    transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1) !important;
  }

  :global(.selection-desktop-dock .selection-stash-select .select-trigger:hover),
  :global(.selection-desktop-dock .selection-stash-select .select-trigger.is-open) {
    background: rgba(255, 255, 255, 0.11) !important;
    border-color: rgba(255, 255, 255, 0.12) !important;
    color: #ffffff !important;
  }

  .selection-desktop-close {
    width: 36px;
    height: 36px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid transparent;
    color: rgba(255, 255, 255, 0.5);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.18s ease;
    flex-shrink: 0;
  }

  .selection-desktop-close:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.08);
    color: #ffffff;
  }

  .selection-desktop-close:active {
    transform: scale(0.95);
  }
</style>
