<script lang="ts">
  import { setContext, type Snippet } from 'svelte';
  import {
    SCROLLABLE_CONTEXT,
    scrollable as scrollAction,
    type ScrollableContext
  } from '$lib/actions/scrollable';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import PullToRefresh from '$lib/components/ui/PullToRefresh.svelte';

  interface Props {
    scrollable?: boolean;
    toolbar?: Snippet;
    children?: Snippet;
    overlay?: Snippet;
    class?: string;
    scrollKey?: string;
    onrefresh?: () => Promise<void> | void;
    onscroll?: (top: number) => void;
  }

  let {
    scrollable = true,
    toolbar,
    children,
    overlay,
    class: extraClass = '',
    scrollKey,
    onrefresh,
    onscroll
  }: Props = $props();

  const scrollContext = $state<ScrollableContext>({ viewport: null });
  setContext(SCROLLABLE_CONTEXT, scrollContext);

  let isScrolledTop = $state(false);
  let isScrolledBottom = $state(false);

  function updateScrollState(viewport: HTMLElement | null) {
    if (!viewport) return;
    const top = viewport.scrollTop;
    const scrollHeight = viewport.scrollHeight;
    const clientHeight = viewport.clientHeight;
    isScrolledTop = top > 60;
    isScrolledBottom = (scrollHeight - top - clientHeight) > 16;
  }

  let maskClass = $derived.by(() => {
    if (configState.settings.scroll_edge_mask === false) return '';
    if (isScrolledTop && isScrolledBottom) return 'mask-both';
    if (isScrolledTop) return 'mask-top';
    if (isScrolledBottom) return 'mask-bottom';
    return '';
  });
</script>

<div
  class="page-shell h-full w-full flex flex-col overflow-hidden relative animate-page-in {extraClass}"
  class:mobile={layoutState.isMobile}
>
  {#if toolbar}
    <div class="page-toolbar shrink-0">
      {@render toolbar()}
    </div>
  {/if}

  {#if scrollable}
    <div
      data-overlayscrollbars-initialize
      class="flex-1 min-h-0 relative overflow-hidden w-full page-scroll-wrapper {maskClass}"
      use:scrollAction={{
        initialScrollTop: scrollKey ? navigationState.scrollFor(scrollKey) : 0,
        onScroll: (top) => {
          if (scrollKey) navigationState.rememberScroll(scrollKey, top);
          updateScrollState(scrollContext.viewport);
          onscroll?.(top);
        },
        onReady: (viewport) => {
          scrollContext.viewport = viewport;
          updateScrollState(viewport);
        }
      }}
    >
      <div class="page-scroll-content w-full">
        {@render children?.()}
      </div>
    </div>
  {:else}
    <div class="page-static-content flex-1 min-h-0 relative overflow-hidden w-full">
      {@render children?.()}
    </div>
  {/if}

  {#if overlay}
    {@render overlay()}
  {/if}

  {#if onrefresh}
    <PullToRefresh {onrefresh} scrollContainer={scrollContext.viewport} />
  {/if}
</div>

<style>
  .page-shell {
    padding-left: 36px;
  }

  .page-toolbar {
    padding-top: 48px;
    padding-right: 36px;
  }

  .page-scroll-content,
  .page-static-content {
    padding: 48px 36px 24px 0;
  }

  .page-shell.mobile {
    padding-left: 12px;
  }

  .page-shell.mobile .page-toolbar {
    padding-top: calc(var(--mobile-status-bar-height) + 8px);
    padding-right: 12px;
  }

  .page-shell.mobile .page-scroll-content,
  .page-shell.mobile .page-static-content {
    padding: 12px 12px calc(var(--mobile-nav-height) + max(16px, env(safe-area-inset-bottom, 16px))) 0;
  }

  .page-shell.mobile:not(:has(.page-toolbar)) .page-scroll-content,
  .page-shell.mobile:not(:has(.page-toolbar)) .page-static-content {
    padding-top: calc(var(--mobile-status-bar-height) + 12px);
  }

  .page-scroll-wrapper {
    transition: -webkit-mask-image 0.25s cubic-bezier(0.16, 1, 0.3, 1), mask-image 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .page-scroll-wrapper.mask-both {
    -webkit-mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.2) 0px, black 120px, black calc(100% - 40px), rgba(0, 0, 0, 0.2) 100%);
    mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.2) 0px, black 120px, black calc(100% - 40px), rgba(0, 0, 0, 0.2) 100%);
  }

  .page-scroll-wrapper.mask-top {
    -webkit-mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.2) 0px, black 120px, black 100%);
    mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.2) 0px, black 120px, black 100%);
  }

  .page-scroll-wrapper.mask-bottom {
    -webkit-mask-image: linear-gradient(to bottom, black 0px, black calc(100% - 40px), rgba(0, 0, 0, 0.2) 100%);
    mask-image: linear-gradient(to bottom, black 0px, black calc(100% - 40px), rgba(0, 0, 0, 0.2) 100%);
  }

  .page-shell.mobile .page-scroll-wrapper.mask-both {
    -webkit-mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.2) 0px, rgba(0, 0, 0, 0.2) 20px, black 200px, black calc(100% - 48px), rgba(0, 0, 0, 0.2) 100%);
    mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.2) 0px, rgba(0, 0, 0, 0.2) 20px, black 200px, black calc(100% - 48px), rgba(0, 0, 0, 0.2) 100%);
  }

  .page-shell.mobile .page-scroll-wrapper.mask-top {
    -webkit-mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.2) 0px, rgba(0, 0, 0, 0.2) 20px, black 200px, black 100%);
    mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.2) 0px, rgba(0, 0, 0, 0.2) 20px, black 200px, black 100%);
  }

  @keyframes pageFadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .animate-page-in {
    animation: pageFadeIn 0.2s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }
</style>
