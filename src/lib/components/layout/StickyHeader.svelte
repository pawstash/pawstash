<script lang="ts">
  import { getContext, type Snippet } from 'svelte';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { i18n } from '$lib/i18n';
  import Button from '$lib/components/ui/Button.svelte';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';

  interface Props {
    title?: string | Snippet;
    back?: boolean | (() => void);
    leading?: Snippet;
    center?: Snippet;
    trailing?: Snippet;
    children?: Snippet;
    threshold?: number;
    class?: string;
  }

  let {
    title,
    back = false,
    leading,
    center,
    trailing,
    children,
    threshold = 100,
    class: extraClass = ''
  }: Props = $props();

  const scrollContext = getContext<ScrollableContext | undefined>(SCROLLABLE_CONTEXT);
  let isVisible = $state(false);

  function handleScroll(e: Event) {
    const target = e.currentTarget as HTMLElement;
    isVisible = target.scrollTop > threshold;
  }

  function handleBack() {
    if (typeof back === 'function') {
      back();
    } else {
      navigationState.back();
    }
  }

  $effect(() => {
    if (!configState.settings.sticky_header) {
      isVisible = false;
      return undefined;
    }

    const viewport = scrollContext?.viewport;
    if (viewport) {
      viewport.addEventListener('scroll', handleScroll, { passive: true });
      isVisible = viewport.scrollTop > threshold;
      return () => {
        viewport.removeEventListener('scroll', handleScroll);
      };
    }
    return undefined;
  });
</script>

{#if configState.settings.sticky_header}
  <div
    class="sticky-header-bar {extraClass}"
    class:visible={isVisible}
    class:is-mobile={layoutState.isMobile}
  >
    <div class="sticky-header-content">
      {#if children}
        {@render children()}
      {:else}
        <div class="sticky-leading-zone">
          {#if back}
            <Button variant="ghost" size="sm" onclick={handleBack} class="btn-icon">
              <IconArrowLeft class="w-4 h-4" />
            </Button>
          {/if}

          {#if title}
            {#if typeof title === 'string'}
              <span class="sticky-title-text">{title}</span>
            {:else}
              {@render title()}
            {/if}
          {/if}

          {#if leading}
            {@render leading()}
          {/if}

          {#if center && !layoutState.isMobile}
            <div class="sticky-inline-tabs">
              {@render center()}
            </div>
          {/if}
        </div>

        {#if trailing}
          <div class="sticky-trailing-zone">
            {@render trailing()}
          </div>
        {/if}
      {/if}
    </div>
  </div>
{/if}

<style>
  .sticky-header-bar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 86px;
    padding-top: 30px;
    background: transparent;
    border-bottom: none;
    z-index: 1000;
    transform: translateY(-100%);
    opacity: 0;
    pointer-events: none;
    mix-blend-mode: normal;
    transition: transform var(--duration-normal) var(--ease-expo),
                opacity var(--duration-normal) var(--ease-expo);
  }

  .sticky-header-bar.visible {
    transform: translateY(0);
    opacity: 1;
    pointer-events: auto;
  }

  .sticky-header-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 56px;
    padding: 0 36px;
    gap: 16px;
    position: relative;
  }

  .sticky-leading-zone {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .sticky-title-text {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    letter-spacing: -0.2px;
  }

  .sticky-inline-tabs {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .sticky-trailing-zone {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    flex-shrink: 0;
  }

  .sticky-header-bar.is-mobile .sticky-header-content:has(:global(.search-active)) .sticky-leading-zone,
  .sticky-header-bar.is-mobile .sticky-header-content:has(:global(.search-active)) .sticky-inline-tabs {
    display: none !important;
  }

  .sticky-header-bar.is-mobile .sticky-header-content:has(:global(.search-active)) .sticky-trailing-zone {
    width: 100%;
    flex: 1;
  }

  .sticky-header-bar.is-mobile {
    height: calc(56px + var(--mobile-status-bar-height));
    padding-top: var(--mobile-status-bar-height);
  }

  .sticky-header-bar.is-mobile .sticky-header-content {
    height: 56px;
    padding: 0 16px;
    gap: 8px;
  }
</style>
