<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import MediaSurface from '$lib/components/ui/MediaSurface.svelte';
  import MaterialShape from '$lib/components/ui/MaterialShape.svelte';
  import PlayerControls from './PlayerControls.svelte';
  import { DOCK_MAX_WIDTH, DOCK_MIN_WIDTH, mediaDockState } from '$lib/state/mediaDockState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { formatBytes } from '$lib/utils/formatters';
  import { thumbnailProxySrc } from '$lib/utils/media';
  import { tooltip } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import IconChevronLeft from '~icons/fluent/chevron-left-24-regular';
  import IconChevronRight from '~icons/fluent/chevron-right-24-regular';
  import IconExpand from '~icons/fluent/arrow-maximize-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import type { MediaViewerItem } from '$lib/components/content/MediaViewer.svelte';

  interface Props {
    item: MediaViewerItem;
    index: number;
    count: number;
    onexpand: () => void;
    onclose: () => void;
    onprevious: () => void;
    onnext: () => void;
  }

  let { item, index, count, onexpand, onclose, onprevious, onnext }: Props = $props();

  const MARGIN = 16;
  const SAFE_MARGIN = 48;

  let dockEl = $state<HTMLDivElement>();
  let media = $state<HTMLMediaElement | null>(null);
  let mediaFailed = $state(false);
  let playing = $state(false);
  let viewport = $state({ w: 0, h: 0 });
  let position = $state<{ x: number; y: number } | null>(null);
  let widthPreview = $state<number | null>(null);
  let pointerNear = $state(false);
  let keyboardFocus = $state(false);

  let width = $derived(Math.max(DOCK_MIN_WIDTH, Math.min(widthPreview ?? mediaDockState.sizePx, Math.max(DOCK_MIN_WIDTH, viewport.w - MARGIN * 2))));
  let height = $derived(Math.round((width * 9) / 16));
  let bottomInset = $derived(layoutState.isMobile ? 88 : MARGIN);
  let dockSide = $derived.by((): 'left' | 'right' =>
    position && viewport.w > 0 && position.x + width / 2 < viewport.w / 2 ? 'left' : 'right'
  );
  let peekTransform = $derived(
    dockSide === 'left'
      ? `translate3d(calc(-100% + ${mediaDockState.peekPx}px - ${MARGIN}px), 0, 0)`
      : `translate3d(calc(100% - ${mediaDockState.peekPx}px + ${MARGIN}px), 0, 0)`
  );

  $effect(() => {
    void width;
    void height;
    void viewport.w;
    void viewport.h;
    if (mediaDockState.dragging || widthPreview !== null) return;
    if (viewport.w <= 0 || viewport.h <= 0) return;
    position = mediaDockState.positionFor(viewport.w, viewport.h, width, height, bottomInset, MARGIN);
  });

  let lastPreset = mediaDockState.preset;
  $effect(() => {
    const preset = mediaDockState.preset;
    if (preset === lastPreset) return;
    lastPreset = preset;
    mediaDockState.clearCustomWidth();
  });

  function attachMedia(element: HTMLMediaElement) {
    media = element;
    sync();
    if (!mediaDockState.keepPlaying && !element.paused) element.pause();
  }

  function handlePlayChange(value: boolean) {
    playing = value;
    if (value) reveal();
    else schedulePeek();
  }

  function handleMediaError() {
    media = null;
    mediaFailed = true;
    playing = false;
  }

  function sync() {
    if (!media) return;
    playing = !media.paused;
  }

  function togglePlay() {
    if (!media) return;
    if (media.paused) {
      void media.play().catch(() => {});
      reveal();
    } else {
      media.pause();
    }
    sync();
    if (media.paused) schedulePeek();
  }

  let peekTimer: ReturnType<typeof setTimeout> | undefined;
  let keyboardMode = false;
  let reduceMotion = typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  function clearPeekTimer() {
    if (peekTimer) clearTimeout(peekTimer);
    peekTimer = undefined;
  }

  function schedulePeek() {
    clearPeekTimer();
    if (playing) {
      mediaDockState.setPeeked(false);
      return;
    }
    if (!mediaDockState.autohide || reduceMotion) return;
    peekTimer = setTimeout(() => {
      if (document.querySelector('.player-settings-menu')) return;
      if (!pointerNear && !keyboardFocus && !mediaDockState.dragging && !resize && !playing) mediaDockState.setPeeked(true);
    }, mediaDockState.autohideDelayMs);
  }

  function peekOnBlur() {
    if (playing || !mediaDockState.autohide || reduceMotion || pointerNear) return;
    if (document.querySelector('.player-settings-menu')) return;
    clearPeekTimer();
    mediaDockState.setPeeked(true);
  }

  function reveal() {
    clearPeekTimer();
    mediaDockState.setPeeked(false);
  }

  function isPointerNear(x: number, y: number): boolean {
    const rect = dockEl?.getBoundingClientRect();
    if (!rect) return false;
    return (
      x >= rect.left - SAFE_MARGIN &&
      x <= rect.right + SAFE_MARGIN &&
      y >= rect.top - SAFE_MARGIN &&
      y <= rect.bottom + SAFE_MARGIN
    );
  }

  function handleWindowPointerMove(event: PointerEvent) {
    const near = isPointerNear(event.clientX, event.clientY);
    if (near) {
      if (!pointerNear) {
        pointerNear = true;
        reveal();
      }
      return;
    }
    if (pointerNear) {
      pointerNear = false;
      schedulePeek();
    }
  }

  function handlePointerEnter() {
    pointerNear = true;
    reveal();
  }

  function handlePointerLeave() {
    pointerNear = false;
    schedulePeek();
  }

  function handleFocusIn() {
    if (!keyboardMode) return;
    keyboardFocus = true;
    reveal();
  }

  function handleFocusOut(event: FocusEvent) {
    const next = event.relatedTarget as Node | null;
    if (next && dockEl?.contains(next)) return;
    keyboardFocus = false;
    schedulePeek();
  }

  let drag: { pointerId: number; offsetX: number; offsetY: number; startX: number; startY: number; moved: boolean } | null = null;
  let resize: { pointerId: number; startX: number; startWidth: number; side: 'left' | 'right' } | null = null;
  let lastTapAt = 0;

  function queueTap() {
    // Play on the first tap; the second tap of a double-click only expands the card.
    const now = performance.now();
    if (now - lastTapAt < 260) {
      lastTapAt = now;
      return;
    }
    lastTapAt = now;
    togglePlay();
  }

  function onPointerDown(event: PointerEvent) {
    if (event.button !== 0) return;
    reveal();
    const target = event.target as HTMLElement;
    if (target.closest('button, .player-controls-seek, .player-controls-volume, .dock-resize')) return;
    drag = {
      pointerId: event.pointerId,
      offsetX: event.clientX - (position?.x ?? 0),
      offsetY: event.clientY - (position?.y ?? 0),
      startX: event.clientX,
      startY: event.clientY,
      moved: false
    };
    dockEl?.setPointerCapture?.(event.pointerId);
    mediaDockState.dragging = true;
    event.preventDefault();
  }

  function onPointerMove(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointerId) return;
    if (!drag.moved && Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY) > 5) drag.moved = true;
    if (!drag.moved) return;
    const x = Math.max(0, Math.min(viewport.w - width, event.clientX - drag.offsetX));
    const y = Math.max(0, Math.min(viewport.h - height, event.clientY - drag.offsetY));
    position = { x, y };
  }

  function onPointerUp(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointerId) return;
    const moved = drag.moved;
    try {
      dockEl?.releasePointerCapture(event.pointerId);
    } catch {}
    drag = null;
    if (moved && position) {
      const side = position.x + width / 2 < viewport.w / 2 ? 'left' : 'right';
      const x = side === 'left' ? MARGIN : viewport.w - width - MARGIN;
      const y = Math.max(MARGIN, Math.min(viewport.h - height - bottomInset, position.y));
      position = { x, y };
      mediaDockState.setPosition(x, y, viewport.w, viewport.h, width, height, bottomInset, MARGIN);
    } else {
      queueTap();
    }
    mediaDockState.dragging = false;
    schedulePeek();
  }

  function onPointerCancel(event: PointerEvent) {
    if (!drag) return;
    try {
      dockEl?.releasePointerCapture(event.pointerId);
    } catch {}
    drag = null;
    mediaDockState.dragging = false;
    position = mediaDockState.positionFor(viewport.w, viewport.h, width, height, bottomInset, MARGIN);
    schedulePeek();
  }

  function onResizeDown(event: PointerEvent) {
    if (event.button !== 0) return;
    event.stopPropagation();
    event.preventDefault();
    reveal();
    resize = { pointerId: event.pointerId, startX: event.clientX, startWidth: width, side: dockSide };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function onResizeMove(event: PointerEvent) {
    if (!resize || event.pointerId !== resize.pointerId) return;
    event.stopPropagation();
    const grow = resize.side === 'left' ? event.clientX - resize.startX : resize.startX - event.clientX;
    const maxWidth = Math.max(DOCK_MIN_WIDTH, Math.min(DOCK_MAX_WIDTH, viewport.w - MARGIN * 2));
    const next = Math.max(DOCK_MIN_WIDTH, Math.min(maxWidth, resize.startWidth + grow));
    widthPreview = next;
    if (position) {
      const x = resize.side === 'left' ? MARGIN : viewport.w - next - MARGIN;
      const y = Math.max(MARGIN, Math.min(viewport.h - Math.round((next * 9) / 16) - MARGIN, position.y));
      position = { x, y };
    }
  }

  function onResizeUp(event: PointerEvent) {
    if (!resize) return;
    const side = resize.side;
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    } catch {}
    resize = null;
    const finalWidth = widthPreview ?? width;
    const finalHeight = Math.round((finalWidth * 9) / 16);
    const x = side === 'left' ? MARGIN : viewport.w - finalWidth - MARGIN;
    const y = Math.max(MARGIN, Math.min(viewport.h - finalHeight - bottomInset, position?.y ?? MARGIN));
    position = { x, y };
    mediaDockState.setWidth(finalWidth);
    mediaDockState.setPosition(x, y, viewport.w, viewport.h, finalWidth, finalHeight, bottomInset, MARGIN);
    widthPreview = null;
    schedulePeek();
  }

  function handleKeydown(event: KeyboardEvent) {
    keyboardMode = true;
    if (event.target !== dockEl) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      onclose();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      onexpand();
    } else if (event.key === ' ') {
      event.preventDefault();
      togglePlay();
    }
  }

  // svelte-ignore state_referenced_locally
  let lastItemKey = $state(item.id || item.url);
  $effect(() => {
    const key = item.id || item.url;
    if (key === lastItemKey) return;
    lastItemKey = key;
    media = null;
    mediaFailed = false;
    playing = false;
  });

  $effect(() => {
    void mediaDockState.autohide;
    void mediaDockState.autohideDelayMs;
    schedulePeek();
  });

  $effect(() => {
    if (playing) reveal();
    else schedulePeek();
  });

  let cleanupFrame: number | undefined;

  onMount(() => {
    const onResize = () => {
      viewport = { w: window.innerWidth, h: window.innerHeight };
      mediaDockState.setPeeked(false);
      schedulePeek();
    };
    const markKeyboard = () => (keyboardMode = true);
    const markPointer = () => (keyboardMode = false);
    window.addEventListener('resize', onResize);
    window.addEventListener('keydown', markKeyboard, true);
    window.addEventListener('pointerdown', markPointer, true);
    window.addEventListener('pointermove', handleWindowPointerMove, true);
    window.addEventListener('blur', peekOnBlur);
    window.addEventListener('focus', reveal);
    requestAnimationFrame(() => {
      viewport = { w: window.innerWidth, h: window.innerHeight };
    });
    mediaDockState.setPeeked(false);
    const tickDock = () => {
      sync();
      cleanupFrame = requestAnimationFrame(tickDock);
    };
    cleanupFrame = requestAnimationFrame(tickDock);
    schedulePeek();
    return () => {
      window.removeEventListener('resize', onResize);
      window.removeEventListener('keydown', markKeyboard, true);
      window.removeEventListener('pointerdown', markPointer, true);
      window.removeEventListener('pointermove', handleWindowPointerMove, true);
      window.removeEventListener('blur', peekOnBlur);
      window.removeEventListener('focus', reveal);
      if (cleanupFrame) cancelAnimationFrame(cleanupFrame);
      clearPeekTimer();
    };
  });

  onDestroy(() => {
    clearPeekTimer();
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  bind:this={dockEl}
  class="media-dock"
  class:is-peeked={mediaDockState.peeked}
  class:is-dragging={mediaDockState.dragging}
  class:is-ready={position !== null}
  class:is-audio={item.kind === 'audio'}
  class:peek-left={dockSide === 'left'}
  class:peek-right={dockSide === 'right'}
  style:left={`${position?.x ?? 0}px`}
  style:top={`${position?.y ?? 0}px`}
  style:width={`${width}px`}
  style:height={`${height}px`}
  style:transform={mediaDockState.peeked ? peekTransform : undefined}
  role="complementary"
  aria-label={`${i18n.t('post.dock_title')}: ${item.name}`}
  tabindex="0"
  onkeydown={handleKeydown}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerCancel}
  onmouseenter={handlePointerEnter}
  onmouseleave={handlePointerLeave}
  onfocusin={handleFocusIn}
  onfocusout={handleFocusOut}
  ondblclick={() => {
    onexpand();
  }}
>
  <div class="dock-media">
    {#if item.kind === 'video' && item.url}
      <MediaSurface
        mediaKey={item.id || item.url}
        kind="video"
        src={item.url}
        poster={item.poster}
        autoplay
        resume
        priority
        controls={false}
        onmedia={attachMedia}
        onerror={handleMediaError}
      />
    {:else if item.kind === 'image'}
      <img src={(thumbnailProxySrc(item.url || item.poster, 800) ?? item.url) || item.poster} alt={item.name} draggable="false" onerror={(e) => { const img = e.currentTarget as HTMLImageElement; if (item.poster && img.src !== item.poster) img.src = item.poster; }} />
    {:else}
      {#if item.poster}
        <img class="dock-art" src={item.poster} alt="" draggable="false" />
      {/if}
      <div class="dock-art-shade"></div>
      <div class="dock-art-icon">
        <MaterialShape name={item.kind === 'audio' ? 'cookie9Sided' : 'square'} size={64} />
        {#if item.kind === 'audio'}<IconMusic />{:else}<IconDocument />{/if}
      </div>
      {#if item.kind === 'audio' && item.url}
        <div class="dock-audio-host" aria-hidden="true">
          <MediaSurface
            mediaKey={item.id || item.url}
            kind="audio"
            src={item.url}
            autoplay
            resume
            priority
            controls={false}
            onmedia={attachMedia}
            onerror={handleMediaError}
          />
        </div>
      {/if}
    {/if}
    {#if mediaFailed}
      <div class="dock-error">
        <span>{i18n.t('post.video_load_failed')}</span>
        <button type="button" onclick={onexpand}>{i18n.t('post.dock_expand')}</button>
      </div>
    {/if}
  </div>

  <div class="dock-head">
    <strong class="dock-name">{item.name}</strong>
  </div>

  {#if count > 1}
    <button type="button" class="dock-nav dock-nav-prev" onclick={onprevious} use:tooltip={i18n.t('post.previous')} aria-label={i18n.t('post.previous')}><IconChevronLeft /></button>
    <button type="button" class="dock-nav dock-nav-next" onclick={onnext} use:tooltip={i18n.t('post.next')} aria-label={i18n.t('post.next')}><IconChevronRight /></button>
  {/if}

  <button type="button" class="dock-close" onclick={onclose} use:tooltip={i18n.t('post.viewer_close')} aria-label={i18n.t('post.viewer_close')}><IconDismiss /></button>

  <div class="dock-controls">
    {#snippet dockActions()}
      {#if item.kind === 'file' && item.size}
        <span class="dock-file-size">{formatBytes(item.size)}</span>
      {/if}
      <button type="button" class="player-controls-btn" onclick={onexpand} use:tooltip={i18n.t('post.dock_expand')} aria-label={i18n.t('post.dock_expand')}><IconExpand /></button>
    {/snippet}
    <PlayerControls
      media={media}
      playable={item.kind === 'video' || item.kind === 'audio'}
      timePosition="meta"
      compact={width < 380}
      actions={dockActions}
      onactivity={reveal}
      onplaychange={handlePlayChange}
    />
  </div>

  <span
    class="dock-resize"
    role="presentation"
    onpointerdown={onResizeDown}
    onpointermove={onResizeMove}
    onpointerup={onResizeUp}
    onpointercancel={onResizeUp}
  ></span>
</div>

<style>
  .media-dock {
    position: fixed;
    z-index: var(--z-mini-player, 1500);
    container-type: inline-size;
    overflow: hidden;
    border-radius: var(--radius-lg);
    background: var(--surface-container);
    color: var(--player-ink);
    box-shadow: var(--floating-shadow);
    transition: transform 320ms cubic-bezier(0.2, 0, 0, 1), left 260ms cubic-bezier(0.2, 0, 0, 1), top 260ms cubic-bezier(0.2, 0, 0, 1), opacity 120ms ease;
    will-change: transform;
    backface-visibility: hidden;
    touch-action: none;
    user-select: none;
    outline: none;
  }

  .media-dock:not(.is-ready) {
    opacity: 0;
  }

  .media-dock.is-dragging {
    transition: none;
    cursor: grabbing;
  }

  .media-dock:focus-visible {
    box-shadow: 0 0 0 2px var(--accent-primary), var(--floating-shadow);
  }

  .dock-media {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: var(--player-backdrop);
  }

  .media-dock.is-audio .dock-media {
    background: linear-gradient(150deg, var(--surface-container-high), var(--surface-container-low));
  }

  .dock-media :global(video) {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    background: var(--player-backdrop);
    pointer-events: none;
  }

  .dock-media > img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    pointer-events: none;
  }

  .dock-media > img:not(.dock-art) {
    object-fit: contain;
    background: var(--player-backdrop);
  }

  .dock-media > .dock-art {
    filter: blur(1px) brightness(0.55);
    transform: scale(1.04);
  }

  .dock-art-shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(160deg, color-mix(in srgb, var(--accent-primary) 20%, transparent), transparent 55%);
  }

  .dock-art-icon {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: color-mix(in srgb, var(--accent-primary) 45%, transparent);
  }

  .dock-art-icon :global(.material-shape) {
    position: absolute;
  }

  .dock-art-icon :global(svg:not(.material-shape)) {
    position: relative;
    width: 34px;
    height: 34px;
    color: var(--accent-primary);
  }

  .dock-audio-host {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    opacity: 0;
    pointer-events: none;
  }

  .dock-head {
    position: absolute;
    inset: 0 0 auto;
    z-index: 2;
    display: flex;
    align-items: flex-start;
    padding: var(--space-2) calc(var(--control-height) + var(--space-2)) var(--space-4) var(--space-3);
    background: linear-gradient(to bottom, color-mix(in srgb, var(--player-fade-color) 62%, transparent), transparent);
    opacity: 1;
    transition: opacity 160ms ease;
    pointer-events: none;
  }

  .dock-name {
    overflow: hidden;
    color: var(--player-ink);
    font-family: var(--font-display);
    font-size: var(--type-title-sm-size);
    line-height: var(--type-title-sm-line);
    font-weight: 400;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.8);
  }

  .dock-file-size {
    flex: none;
    margin-inline-end: var(--space-1);
    color: var(--player-ink-secondary);
    font-family: var(--font-sans);
    font-size: var(--type-body-sm-size);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
  }

  .dock-controls {
    position: absolute;
    inset: auto 0 0;
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4) var(--space-3) var(--space-3);
    background: linear-gradient(to top, color-mix(in srgb, var(--player-fade-color) 72%, transparent), transparent);
    opacity: 1;
    transition: opacity 160ms ease;
  }

  .media-dock:hover .dock-controls,
  .media-dock:focus-within .dock-controls {
    opacity: 1;
  }

  .media-dock:hover .dock-controls,
  .media-dock:focus-within .dock-controls {
    pointer-events: auto;
  }

  .dock-close {
    position: absolute;
    top: var(--space-1);
    right: var(--space-1);
    z-index: 4;
    display: grid;
    place-items: center;
    width: var(--control-height);
    height: var(--control-height);
    padding: 0;
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--player-ink-secondary);
    cursor: default;
    transition: background-color var(--duration-fast) var(--ease-standard), color var(--duration-fast) var(--ease-standard);
  }

  .dock-close:hover {
    background: var(--player-state-hover);
    color: var(--player-ink);
  }

  .dock-close :global(svg) {
    width: var(--control-icon-size);
    height: var(--control-icon-size);
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.6));
  }

  .dock-nav {
    position: absolute;
    top: 50%;
    z-index: 3;
    display: grid;
    place-items: center;
    width: var(--control-height-lg);
    height: var(--control-height-lg);
    padding: 0;
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--player-ink-secondary);
    transform: translateY(-50%);
    cursor: default;
    opacity: 0;
    transition: opacity var(--duration-fast) ease, background-color var(--duration-fast) var(--ease-standard), color var(--duration-fast) var(--ease-standard), transform var(--duration-fast) var(--ease-standard);
  }

  .dock-nav-prev {
    left: 2px;
  }

  .dock-nav-next {
    right: 2px;
  }

  .media-dock:hover .dock-nav,
  .media-dock:focus-within .dock-nav {
    opacity: 1;
  }

  .dock-nav:hover {
    background: var(--player-state-hover);
    color: var(--player-ink);
    transform: translateY(-50%) scale(1.08);
  }

  .dock-nav:active {
    background: var(--player-state-pressed);
    transform: translateY(-50%) scale(0.96);
  }

  .dock-nav :global(svg) {
    width: var(--control-icon-lg);
    height: var(--control-icon-lg);
    filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.75));
  }

  .dock-close:focus-visible,
  .dock-nav:focus-visible,
  .dock-resize:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: 2px;
  }

  .dock-error {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-4);
    background: var(--player-scrim-strong);
    color: var(--player-ink);
    text-align: center;
  }

  .dock-error span {
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
  }

  .dock-error button {
    height: var(--control-height-sm);
    padding-inline: var(--space-3);
    border: 0;
    border-radius: var(--radius-full);
    background: var(--player-state-hover);
    color: var(--player-ink);
    font-size: var(--type-label-lg-size);
    cursor: pointer;
  }

  .dock-error button:hover {
    background: var(--player-state-pressed);
  }

  .dock-resize {
    position: absolute;
    top: 0;
    bottom: 0;
    z-index: 1;
    width: 12px;
    cursor: ew-resize;
  }

  .media-dock.peek-left .dock-resize {
    right: 0;
  }

  .media-dock.peek-right .dock-resize {
    left: 0;
  }

  .dock-resize::after {
    content: '';
    position: absolute;
    top: 50%;
    width: 3px;
    height: 40px;
    border-radius: var(--radius-full);
    background: var(--player-ink-secondary);
    transform: translateY(-50%);
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-standard), background-color var(--duration-fast) var(--ease-standard);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
    pointer-events: none;
  }

  .media-dock.peek-left .dock-resize::after {
    right: 4px;
  }

  .media-dock.peek-right .dock-resize::after {
    left: 4px;
  }

  .media-dock:hover .dock-resize::after {
    opacity: 0.55;
  }

  .dock-resize:hover::after,
  .dock-resize:active::after {
    opacity: 1;
    background: var(--player-ink);
  }

  .media-dock:hover .dock-resize,
  .media-dock:focus-within .dock-resize {
    opacity: 1;
  }

  .media-dock.is-peeked .dock-head,
  .media-dock.is-peeked .dock-controls,
  .media-dock.is-peeked .dock-close,
  .media-dock.is-peeked .dock-nav,
  .media-dock.is-peeked .dock-resize {
    opacity: 0;
    pointer-events: none;
  }

  .media-dock.is-peeked::after {
    content: '';
    position: absolute;
    top: 50%;
    width: 3px;
    height: 40px;
    border-radius: var(--radius-full);
    background: var(--accent-primary);
    transform: translateY(-50%);
  }

  .media-dock.is-peeked.peek-left::after {
    right: 4px;
  }

  .media-dock.is-peeked.peek-right::after {
    left: 4px;
  }

  @container (max-width: 380px) {
    .dock-name {
      font-size: var(--type-body-sm-size);
    }

    .dock-controls {
      padding: var(--space-3) var(--space-2) var(--space-2);
    }

    .dock-close {
      width: var(--control-height-sm);
      height: var(--control-height-sm);
    }

    .dock-close :global(svg) {
      width: var(--control-icon-sm);
      height: var(--control-icon-sm);
    }

    .dock-nav {
      width: var(--control-height);
      height: var(--control-height);
    }
  }

  @media (hover: none) {
    .dock-controls,
    .dock-nav {
      opacity: 1;
    }

    .dock-controls {
      pointer-events: auto;
    }

    .dock-resize {
      display: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .media-dock,
    .media-dock.is-peeked {
      transition: none;
      transform: none;
    }
  }
</style>
