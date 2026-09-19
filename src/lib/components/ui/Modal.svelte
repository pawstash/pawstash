<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { portal } from '$lib/actions/portal';
  import { scrollable } from '$lib/actions/scrollable';
  import { dialogFocus } from '$lib/actions/dialogFocus';
  import { ripple } from '$lib/motion';
  import { i18n } from '$lib/i18n';
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
    borderlessFooter?: boolean;
    scrollable?: boolean;
    icon?: Component;
    tone?: 'accent' | 'danger';
    dismissible?: boolean;
    onclose: () => void;
    children?: Snippet;
    footer?: Snippet;
    floating?: Snippet;
    maxHeight?: string;
  }

  let {
    isOpen = false,
    title = '',
    size = 'md',
    position = 'center',
    fixedHeight = false,
    flush = false,
    borderlessHeader = false,
    borderlessFooter = false,
    scrollable: isScrollable = true,
    icon,
    tone = 'accent',
    dismissible = true,
    onclose,
    children,
    footer,
    floating,
    maxHeight
  }: Props = $props();

  const maxWidths: Record<string, string> = {
    sm: '400px',
    md: '440px',
    lg: '560px',
    xl: '640px',
    '2xl': '880px',
    full: '96vw'
  };

  const titleId = $props.id();
  const isHero = $derived(Boolean(icon));
</script>

