<script lang="ts">
  import { untrack, type Component, type Snippet } from 'svelte';
  import { portal } from '$lib/actions/portal';
  import { scrollable } from '$lib/actions/scrollable';
  import { dialogFocus } from '$lib/actions/dialogFocus';
  import { ripple } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import MaterialShape from '$lib/components/ui/MaterialShape.svelte';
  import { heroShapeCycle } from '$lib/utils/materialShapes';
  import IconClose from '~icons/fluent/dismiss-24-regular';

  interface Props {
    isOpen?: boolean;
    title?: string;
    size?: 'sm' | 'md' | 'lg' | 'xl' | '2xl' | 'full';
    flush?: boolean;
    borderlessFooter?: boolean;
    scrollable?: boolean;
    icon?: Component;
    tone?: 'accent' | 'danger';
    dismissible?: boolean;
    onclose: () => void;
    children?: Snippet;
    footer?: Snippet;
  }

  let {
    isOpen = false,
    title = '',
    size = 'md',
    flush = false,
    borderlessFooter = false,
    scrollable: isScrollable = true,
    icon,
    tone = 'accent',
    dismissible = true,
    onclose,
    children,
    footer
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
  let heroNonce = $state(Math.random());
  let wasOpen = untrack(() => isOpen);
  $effect.pre(() => {
    if (isOpen && !wasOpen) heroNonce = Math.random();
    wasOpen = isOpen;
  });
  const heroCycle = $derived(heroShapeCycle(`${title}\u0001${heroNonce}`, tone));
  const heroShape = $derived(heroCycle[0] ?? 'circle');
</script>

{#if isOpen}
  <div
    use:portal={'body'}
    class="modal-overlay"
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
      class:is-flush={flush}
      class:is-hero={isHero}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabindex="-1"
      style:max-width={maxWidths[size] || maxWidths.md}
      use:dialogFocus={{ onEscape: onclose }}
    >
      <div class="modal-header" class:is-flush={flush}>
        {#if icon}
          {@const HeroIcon = icon}
          <span class="modal-hero" data-tone={tone} aria-hidden="true">
            <MaterialShape
              name={heroShape}
              size={56}
              duration={700}
              cycle={heroCycle}
              spin
              class="modal-hero__shape"
            />
            <span class="modal-hero__glyph"><HeroIcon /></span>
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
          <div class="modal-body-wrapper" use:scrollable>
            <div class="modal-body" class:is-flush={flush} class:has-footer={Boolean(footer)}>
              {@render children()}
            </div>
          </div>
        {:else}
          <div class="modal-body-wrapper is-unscrollable">
            <div class="modal-body" class:is-flush={flush} class:has-footer={Boolean(footer)}>
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
    background: var(--surface-container-high);
    border-radius: calc(var(--modal-radius) * var(--ui-scale, 1));
    box-shadow: var(--modal-shadow);
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
    font-weight: 400;
    color: var(--text-primary);
    line-height: var(--type-title-lg-line);
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
    position: relative;
    display: grid;
    place-items: center;
    width: calc(var(--modal-hero-box) * var(--ui-scale, 1));
    height: calc(var(--modal-hero-box) * var(--ui-scale, 1));
    color: var(--modal-hero-tone);
  }

  .modal-hero :global(.modal-hero__shape) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    color: color-mix(in srgb, var(--modal-hero-tone) 20%, transparent);
  }

  .modal-hero__glyph {
    position: relative;
    display: grid;
    place-items: center;
  }

  .modal-hero[data-tone='accent'] {
    --modal-hero-tone: var(--accent-primary);
  }

  .modal-hero[data-tone='danger'] {
    --modal-hero-tone: var(--status-error);
  }

  .modal-hero__glyph :global(svg) {
    width: calc(var(--modal-hero-icon) * var(--ui-scale, 1));
    height: calc(var(--modal-hero-icon) * var(--ui-scale, 1));
  }

  .modal-box.is-hero .modal-close-btn {
    position: absolute;
    top: calc(var(--modal-padding) * var(--ui-scale, 1));
    inset-inline-end: calc(
      (var(--modal-padding) - var(--modal-close-slack)) * var(--ui-scale, 1)
    );
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
    color: var(--text-secondary);
    cursor: pointer;
    box-sizing: border-box;
    transition: color var(--duration-fast) var(--ease-standard),
                background-color var(--duration-fast) var(--ease-standard);
  }

  .modal-close-btn :global(svg) {
    width: calc(var(--modal-close-icon) * var(--ui-scale, 1));
    height: calc(var(--modal-close-icon) * var(--ui-scale, 1));
  }

  @media (hover: hover) {
    .modal-close-btn:hover {
      color: var(--text-primary);
      background: var(--state-layer-hover);
    }
  }

  .modal-close-btn:active {
    background: var(--state-layer-pressed);
  }

  .modal-close-btn:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -1px;
  }

  .modal-overlay.mobile {
    padding-top: calc(var(--mobile-status-bar-height, 36px) + 8px);
    padding-bottom: var(--dock-space);
    padding-left: 10px;
    padding-right: 10px;
  }

  .modal-overlay.mobile .modal-box {
    max-height: 100%;
  }

  @media (max-width: 768px) {
    .modal-overlay {
      padding-top: calc(var(--mobile-status-bar-height, 36px) + 8px);
      padding-bottom: var(--dock-space);
      padding-left: 10px;
      padding-right: 10px;
    }

    .modal-box {
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

  .modal-body.has-footer {
    padding-bottom: 0;
  }

  .modal-footer {
    display: flex;
    flex-direction: column;
    padding:
      calc(var(--modal-content-footer-gap) * var(--ui-scale, 1))
      calc(var(--modal-padding) * var(--ui-scale, 1))
      calc(var(--modal-padding) * var(--ui-scale, 1));
    border-top: 1px solid var(--floating-divider-color);
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
