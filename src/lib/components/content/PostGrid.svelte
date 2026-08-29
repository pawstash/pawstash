<script module lang="ts">
  const gridScrollPositions = new Map<string, number>();
</script>

<script lang="ts">
  import { getContext, onDestroy } from 'svelte';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import type { Post } from '$lib/types/content';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { apiSaveSettings } from '$lib/utils/ipc';
  import { i18n } from '$lib/i18n';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import PostCard from './PostCard.svelte';
  import IconEmpty from '~icons/fluent/image-off-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  interface Props {
    posts: Post[];
    loading?: boolean;
    hasMore?: boolean;
    onLoadMore?: () => void | Promise<void>;
    stateKey?: string;
    paginationKey?: string | number;
    ariaLabel?: string;
    emptyTitle?: string;
    emptyDescription?: string;
    showCreator?: boolean;
  }

  let {
    posts,
    loading = false,
    hasMore = false,
    onLoadMore,
    stateKey = 'post-grid',
    paginationKey,
    ariaLabel = i18n.t('feed.title'),
    emptyTitle = i18n.t('feed.empty'),
    emptyDescription = i18n.t('feed.empty_desc'),
    showCreator = true
  }: Props = $props();
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let scaleVisible = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let root: HTMLDivElement;
  let loadSentinel: HTMLDivElement;
  const scrollContext = getContext<ScrollableContext | undefined>(SCROLLABLE_CONTEXT);
  let viewport: HTMLElement | null = null;
  let resizeObserver: ResizeObserver | undefined;
  let intersectionObserver: IntersectionObserver | undefined;
  let viewportFrame: number | undefined;
  let gridWidth = $state(0);
  let viewportHeight = $state(0);
  let relativeScroll = $state(0);
  let gridTop = 0;
  let activeStateKey = $state('');
  let pendingScrollRestore = $state<number | null>(null);
  let lastRequestKey = '';
  const ratios = { square: '1 / 1', portrait: '4 / 5', landscape: '3 / 2', widescreen: '16 / 9' } as const;
  const ratioValues = { square: 1, portrait: 4 / 5, landscape: 3 / 2, widescreen: 16 / 9 } as const;
  const overscanRows = 3;

  let postKeys = $derived(posts.map((p) => `${p.service}:${p.user}:${p.id}`));
  let postsMap = $derived(new Map(posts.map((p) => [`${p.service}:${p.user}:${p.id}`, p])));

  $effect(() => {
    selectionState.setContext('posts', postKeys, postsMap);
  });

  let baseCardWidth = $derived(layoutState.isMobile ? 155 : 245);
  let scale = $derived(configState.settings.grid_scale / 100);
  let gap = $derived(Math.round((layoutState.isMobile ? 8 : 10) * scale));
  let targetCardWidth = $derived(baseCardWidth * scale);
  let columnCount = $derived(Math.max(1, Math.floor((gridWidth + gap) / (targetCardWidth + gap))));
  let cardWidth = $derived(columnCount > 0 ? (gridWidth - gap * (columnCount - 1)) / columnCount : gridWidth);
  let cardHeight = $derived(Math.round(cardWidth / ratioValues[configState.settings.grid_aspect_ratio]));
  let rowStride = $derived(cardHeight + gap);
  let rowCount = $derived(Math.ceil(posts.length / columnCount));
  let virtualHeight = $derived(rowCount > 0 ? rowCount * cardHeight + (rowCount - 1) * gap : 0);
  let firstRow = $derived(rowStride > 0 ? Math.max(0, Math.floor(relativeScroll / rowStride) - overscanRows) : 0);
  let lastRow = $derived(gridWidth > 0 && rowStride > 0
    ? Math.min(rowCount, Math.ceil((relativeScroll + viewportHeight) / rowStride) + overscanRows)
    : 0);
  let visibleRows = $derived.by(() => Array.from(
    { length: Math.max(0, lastRow - firstRow) },
    (_, offset) => {
      const index = firstRow + offset;
      return { index, posts: posts.slice(index * columnCount, (index + 1) * columnCount) };
    }
  ));

  function scheduleSave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => void apiSaveSettings(configState.settings), 300);
  }

  function setScale(next: number) {
    configState.settings.grid_scale = Math.max(60, Math.min(160, Math.round(next / 5) * 5));
    scaleVisible = true;
    if (hideTimer) clearTimeout(hideTimer);
    hideTimer = setTimeout(() => scaleVisible = false, 900);
    scheduleSave();
  }

  function handleWheel(event: WheelEvent) {
    if (!event.ctrlKey) return;
    event.preventDefault();
    setScale(configState.settings.grid_scale + (event.deltaY < 0 ? 5 : -5));
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!event.ctrlKey || event.key !== '0') return;
    event.preventDefault();
    setScale(100);
  }

  function updateSizeMetrics() {
    if (!root || !viewport) return;
    const viewportRect = viewport.getBoundingClientRect();
    gridWidth = root.clientWidth;
    viewportHeight = viewport.clientHeight;
    gridTop = root.getBoundingClientRect().top - viewportRect.top + viewport.scrollTop;
  }

  function updateScrollMetrics(savePosition = true) {
    if (!viewport) return;
    relativeScroll = Math.max(0, viewport.scrollTop - gridTop);
    if (savePosition) gridScrollPositions.set(activeStateKey, viewport.scrollTop);
  }

  function handleScroll() {
    updateScrollMetrics(true);
    if (pendingScrollRestore !== null && viewport) {
      if (Math.abs(viewport.scrollTop - pendingScrollRestore) > 5) {
        pendingScrollRestore = null;
      }
    }
  }

  function requestMore() {
    const requestKey = `${stateKey}:${paginationKey ?? posts.length}`;
    if (!hasMore || loading || !onLoadMore || lastRequestKey === requestKey) return;
    lastRequestKey = requestKey;
    queueMicrotask(() => void onLoadMore());
  }

  function attachViewport(nextViewport: HTMLElement | null) {
    if (viewport === nextViewport) return;
    if (viewport) viewport.removeEventListener('scroll', handleScroll);
    resizeObserver?.disconnect();
    intersectionObserver?.disconnect();
    if (viewportFrame) cancelAnimationFrame(viewportFrame);
    
    viewport = nextViewport;
    if (!viewport) return;
    
    viewport.addEventListener('scroll', handleScroll, { passive: true });
    
    resizeObserver = new ResizeObserver(() => {
      updateSizeMetrics();
      updateScrollMetrics(false);
    });
    resizeObserver.observe(viewport);
    
    intersectionObserver = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) requestMore();
      },
      { root: viewport, rootMargin: '1200px 0px' }
    );
    intersectionObserver.observe(loadSentinel);
    
    viewportFrame = requestAnimationFrame(() => {
      if (!viewport) return;
      updateSizeMetrics();
      updateScrollMetrics(true);
    });
  }

  $effect(() => {
    if (root) {
      attachViewport(scrollContext?.viewport ?? null);
    }
    return () => {
      if (viewport) viewport.removeEventListener('scroll', handleScroll);
      resizeObserver?.disconnect();
      intersectionObserver?.disconnect();
      if (viewportFrame) cancelAnimationFrame(viewportFrame);
    };
  });

  $effect(() => {
    const nextStateKey = stateKey;
    activeStateKey = nextStateKey;
    lastRequestKey = '';
    if (viewport) {
      const saved = gridScrollPositions.get(nextStateKey);
      pendingScrollRestore = saved !== undefined ? saved : 0;
    }
  });

  $effect(() => {
    if (pendingScrollRestore !== null && viewport && virtualHeight > 0) {
      const target = pendingScrollRestore;
      requestAnimationFrame(() => {
        if (!viewport || pendingScrollRestore === null) return;
        viewport.scrollTop = target;
        updateSizeMetrics();
        updateScrollMetrics(true);
        if (target === 0 || viewport.scrollTop > 0 || Math.abs(viewport.scrollTop - target) < 2) {
          pendingScrollRestore = null;
        }
      });
    }
  });

  $effect(() => {
    configState.settings.grid_scale;
    configState.settings.grid_aspect_ratio;
    if (root) {
      requestAnimationFrame(() => {
        updateSizeMetrics();
        updateScrollMetrics(false);
      });
    }
  });

  $effect(() => {
    const remaining = virtualHeight - (relativeScroll + viewportHeight);
    const nearEnd = viewportHeight > 0 && remaining <= Math.max(viewportHeight * 2, rowStride * 4);
    if (nearEnd) requestMore();
  });

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
    if (hideTimer) clearTimeout(hideTimer);
    if (viewport) {
      gridScrollPositions.set(activeStateKey, viewport.scrollTop);
      viewport.removeEventListener('scroll', handleScroll);
    }
    resizeObserver?.disconnect();
    intersectionObserver?.disconnect();
    if (viewportFrame) cancelAnimationFrame(viewportFrame);
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  bind:this={root}
  class="grid-root"
  role="feed"
  aria-label={ariaLabel}
  onwheel={handleWheel}
  style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-ratio: ${ratios[configState.settings.grid_aspect_ratio]}; --grid-gap: ${gap}px;`}
>
  {#if scaleVisible}
    <div class="scale-indicator">{configState.settings.grid_scale}%</div>
  {/if}

  {#if posts.length > 0}
    <div class="virtual-grid" style:height={`${virtualHeight}px`}>
      {#each visibleRows as row (row.index)}
        <div
          class="virtual-row"
          style={`transform: translateY(${Math.round(row.index * rowStride)}px); grid-template-columns: repeat(${columnCount}, minmax(0, 1fr)); gap: ${gap}px;`}
        >
          {#each row.posts as post (`${post.service}:${post.user}:${post.id}`)}
            <PostCard {post} {showCreator} orderedKeys={postKeys} itemsMap={postsMap} />
          {/each}
        </div>
      {/each}
    </div>
  {:else if loading}
    <div class="post-grid">
      {#each Array(12) as _}
        <div class="skeleton-card"></div>
      {/each}
    </div>
  {:else}
    <div class="empty-state">
      <IconEmpty />
      <strong>{emptyTitle}</strong>
      <span>{emptyDescription}</span>
    </div>
  {/if}

  <div class="load-sentinel" bind:this={loadSentinel} aria-hidden="true"></div>

  {#if posts.length > 0 && loading}
    <div class="loading-tail"><IconLoading />{i18n.t('feed.loading')}</div>
  {/if}
</div>

<style>
  .grid-root { position: relative; width: 100%; }
  .virtual-grid { position: relative; width: 100%; contain: layout style; }
  .virtual-row { position: absolute; top: 0; left: 0; right: 0; display: grid; align-items: start; will-change: transform; }
  .post-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--grid-card-width)), 1fr)); gap: var(--grid-gap); align-items: start; }
  .skeleton-card { aspect-ratio: var(--grid-ratio); border-radius: calc(12px * var(--grid-scale)); background: linear-gradient(105deg, rgba(255,255,255,.035) 20%, rgba(255,255,255,.08) 42%, rgba(255,255,255,.035) 64%); background-size: 220% 100%; animation: shimmer 1.4s linear infinite; }
  .scale-indicator { position: fixed; z-index: 80; left: 50%; bottom: 34px; transform: translateX(-50%); padding: 7px 12px; border: 1px solid rgba(255,255,255,.14); border-radius: 999px; background: rgba(10,10,14,.82); color: white; font-size: 12px; font-weight: 650; backdrop-filter: blur(14px); pointer-events: none; }
  .empty-state { min-height: 310px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 6px; color: rgba(255,255,255,.42); text-align: center; }
  .empty-state :global(svg) { width: 34px; height: 34px; margin-bottom: 5px; }
  .empty-state strong { color: rgba(255,255,255,.76); font-size: 14px; }
  .empty-state span { max-width: 360px; font-size: 12px; }
  .loading-tail { display: flex; align-items: center; justify-content: center; gap: 8px; min-height: 48px; color: rgba(255,255,255,.48); font-size: 11px; }
  .load-sentinel { width: 100%; height: 1px; pointer-events: none; }
  .loading-tail :global(svg) { width: 20px; height: 20px; color: white; }
  @keyframes shimmer { to { background-position: -220% 0; } }
  @media (prefers-reduced-motion: reduce) { .skeleton-card { animation: none; } }
</style>