{#if isOpen}
  <div
    use:portal={'body'}
    class="modal-overlay"
    class:is-top={position === 'top'}
    class:mobile={layoutState.isMobile}
  >
    <button
      type="button"
      class="modal-backdrop"
      tabindex="-1"
      aria-hidden="true"
      onclick={onclose}
    ></button>

    <div
      class="modal-box"
      class:is-fixed-height={fixedHeight}
      class:is-flush={flush}
      class:is-hero={isHero}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabindex="-1"
      style:max-width={maxWidths[size] || maxWidths.md}
      style:max-height={maxHeight || undefined}
      use:dialogFocus={{ onEscape: onclose }}
    >
      <div class="modal-header" class:is-flush={flush} class:is-borderless={borderlessHeader}>
        {#if icon}
          {@const HeroIcon = icon}
          <span class="modal-hero" data-tone={tone} aria-hidden="true">
            <HeroIcon />
          </span>
        {/if}

        <h3 class="modal-title" id={titleId}>{title}</h3>

        {#if dismissible}
          <button
            type="button"
            class="modal-close-btn"
            use:ripple
            onclick={onclose}
            aria-label={i18n.t('common.close')}
          >
            <IconClose />
          </button>
        {/if}
      </div>

      {#if children}
        {#if isScrollable}
          <div class="modal-body-wrapper" class:is-fixed-height={fixedHeight} use:scrollable>
            <div class="modal-body" class:is-flush={flush} class:has-footer={Boolean(footer)} class:is-fixed-height={fixedHeight}>
              {@render children()}
            </div>
          </div>
        {:else}
          <div class="modal-body-wrapper is-unscrollable" class:is-fixed-height={fixedHeight}>
            <div class="modal-body" class:is-flush={flush} class:has-footer={Boolean(footer)} class:is-fixed-height={fixedHeight}>
              {@render children()}
            </div>
          </div>
        {/if}
      {/if}

      {#if footer}
        <div class="modal-footer" class:is-flush={flush} class:is-borderless={borderlessFooter}>
          {@render footer()}
        </div>
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
    z-index: 2000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 12px;
    box-sizing: border-box;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: var(--modal-scrim);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    cursor: default;
    animation: modal-backdrop-in var(--duration-normal) var(--ease-expo) both;
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
    --modal-close-slack: calc(
      (var(--modal-close-size) - var(--modal-close-icon)) / 2 * var(--ui-scale, 1)
    );
    position: relative;
    z-index: 10;
    width: 100%;
    max-height: min(90vh, calc(100vh - 48px));
    display: flex;
    flex-direction: column;
    background: var(--floating-bg);
    border: var(--floating-border);
    border-radius: calc(var(--modal-radius) * var(--ui-scale, 1));
    box-shadow: var(--modal-shadow);
    backdrop-filter: var(--floating-backdrop);
    -webkit-backdrop-filter: var(--floating-backdrop);
    overflow: hidden;
    box-sizing: border-box;
    outline: none;
    animation: modal-box-in var(--duration-normal) var(--ease-modal-spring) both;
  }

  @keyframes modal-box-in {
    from {
      opacity: 0;
      transform: translateY(12px) scale(0.94);
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
    gap: calc(var(--space-2) * var(--ui-scale, 1));
    padding:
      calc(var(--modal-padding) * var(--ui-scale, 1))
      calc(var(--modal-padding) * var(--ui-scale, 1) - var(--modal-close-slack))
      0
      calc(var(--modal-padding) * var(--ui-scale, 1));
    flex-shrink: 0;
    box-sizing: border-box;
  }

  .modal-title {
    margin: 0;
    min-width: 0;
    font-family: var(--font-display);
    font-size: calc(var(--modal-title-size) * var(--ui-scale, 1));
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    letter-spacing: -0.015em;
    line-height: 1.3;
    overflow-wrap: anywhere;
  }

  .modal-box.is-hero .modal-header {
    flex-direction: column;
    align-items: center;
    gap: calc(var(--modal-hero-gap) * var(--ui-scale, 1));
    padding:
      calc(var(--modal-hero-padding-top) * var(--ui-scale, 1))
      calc(var(--modal-padding) * var(--ui-scale, 1))
      0;
  }

  .modal-box.is-hero .modal-title {
    text-align: center;
    font-size: calc(var(--modal-hero-title-size) * var(--ui-scale, 1));
    line-height: 1.25;
  }

  .modal-hero {
    display: grid;
    place-items: center;
    width: calc(var(--modal-hero-box) * var(--ui-scale, 1));
    height: calc(var(--modal-hero-box) * var(--ui-scale, 1));
    border-radius: var(--radius-full);
    background: color-mix(in srgb, var(--modal-hero-tone) 18%, transparent);
    color: var(--modal-hero-tone);
  }

  .modal-hero[data-tone='accent'] {
    --modal-hero-tone: var(--accent-primary);
  }

  .modal-hero[data-tone='danger'] {
    --modal-hero-tone: var(--status-error);
  }

  .modal-hero :global(svg) {
    width: calc(var(--modal-hero-icon) * var(--ui-scale, 1));
    height: calc(var(--modal-hero-icon) * var(--ui-scale, 1));
  }

  .modal-close-btn {
    flex: none;
    width: calc(var(--modal-close-size) * var(--ui-scale, 1));
    height: calc(var(--modal-close-size) * var(--ui-scale, 1));
    border-radius: var(--radius-full);
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

  .modal-close-btn :global(svg) {
    width: calc(var(--modal-close-icon) * var(--ui-scale, 1));
    height: calc(var(--modal-close-icon) * var(--ui-scale, 1));
  }

  .modal-close-btn:hover {
    color: var(--text-primary);
    background: rgba(var(--surface-tint-rgb), 0.08);
  }

  .modal-close-btn:active {
    background: rgba(var(--surface-tint-rgb), 0.14);
  }

  .modal-close-btn:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -1px;
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

  .modal-body-wrapper {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
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

  .modal-body {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding:
      calc(var(--modal-title-content-gap) * var(--ui-scale, 1))
      calc(var(--modal-padding) * var(--ui-scale, 1))
      calc(var(--modal-padding) * var(--ui-scale, 1));
    box-sizing: border-box;
  }

  .modal-box.is-hero .modal-body {
    text-align: center;
  }

  .modal-body.has-footer {
    padding-bottom: 0;
  }

  .modal-body.is-fixed-height {
    flex: 1 1 0%;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .modal-footer {
    display: flex;
    flex-direction: column;
    padding:
      calc(var(--modal-content-footer-gap) * var(--ui-scale, 1))
      calc(var(--modal-padding) * var(--ui-scale, 1))
      calc(var(--modal-padding) * var(--ui-scale, 1));
    border-top: var(--border-width) solid var(--border-color);
    flex-shrink: 0;
    box-sizing: border-box;
  }

  .modal-footer.is-borderless {
    border-top: none;
  }

  .modal-footer.is-flush {
    padding: 0;
  }

  .modal-box.is-flush {
    padding: 0;
  }

  .modal-header.is-flush {
    padding:
      calc(var(--modal-padding) * var(--ui-scale, 1))
      calc(var(--modal-padding) * var(--ui-scale, 1) - var(--modal-close-slack))
      calc(var(--modal-title-content-gap) * var(--ui-scale, 1))
      calc(var(--modal-padding) * var(--ui-scale, 1));
  }

  .modal-header.is-borderless {
    border-bottom: none !important;
  }

  .modal-body.is-flush {
    padding: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .modal-backdrop,
    .modal-box {
      animation: none;
    }
  }
</style>
