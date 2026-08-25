<script lang="ts">
  import type { Snippet } from 'svelte';
  import { scrollable } from '$lib/actions/scrollable';
  import { ripple } from '$lib/motion';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import IconClose from '~icons/fluent/dismiss-24-regular';

  interface Props {
    isOpen?: boolean;
    title?: string;
    size?: 'sm' | 'md' | 'lg' | 'xl' | '2xl' | 'full';
    position?: 'center' | 'top';
    fixedHeight?: boolean;
    flush?: boolean;
    borderlessHeader?: boolean;
    scrollable?: boolean;
    onclose: () => void;
    children?: Snippet;
    floating?: Snippet;
  }

  let {
    isOpen = false,
    title = '',
    size = 'md',
    position = 'center',
    fixedHeight = false,
    flush = false,
    borderlessHeader = false,
    scrollable: isScrollable = true,
    onclose,
    children,
    floating
  }: Props = $props();

  const maxWidths: Record<string, string> = {
    sm: '340px',
    md: '380px',
    lg: '480px',
    xl: '640px',
    '2xl': '880px',
    full: '96vw'
  };
</script>

{#if isOpen}
  <div
    class="modal-overlay"
    class:is-top={position === 'top'}
    class:mobile={layoutState.isMobile}
  >
    <div
      role="button"
      tabindex="0"
      class="modal-backdrop"
      onclick={onclose}
      onkeydown={(e) => (e.key === 'Escape' || e.key === 'Enter') && onclose()}
    ></div>

    <div
      class="modal-box"
      class:is-fixed-height={fixedHeight}
      class:is-flush={flush}
      style:max-width={maxWidths[size] || '380px'}
    >
      <div class="modal-header" class:is-flush={flush} class:is-borderless={borderlessHeader}>
        <h3 class="modal-title">{title}</h3>
        <button
          type="button"
          class="modal-close-btn"
          use:ripple
          onclick={onclose}
          aria-label="Close"
        >
          <IconClose class="w-5 h-5" />
        </button>
      </div>

      {#if children}
        {#if isScrollable}
          <div class="modal-body-wrapper" class:is-fixed-height={fixedHeight} use:scrollable>
            <div class="modal-body" class:is-flush={flush} class:is-fixed-height={fixedHeight}>
              {@render children()}
            </div>
          </div>
        {:else}
          <div class="modal-body-wrapper is-unscrollable" class:is-fixed-height={fixedHeight}>
            <div class="modal-body" class:is-flush={flush} class:is-fixed-height={fixedHeight}>
              {@render children()}
            </div>
          </div>
        {/if}
      {/if}

      {#if floating}
        {@render floating()}
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 12px;
    box-sizing: border-box;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.72);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    animation: modal-backdrop-in 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes modal-backdrop-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .modal-box {
    position: relative;
    z-index: 10;
    width: 100%;
    max-height: min(90vh, calc(100vh - 48px));
    display: flex;
    flex-direction: column;
    padding: var(--modal-padding, 6px);
    background: var(--floating-bg);
    border: var(--floating-border);
    border-radius: var(--modal-radius, 20px);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.75), 0 0 0 1px rgba(255, 255, 255, 0.05);
    backdrop-filter: var(--floating-backdrop);
    -webkit-backdrop-filter: var(--floating-backdrop);
    overflow: hidden;
    box-sizing: border-box;
    animation: modal-box-in 0.22s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes modal-box-in {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 6px 6px 10px;
    border-bottom: none;
    flex-shrink: 0;
    box-sizing: border-box;
  }

  .modal-title {
    margin: 0;
    font-family: var(--font-outfit, var(--font-sans));
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: -0.01em;
    line-height: 1.25;
  }

  .modal-close-btn {
    width: 28px;
    height: 28px;
    border-radius: var(--floating-item-radius, 10px);
    display: grid;
    place-items: center;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-muted);
    cursor: pointer;
    box-sizing: border-box;
    transition: color var(--duration-fast, 150ms) var(--ease-expo),
                background var(--duration-fast, 150ms) var(--ease-expo);
  }

  .modal-close-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
  }

  .modal-close-btn:active {
    background: rgba(255, 255, 255, 0.14);
  }

  .modal-overlay.is-top {
    align-items: flex-start;
    padding: 20px 16px;
  }

  .modal-box.is-fixed-height {
    height: calc(100vh - 40px);
    max-height: calc(100vh - 40px);
  }

  /* Mobile safe area: status bar (top) + floating navbar (bottom) */
  .modal-overlay.mobile {
    padding-top: calc(var(--mobile-status-bar-height, 36px) + 8px);
    padding-bottom: calc(var(--mobile-nav-height, 64px) + max(16px, env(safe-area-inset-bottom, 16px)));
    padding-left: 10px;
    padding-right: 10px;
  }

  .modal-overlay.mobile.is-top {
    align-items: flex-start;
    padding-top: calc(var(--mobile-status-bar-height, 36px) + 8px);
    padding-bottom: calc(var(--mobile-nav-height, 64px) + max(16px, env(safe-area-inset-bottom, 16px)));
    padding-left: 10px;
    padding-right: 10px;
  }

  .modal-overlay.mobile .modal-box {
    max-height: 100%;
  }

  .modal-overlay.mobile .modal-box.is-fixed-height {
    height: 100%;
    max-height: 100%;
  }

  @media (max-width: 768px) {
    .modal-overlay {
      padding-top: calc(var(--mobile-status-bar-height, 36px) + 8px);
      padding-bottom: calc(var(--mobile-nav-height, 64px) + max(16px, env(safe-area-inset-bottom, 16px)));
      padding-left: 10px;
      padding-right: 10px;
    }

    .modal-overlay.is-top {
      align-items: flex-start;
      padding-top: calc(var(--mobile-status-bar-height, 36px) + 8px);
      padding-bottom: calc(var(--mobile-nav-height, 64px) + max(16px, env(safe-area-inset-bottom, 16px)));
      padding-left: 10px;
      padding-right: 10px;
    }

    .modal-box {
      max-height: 100%;
    }

    .modal-box.is-fixed-height {
      height: 100%;
      max-height: 100%;
    }
  }

  .modal-body-wrapper.is-fixed-height {
    flex: 1 1 0%;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .modal-body-wrapper.is-unscrollable {
    overflow: hidden;
  }

  .modal-body.is-fixed-height {
    flex: 1 1 0%;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .modal-box.is-flush {
    padding: 0;
  }

  .modal-header.is-flush {
    padding: 10px 14px 6px 14px;
  }

  .modal-header.is-borderless {
    border-bottom: none !important;
  }

  .modal-body {
    padding: 0;
    box-sizing: border-box;
  }

  .modal-body.is-flush {
    padding: 0;
  }
</style>
