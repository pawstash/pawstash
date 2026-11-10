<script lang="ts">
  import { setContext, type Snippet } from 'svelte';
  import {
    SCROLLABLE_CONTEXT,
    scrollable as scrollAction,
    type ScrollableContext
  } from '$lib/actions/scrollable';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import PullToRefresh from '$lib/components/ui/PullToRefresh.svelte';
  import TopAppBar from './TopAppBar.svelte';
  import type { Crumb } from './TopAppBar.svelte';
  import type { MenuItem } from './OverflowMenu.svelte';

  interface Props {
    title?: string;
    trail?: Crumb[];
    heading?: Snippet<[boolean]>;
    titleAfter?: number;
    titleTrigger?: HTMLElement | null;
    largeTitle?: boolean;
    back?: boolean | (() => void);
    views?: Snippet;
    filter?: Snippet;
    actions?: Snippet;
    menu?: MenuItem[];
    selectionActions?: Snippet;
    selectionTotal?: number;
    onSelectAll?: () => void;
    hideOnScroll?: boolean;
    scrollable?: boolean;
    children?: Snippet;
    overlay?: Snippet;
    class?: string;
    scrollKey?: string;
    onrefresh?: () => Promise<void> | void;
    onscroll?: (top: number) => void;
    viewport?: HTMLElement | null;
  }

  let {
    title,
    trail,
    heading,
    titleAfter,
    titleTrigger,
    largeTitle = false,
    back,
    views,
    filter,
    actions,
    menu,
    selectionActions,
    selectionTotal = 0,
    onSelectAll,
    hideOnScroll,
    scrollable = true,
    children,
    overlay,
    class: extraClass = '',
    scrollKey,
    onrefresh,
    onscroll,
    viewport = $bindable(null)
  }: Props = $props();

  const scrollContext = $state<ScrollableContext>({ viewport: null });
  setContext(SCROLLABLE_CONTEXT, scrollContext);

  let barHeight = $state(0);
  let barHidden = $state(false);
  let largeTitleHeight = $state(0);
  let scrolled = $state(false);
  let showLargeTitle = $derived(largeTitle && layoutState.isMobile && Boolean(title));

  let fades = $derived(!showLargeTitle);
  let fadeTop = $derived(
    !layoutState.isMobile ? '0px' : barHidden ? 'var(--mobile-status-bar-height)' : `${barHeight}px`
  );
</script>

<div
  class="page-shell animate-page-in {extraClass}"
  class:mobile={layoutState.isMobile}
  style:--page-bar-space={layoutState.isMobile ? `${barHeight}px` : '0px'}
>
  <TopAppBar
    {title}
    {trail}
    {heading}
    titleAfter={showLargeTitle ? largeTitleHeight : titleAfter}
    {titleTrigger}
    {back}
    {views}
    {filter}
    {actions}
    {menu}
    selection={selectionActions ? { total: selectionTotal, onSelectAll, actions: selectionActions } : undefined}
    hideOnScroll={showLargeTitle ? false : hideOnScroll}
    solid={showLargeTitle}
    bind:height={barHeight}
    bind:hidden={barHidden}
  />

  {#if scrollable}
    <div
      data-overlayscrollbars-initialize
      class="page-body"
      class:fades
      class:is-scrolled={scrolled}
      style:--fade-top={fadeTop}
      use:scrollAction={{
        nativeOnTouch: true,
        initialScrollTop: scrollKey ? navigationState.scrollFor(scrollKey) : 0,
        onScroll: (top) => {
          scrolled = top > 0;
          if (scrollKey) navigationState.rememberScroll(scrollKey, top);
          onscroll?.(top);
        },
        onReady: (vp) => {
          scrollContext.viewport = vp;
          viewport = vp;
          scrolled = (vp?.scrollTop ?? 0) > 0;
        }
      }}
    >
      <div class="page-content">
        {#if showLargeTitle}
          <h1 class="page-large-title" bind:clientHeight={largeTitleHeight}>{title}</h1>
        {/if}
        {@render children?.()}
      </div>
    </div>
  {:else}
    <div class="page-body page-content is-static">
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
  @property --fade-top {
    syntax: '<length>';
    inherits: false;
    initial-value: 0px;
  }

  @property --fade-size {
    syntax: '<length>';
    inherits: false;
    initial-value: 0px;
  }

  .page-shell {
    --content-top: 24px;
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-width: 0;
    overflow: hidden;
  }

  .page-body {
    position: relative;
    flex: 1 1 0;
    min-height: 0;
    width: 100%;
    padding-inline-end: var(--edge-right, 0px);
    overflow: hidden;
  }

  .page-body.fades {
    -webkit-mask-image: linear-gradient(to bottom, transparent 0, #000 var(--content-top));
    mask-image: linear-gradient(to bottom, transparent 0, #000 var(--content-top));
  }

  .page-shell.mobile .page-body.fades {
    --fade-size: 0px;
    -webkit-mask-image: linear-gradient(to bottom, transparent var(--fade-top), #000 calc(var(--fade-top) + var(--fade-size)));
    mask-image: linear-gradient(to bottom, transparent var(--fade-top), #000 calc(var(--fade-top) + var(--fade-size)));
    transition:
      --fade-top var(--duration-normal) var(--ease-standard),
      --fade-size var(--duration-fast) var(--ease-standard);
  }

  .page-shell.mobile .page-body.fades.is-scrolled {
    --fade-size: 32px;
  }

  /* WebKitGTK repaints a masked ancestor of the scroller offscreen on every scroll frame. */
  :global(html.webkitgtk) .page-body.fades {
    -webkit-mask-image: none;
    mask-image: none;
  }

  :global(html.webkitgtk:not([data-backdrop])) .page-body.fades::after {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 1;
    height: var(--content-top);
    background: linear-gradient(to bottom, var(--surface), transparent);
    pointer-events: none;
  }

  .page-content {
    box-sizing: border-box;
    width: 100%;
    padding: var(--content-top) var(--gutter) 24px;
  }

  .page-shell.mobile .page-content {
    padding-top: calc(var(--page-bar-space) + 8px);
    padding-bottom: var(--dock-space);
  }

  .page-large-title {
    margin: 0;
    padding: 16px 0 24px;
    color: var(--text-primary);
    font-family: var(--font-display);
    font-size: var(--type-headline-size);
    line-height: var(--type-headline-line);
    font-weight: 400;
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
    animation: pageFadeIn var(--duration-normal) var(--ease-expo) backwards;
  }
</style>
