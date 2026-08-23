<script module lang="ts">
  export type MediaViewerKind = 'image' | 'video' | 'audio' | 'file';

  export interface MediaViewerItem {
    id: string;
    url: string;
    name: string;
    kind: MediaViewerKind;
    size?: number;
    poster?: string;
    width?: number;
    height?: number;
    html?: string;
    downloadStatus?: 'queued' | 'resolving' | 'downloading' | 'paused' | 'verifying' | 'completed' | 'failed' | 'cancelled' | 'missing';
    downloadedBytes?: number;
    totalBytes?: number;
  }
</script>

<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { portal } from '$lib/actions/portal';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import { tooltip } from '$lib/motion';
  import { playbackState } from '$lib/state/playbackState.svelte';
  import { handleGlobalPanicKey, panicCapture } from '$lib/utils/panic';
  import Button from '$lib/components/ui/Button.svelte';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconChevronLeft from '~icons/fluent/chevron-left-24-regular';
  import IconChevronRight from '~icons/fluent/chevron-right-24-regular';
  import IconZoomIn from '~icons/fluent/zoom-in-24-regular';
  import IconZoomOut from '~icons/fluent/zoom-out-24-regular';
  import IconArrowReset from '~icons/fluent/arrow-reset-24-regular';
  import IconFullscreen from '~icons/fluent/full-screen-maximize-24-regular';
  import IconFullscreenExit from '~icons/fluent/full-screen-minimize-24-regular';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconCheck from '~icons/fluent/checkmark-20-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';

  interface Props {
    items: MediaViewerItem[];
    initialIndex?: number;
    initialTime?: number;
    onclose: (finalIndex?: number, finalTime?: number) => void;
    ondownload?: (item: MediaViewerItem, index: number) => void | Promise<void>;
  }

  let { items, initialIndex = 0, initialTime = 0, onclose, ondownload }: Props = $props();

  const MIN_SCALE = 1;
  const MAX_SCALE = 8;
  let index = $state(0);
  let scale = $state(1);
  let translateX = $state(0);
  let translateY = $state(0);
  let swipeOffset = $state(0);
  let dismissOffsetY = $state(0);
  let isDismissing = $state(false);
  let dismissScale = $derived(1 - Math.min(0.25, Math.abs(dismissOffsetY) * 0.0005));
  let dismissOpacity = $derived(Math.max(0.1, 1 - Math.abs(dismissOffsetY) / 350));
  let controlsVisible = $state(true);
  let fullscreen = $state(false);
  let root = $state<HTMLDivElement>();
  let stage = $state<HTMLDivElement>();
  let fitFrame = $state<HTMLDivElement>();
  let mediaElement = $state<HTMLImageElement>();
  let videoElement = $state<HTMLVideoElement>();
  let loadedWidth = $state(0);
  let loadedHeight = $state(0);
  let controlsTimer: ReturnType<typeof setTimeout> | undefined;
  let closing = false;
  let hasAppliedInitialTime = false;

  function handleVideoMetadata(e: Event) {
    const video = e.currentTarget as HTMLVideoElement;
    if (video.videoWidth > 0 && video.videoHeight > 0) {
      loadedWidth = video.videoWidth;
      loadedHeight = video.videoHeight;
    }
    if (!hasAppliedInitialTime && initialTime > 0 && index === initialIndex) {
      hasAppliedInitialTime = true;
      video.currentTime = initialTime;
    } else if (current) {
      const saved = playbackState.getTime(current.id || current.url);
      if (saved !== undefined && saved > 0) {
        video.currentTime = saved;
      }
    }
  }

  function handleVideoTimeUpdate(e: Event) {
    const video = e.currentTarget as HTMLVideoElement;
    if (current && video.duration > 0) {
      playbackState.saveTime(current.id || current.url, video.currentTime, video.duration);
    }
  }

  function handleVideoEnded() {
    if (current) {
      playbackState.clearTime(current.id || current.url);
    }
  }

  const unregisterBack = navigationState.registerBackHandler(() => {
    close();
    return true;
  });

  onDestroy(() => {
    unregisterBack();
  });

  type PointerPoint = { x: number; y: number; startX: number; startY: number; startedAt: number };
  const pointers = new Map<number, PointerPoint>();

  let current = $derived(items[index]);
  let currentId = $derived(current?.id);
  let currentWidth = $derived(current?.width || loadedWidth);
  let currentHeight = $derived(current?.height || loadedHeight);
  let currentResolution = $derived(currentWidth > 0 && currentHeight > 0 ? `${currentWidth} × ${currentHeight}` : '');
  let currentDownloadActive = $derived(Boolean(current && ['queued', 'resolving', 'downloading', 'paused', 'verifying'].includes(current.downloadStatus || '')));
  let currentDownloaded = $derived(current?.downloadStatus === 'completed');
  let currentDownloadBytes = $derived(Math.max(current?.totalBytes || 0, current?.downloadedBytes || 0, current?.size || 0));
  let currentDownloadProgress = $derived(current?.totalBytes ? Math.min(100, Math.round((current.downloadedBytes || 0) / current.totalBytes * 100)) : 0);
  let transform = $derived(`translate3d(${translateX + swipeOffset}px, ${translateY + dismissOffsetY}px, 0) scale(${scale * dismissScale})`);
  let visibleThumbnails = $derived.by(() => {
    const count = Math.min(9, items.length);
    const start = Math.max(0, Math.min(index - Math.floor(count / 2), items.length - count));
    return items.slice(start, start + count).map((item, offset) => ({ item, index: start + offset }));
  });

  function resetTransform() {
    scale = MIN_SCALE;
    translateX = 0;
    translateY = 0;
    swipeOffset = 0;
    dismissOffsetY = 0;
    isDismissing = false;
    pointers.clear();
  }

  function clampTranslation(nextX = translateX, nextY = translateY) {
    if (!fitFrame || !mediaElement || scale <= MIN_SCALE) {
      translateX = 0;
      translateY = 0;
      return;
    }
    const maxX = Math.max(0, (mediaElement.clientWidth * scale - fitFrame.clientWidth) / 2);
    const maxY = Math.max(0, (mediaElement.clientHeight * scale - fitFrame.clientHeight) / 2);
    translateX = Math.max(-maxX, Math.min(maxX, nextX));
    translateY = Math.max(-maxY, Math.min(maxY, nextY));
  }

  function setScale(nextScale: number, clientX?: number, clientY?: number) {
    if (current?.kind !== 'image') return;
    const clamped = Math.max(MIN_SCALE, Math.min(MAX_SCALE, nextScale));
    if (!fitFrame || clamped === scale) return;

    if (clientX !== undefined && clientY !== undefined) {
      const rect = fitFrame.getBoundingClientRect();
      const cursorX = clientX - (rect.left + rect.width / 2);
      const cursorY = clientY - (rect.top + rect.height / 2);
      const ratio = clamped / scale;
      translateX = cursorX - (cursorX - translateX) * ratio;
      translateY = cursorY - (cursorY - translateY) * ratio;
    }

    scale = clamped;
    requestAnimationFrame(() => clampTranslation());
  }

  function navigate(delta: number) {
    if (items.length < 2) return;
    index = (index + delta + items.length) % items.length;
  }

  function select(nextIndex: number) {
    if (nextIndex === index) return;
    index = nextIndex;
  }

  function handleImageLoad(event: Event) {
    const image = event.currentTarget as HTMLImageElement;
    loadedWidth = image.naturalWidth;
    loadedHeight = image.naturalHeight;
  }

  function requestDownload() {
    if (!current || currentDownloadActive || currentDownloaded) return;
    void ondownload?.(current, index);
  }

  function registerActivity() {
    controlsVisible = true;
    if (controlsTimer) clearTimeout(controlsTimer);
    controlsTimer = setTimeout(() => {
      if (document.activeElement?.closest('.media-viewer-controls')) return;
      controlsVisible = false;
    }, 3200);
  }

  let wheelNavThrottle = 0;

  function handleWheel(event: WheelEvent) {
    // 1. Pinch or Ctrl+Wheel or when zoomed in -> Zoom image
    if (current?.kind === 'image' && (event.ctrlKey || scale > MIN_SCALE)) {
      event.preventDefault();
      const factor = Math.exp(-event.deltaY * 0.002);
      setScale(scale * factor, event.clientX, event.clientY);
      registerActivity();
      return;
    }

    // 2. Trackpad horizontal swipe, Shift+Wheel or mouse horizontal tilt -> Navigate media
    if (scale <= MIN_SCALE + 0.05 && items.length > 1) {
      const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : (event.shiftKey ? event.deltaY : 0);
      if (Math.abs(delta) > 25) {
        event.preventDefault();
        const now = performance.now();
        if (now - wheelNavThrottle > 320) {
          wheelNavThrottle = now;
          navigate(delta > 0 ? 1 : -1);
          registerActivity();
        }
        return;
      }
    }

    // 3. Normal vertical wheel on image -> Zoom
    if (current?.kind === 'image') {
      event.preventDefault();
      const factor = Math.exp(-event.deltaY * 0.0015);
      setScale(scale * factor, event.clientX, event.clientY);
      registerActivity();
    }
  }

  function handleDoubleClick(event: MouseEvent) {
    if (current?.kind !== 'image') return;
    setScale(scale > 1.05 ? 1 : 2.5, event.clientX, event.clientY);
    registerActivity();
  }

  function handlePointerDown(event: PointerEvent) {
    registerActivity();
    const target = event.target as HTMLElement;
    // Don't intercept clicks on buttons or interactive media controls
    if (target.closest('button, input, select, audio, .media-viewer-controls')) return;

    pointers.set(event.pointerId, {
      x: event.clientX,
      y: event.clientY,
      startX: event.clientX,
      startY: event.clientY,
      startedAt: performance.now()
    });
    stage?.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent) {
    const previous = pointers.get(event.pointerId);
    if (!previous) return;

    const before = [...pointers.values()];
    pointers.set(event.pointerId, { ...previous, x: event.clientX, y: event.clientY });
    const after = [...pointers.values()];

    // Multi-touch pinch-to-zoom
    if (after.length >= 2 && current?.kind === 'image') {
      const [oldA, oldB] = before;
      const [newA, newB] = after;
      const oldDistance = Math.hypot(oldB.x - oldA.x, oldB.y - oldA.y);
      const newDistance = Math.hypot(newB.x - newA.x, newB.y - newA.y);
      const oldMidX = (oldA.x + oldB.x) / 2;
      const oldMidY = (oldA.y + oldB.y) / 2;
      const newMidX = (newA.x + newB.x) / 2;
      const newMidY = (newA.y + newB.y) / 2;
      translateX += newMidX - oldMidX;
      translateY += newMidY - oldMidY;
      if (oldDistance > 0) setScale(scale * (newDistance / oldDistance), newMidX, newMidY);
      event.preventDefault();
      return;
    }

    const deltaX = event.clientX - previous.x;
    const deltaY = event.clientY - previous.y;

    if (scale > MIN_SCALE + 0.05 && current?.kind === 'image') {
      clampTranslation(translateX + deltaX, translateY + deltaY);
      event.preventDefault();
    } else {
      const totalDeltaX = event.clientX - previous.startX;
      const totalDeltaY = event.clientY - previous.startY;

      // Detect vertical swipe gesture to dismiss (up or down with single pointer)
      if (pointers.size === 1 && Math.abs(totalDeltaY) > 8 && Math.abs(totalDeltaY) > Math.abs(totalDeltaX) * 1.1) {
        dismissOffsetY = totalDeltaY;
        swipeOffset = 0;
        event.preventDefault();
        return;
      }

      if (dismissOffsetY === 0 && items.length > 1) {
        // Allow horizontal swipe drag
        if (Math.abs(totalDeltaX) > 6 || Math.abs(swipeOffset) > 0) {
          swipeOffset = Math.max(-180, Math.min(180, totalDeltaX));
          event.preventDefault();
        }
      }
    }
  }

  function finishPointer(event: PointerEvent) {
    const point = pointers.get(event.pointerId);
    pointers.delete(event.pointerId);
    if (!point) return;

    if (dismissOffsetY !== 0) {
      const deltaY = event.clientY - point.startY;
      const elapsed = Math.max(1, performance.now() - point.startedAt);
      const velocityY = Math.abs(deltaY) / elapsed;

      if (Math.abs(dismissOffsetY) > 110 || (Math.abs(dismissOffsetY) > 30 && velocityY > 0.25)) {
        isDismissing = true;
        dismissOffsetY = dismissOffsetY > 0 ? 600 : -600;
        setTimeout(() => {
          close();
        }, 140);
        return;
      } else {
        dismissOffsetY = 0;
      }
    }

    if (scale <= MIN_SCALE + 0.05 && items.length > 1) {
      const deltaX = event.clientX - point.startX;
      const deltaY = event.clientY - point.startY;
      const elapsed = Math.max(1, performance.now() - point.startedAt);
      const velocity = Math.abs(deltaX) / elapsed;

      // Navigate if dragged > 40px or swift swipe gesture (>18px with velocity)
      if ((Math.abs(deltaX) > 40 || (Math.abs(deltaX) > 18 && velocity > 0.14)) && Math.abs(deltaX) > Math.abs(deltaY) * 0.7) {
        navigate(deltaX < 0 ? 1 : -1);
      }
    }

    swipeOffset = 0;
    if (pointers.size === 0) clampTranslation();
  }

  async function toggleFullscreen() {
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await root?.requestFullscreen();
    } catch {
    }
  }

  function close() {
    if (closing) return;
    closing = true;
    const finalTime = videoElement?.currentTime;
    if (videoElement) {
      videoElement.pause();
    }
    if (document.fullscreenElement === root) {
      void document.exitFullscreen().finally(() => onclose(index, finalTime));
    } else {
      onclose(index, finalTime);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (handleGlobalPanicKey(event)) return;

    if (event.key === 'Tab' && root) {
      const focusable = [...root.querySelectorAll<HTMLElement>('button:not([disabled]), video[controls], audio[controls], [tabindex]:not([tabindex="-1"])')];
      if (focusable.length > 0) {
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
      return;
    }

    const target = event.target;
    const nativeMediaControl = target instanceof HTMLVideoElement || target instanceof HTMLAudioElement;
    if (nativeMediaControl && !['Escape', 'f', 'F'].includes(event.key)) return;

    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      navigate(-1);
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      navigate(1);
    } else if (event.key === '+' || event.key === '=') {
      event.preventDefault();
      setScale(scale * 1.25);
    } else if (event.key === '-') {
      event.preventDefault();
      setScale(scale / 1.25);
    } else if (event.key === '0') {
      event.preventDefault();
      resetTransform();
    } else if (event.key.toLowerCase() === 'f') {
      event.preventDefault();
      void toggleFullscreen();
    }
    registerActivity();
  }

  function handleViewerClick(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (target.closest('button, video, audio, .media-viewer-media')) return;
    controlsVisible = !controlsVisible;
    if (controlsVisible) registerActivity();
  }

  $effect(() => {
    currentId;
    resetTransform();
    loadedWidth = 0;
    loadedHeight = 0;
    registerActivity();

    for (const neighbor of [items[index - 1], items[index + 1]]) {
      if (neighbor?.kind === 'image') {
        const preload = new Image();
        preload.decoding = 'async';
        preload.src = neighbor.url;
      }
    }
  });

  onMount(() => {
    index = Math.max(0, Math.min(initialIndex, items.length - 1));
    const previousOverflow = document.documentElement.style.overflow;
    const previousFocus = document.activeElement as HTMLElement | null;
    const backgroundElements = [...document.body.children]
      .filter((element): element is HTMLElement => element instanceof HTMLElement && element !== root)
      .map((element) => ({ element, inert: element.inert }));
    for (const { element } of backgroundElements) element.inert = true;
    document.documentElement.style.overflow = 'hidden';
    const handleFullscreenChange = () => fullscreen = document.fullscreenElement === root;
    document.addEventListener('fullscreenchange', handleFullscreenChange);
    void tick().then(() => root?.focus());
    registerActivity();

    return () => {
      document.documentElement.style.overflow = previousOverflow;
      document.removeEventListener('fullscreenchange', handleFullscreenChange);
      if (controlsTimer) clearTimeout(controlsTimer);
      for (const { element, inert } of backgroundElements) element.inert = inert;
      previousFocus?.focus();
    };
  });
</script>

<div
  bind:this={root}
  use:portal
  class="media-viewer"
  class:controls-hidden={!controlsVisible}
  class:is-dismissing={isDismissing}
  style:opacity={dismissOffsetY !== 0 ? dismissOpacity : undefined}
  role="dialog"
  aria-modal="true"
  aria-label={i18n.t('post.viewer_title')}
  tabindex="-1"
  onmousemove={registerActivity}
  onclick={handleViewerClick}
  onkeydown={handleKeydown}
>
  <header class="media-viewer-topbar media-viewer-controls">
    <div class="media-viewer-identity">
      <strong title={current?.name}>{current?.name || i18n.t('post.file')}</strong>
      <span>{currentResolution ? `${currentResolution} · ` : ''}{index + 1} / {items.length}{current?.size ? ` · ${formatBytes(current.size)}` : ''}</span>
    </div>

    <div class="media-viewer-actions">
      {#if current?.kind === 'image'}
        <Button variant="ghost" class="viewer-icon-btn desktop-zoom-control" onclick={() => setScale(scale / 1.25)} disabled={scale <= MIN_SCALE} title={i18n.t('post.viewer_zoom_out')} aria-label={i18n.t('post.viewer_zoom_out')}><IconZoomOut /></Button>
        <button class="zoom-value desktop-zoom-control" type="button" onclick={resetTransform} use:tooltip={i18n.t('post.viewer_reset')} aria-label={i18n.t('post.viewer_reset')}>{Math.round(scale * 100)}%</button>
        <Button variant="ghost" class="viewer-icon-btn desktop-zoom-control" onclick={() => setScale(scale * 1.25)} disabled={scale >= MAX_SCALE} title={i18n.t('post.viewer_zoom_in')} aria-label={i18n.t('post.viewer_zoom_in')}><IconZoomIn /></Button>
      {/if}
      {#if ondownload && current}
        <Button
          variant={currentDownloaded ? 'accent' : 'ghost'}
          class={`viewer-icon-btn viewer-download-icon${currentDownloaded ? ' is-downloaded' : ''}${currentDownloadActive ? ' is-downloading' : ''}`}
          onclick={requestDownload}
          disabled={currentDownloadActive}
          title={i18n.t(currentDownloaded ? 'post.downloaded' : currentDownloadActive ? 'post.downloading' : 'post.download')}
          aria-label={i18n.t(currentDownloaded ? 'post.downloaded' : currentDownloadActive ? 'post.downloading' : 'post.download')}
        >
          {#if currentDownloaded}<IconCheck />{:else if currentDownloadActive}<IconLoading />{:else}<IconDownload />{/if}
        </Button>
      {/if}
      <Button variant="ghost" class="viewer-icon-btn viewer-fullscreen-btn" onclick={toggleFullscreen} title={i18n.t(fullscreen ? 'post.viewer_exit_fullscreen' : 'post.viewer_fullscreen')} aria-label={i18n.t(fullscreen ? 'post.viewer_exit_fullscreen' : 'post.viewer_fullscreen')}>
        {#if fullscreen}<IconFullscreenExit />{:else}<IconFullscreen />{/if}
      </Button>
      <Button variant="ghost" class="viewer-icon-btn viewer-close-btn" onclick={close} title={i18n.t('post.viewer_close')} aria-label={i18n.t('post.viewer_close')}><IconDismiss /></Button>
    </div>
  </header>

  <div
    bind:this={stage}
    class="media-viewer-stage"
    class:image-interactive={current?.kind === 'image'}
    class:is-swiping={swipeOffset !== 0 || dismissOffsetY !== 0}
    class:can-swipe={items.length > 1 && scale <= MIN_SCALE}
    role="group"
    aria-label={current?.name || i18n.t('post.file')}
    onwheel={handleWheel}
    ondblclick={handleDoubleClick}
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={finishPointer}
    onpointercancel={finishPointer}
  >
    {#if current}
      {#key current.id}
        <div
          bind:this={fitFrame}
          class="media-fit-frame"
          class:is-swiping={swipeOffset !== 0 || dismissOffsetY !== 0}
          class:file-frame={!['image', 'video'].includes(current.kind)}
        >
        {#if current.kind === 'image'}
          <img
            bind:this={mediaElement}
            class="media-viewer-media image-media"
            class:zoomed={scale > MIN_SCALE}
            class:is-swiping={swipeOffset !== 0 || dismissOffsetY !== 0}
            src={current.url}
            alt={current.name}
            draggable="false"
            style:transform
            onload={handleImageLoad}
          />
        {:else if current.html}
          <div
            class="viewer-embed-state"
            class:is-swiping={swipeOffset !== 0 || dismissOffsetY !== 0}
            style:transform={swipeOffset !== 0 || dismissOffsetY !== 0 ? `translate3d(${swipeOffset}px, ${dismissOffsetY}px, 0) scale(${dismissScale})` : undefined}
          >
            <div class="viewer-embed-iframe-wrapper">
              {@html current.html}
            </div>
          </div>
        {:else if current.kind === 'video'}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video
            bind:this={videoElement}
            class="media-viewer-media"
            class:is-swiping={swipeOffset !== 0 || dismissOffsetY !== 0}
            style:transform={swipeOffset !== 0 || dismissOffsetY !== 0 ? `translate3d(${swipeOffset}px, ${dismissOffsetY}px, 0) scale(${dismissScale})` : undefined}
            src={current.url}
            poster={current.poster}
            controls
            autoplay
            playsinline
            use:panicCapture
            onkeydown={handleGlobalPanicKey}
            onloadedmetadata={handleVideoMetadata}
            ontimeupdate={handleVideoTimeUpdate}
            onended={handleVideoEnded}
          ></video>
        {:else if current.kind === 'audio'}
          <div
            class="viewer-file-state"
            class:is-swiping={swipeOffset !== 0 || dismissOffsetY !== 0}
            style:transform={swipeOffset !== 0 || dismissOffsetY !== 0 ? `translate3d(${swipeOffset}px, ${dismissOffsetY}px, 0) scale(${dismissScale})` : undefined}
          >
            <IconMusic />
            <strong>{current.name}</strong>
            {#if current.size}<span>{formatBytes(current.size)}</span>{/if}
            <audio src={current.url} controls autoplay></audio>
          </div>
        {:else}
          <div
            class="viewer-file-state"
            class:is-swiping={swipeOffset !== 0 || dismissOffsetY !== 0}
            style:transform={swipeOffset !== 0 || dismissOffsetY !== 0 ? `translate3d(${swipeOffset}px, ${dismissOffsetY}px, 0) scale(${dismissScale})` : undefined}
          >
            <IconDocument />
            <strong>{current.name}</strong>
            {#if current.size}<span>{formatBytes(current.size)}</span>{/if}
            {#if ondownload}
              <Button
                variant="ghost"
                class={`viewer-download-pill${currentDownloaded ? ' is-downloaded' : ''}${currentDownloadActive ? ' is-downloading' : ''}`}
                onclick={requestDownload}
                disabled={currentDownloadActive}
              >
                {#if currentDownloaded}
                  <IconCheck /><span>{i18n.t('post.downloaded')}{currentDownloadBytes ? ` · ${formatBytes(currentDownloadBytes)}` : ''}</span>
                {:else if currentDownloadActive}
                  <IconLoading /><span>{i18n.t('post.downloading')}{currentDownloadProgress ? ` · ${currentDownloadProgress}%` : ''}</span>
                {:else}
                  <IconDownload /><span>{i18n.t('post.download')}{currentDownloadBytes ? ` · ${formatBytes(currentDownloadBytes)}` : ''}</span>
                {/if}
              </Button>
            {/if}
          </div>
        {/if}
        </div>
      {/key}
    {/if}
  </div>

  {#if items.length > 1}
    <Button variant="ghost" class="viewer-nav viewer-prev media-viewer-controls" onclick={() => navigate(-1)} title={i18n.t('post.previous')} aria-label={i18n.t('post.previous')}><IconChevronLeft /></Button>
    <Button variant="ghost" class="viewer-nav viewer-next media-viewer-controls" onclick={() => navigate(1)} title={i18n.t('post.next')} aria-label={i18n.t('post.next')}><IconChevronRight /></Button>
  {/if}

  <footer class="media-viewer-bottom media-viewer-controls">
    {#if current?.kind === 'image'}
      <div class="mobile-zoom-controls">
        <Button variant="ghost" class="viewer-icon-btn" onclick={() => setScale(scale / 1.25)} disabled={scale <= MIN_SCALE} aria-label={i18n.t('post.viewer_zoom_out')}><IconZoomOut /></Button>
        <Button variant="ghost" class="viewer-reset-btn" onclick={resetTransform}><IconArrowReset /><span>{Math.round(scale * 100)}%</span></Button>
        <Button variant="ghost" class="viewer-icon-btn" onclick={() => setScale(scale * 1.25)} disabled={scale >= MAX_SCALE} aria-label={i18n.t('post.viewer_zoom_in')}><IconZoomIn /></Button>
      </div>
    {/if}

    {#if items.length > 1 && current?.kind !== 'image'}
      <div class="mobile-paging-controls">
        <Button variant="ghost" class="viewer-icon-btn" onclick={() => navigate(-1)} aria-label={i18n.t('post.previous')}><IconChevronLeft /></Button>
        <span>{index + 1} / {items.length}</span>
        <Button variant="ghost" class="viewer-icon-btn" onclick={() => navigate(1)} aria-label={i18n.t('post.next')}><IconChevronRight /></Button>
      </div>
    {/if}

    {#if items.length > 1}
      <div class="viewer-filmstrip" aria-label={i18n.t('post.viewer_thumbnails')}>
        {#each visibleThumbnails as thumbnail (thumbnail.item.id)}
          <button class:active={thumbnail.index === index} type="button" onclick={() => select(thumbnail.index)} aria-label={`${thumbnail.index + 1}: ${thumbnail.item.name}`}>
            {#if thumbnail.item.kind === 'image'}
              <img src={thumbnail.item.url} alt="" loading="lazy" />
            {:else if thumbnail.item.kind === 'video'}
              <IconVideo />
            {:else if thumbnail.item.kind === 'audio'}
              <IconMusic />
            {:else}
              <IconDocument />
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </footer>
</div>

<style>
  .media-viewer {
    position: fixed;
    inset: 0;
    z-index: 2147483000;
    overflow: hidden;
    background:
      radial-gradient(circle at 50% 42%, rgba(255, 255, 255, 0.035), transparent 45%),
      rgba(3, 3, 4, 0.985);
    color: var(--text-primary, #fff);
    outline: none;
    animation: viewer-enter 180ms var(--ease-expo, ease-out);
    overscroll-behavior: none;
  }

  .media-viewer.is-dismissing {
    transition: opacity 140ms ease-out;
    pointer-events: none;
  }

  .media-viewer.is-dismissing .media-fit-frame {
    transition: transform 140ms ease-out;
  }

  @keyframes viewer-enter {
    from { opacity: 0; transform: scale(0.992); }
    to { opacity: 1; transform: scale(1); }
  }

  .media-viewer-topbar,
  .media-viewer-bottom {
    position: absolute;
    left: 0;
    right: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    transition: opacity 180ms ease, transform 180ms ease;
  }

  .media-viewer-topbar {
    top: 0;
    min-height: 76px;
    justify-content: space-between;
    gap: 16px;
    padding: max(14px, env(safe-area-inset-top)) max(18px, env(safe-area-inset-right)) 14px max(18px, env(safe-area-inset-left));
    background: linear-gradient(to bottom, rgba(0, 0, 0, 0.78), transparent);
  }

  .media-viewer-bottom {
    bottom: 0;
    min-height: 88px;
    justify-content: center;
    flex-direction: column;
    gap: 10px;
    padding: 12px max(18px, env(safe-area-inset-right)) max(14px, env(safe-area-inset-bottom)) max(18px, env(safe-area-inset-left));
    background: linear-gradient(to top, rgba(0, 0, 0, 0.78), transparent);
  }

  .media-viewer-identity {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .media-viewer-identity strong {
    overflow: hidden;
    color: #fff;
    font-size: 14px;
    font-weight: 650;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }

  .media-viewer-identity span {
    color: rgba(255, 255, 255, 0.55);
    font-size: 12px;
  }

  .media-viewer-actions,
  .mobile-zoom-controls {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  :global(.media-viewer .viewer-icon-btn),
  :global(.media-viewer .viewer-reset-btn) {
    height: 44px !important;
    min-height: 44px !important;
    border: 0 !important;
    background: rgba(24, 24, 28, 0.82) !important;
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.24) !important;
  }

  :global(.media-viewer .viewer-icon-btn) {
    width: 44px !important;
    min-width: 44px !important;
    padding: 0 !important;
    border-radius: 50% !important;
  }

  :global(.media-viewer .viewer-icon-btn:hover),
  :global(.media-viewer .viewer-reset-btn:hover) {
    background: rgba(48, 48, 54, 0.94) !important;
  }

  :global(.media-viewer .viewer-icon-btn svg),
  :global(.media-viewer .viewer-reset-btn svg) {
    width: 20px !important;
    height: 20px !important;
  }

  .zoom-value {
    height: 36px;
    min-width: 58px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-full);
    background: rgba(24, 24, 28, 0.82);
    color: rgba(255, 255, 255, 0.8);
    font-family: var(--font-sans);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .media-viewer-stage {
    position: absolute;
    inset: 0;
    overflow: hidden;
    touch-action: none;
    user-select: none;
  }

  .media-viewer-stage.can-swipe {
    cursor: grab;
  }

  .media-viewer-stage.can-swipe:active,
  .media-viewer-stage.is-swiping {
    cursor: grabbing;
  }

  .media-fit-frame {
    position: absolute;
    inset: 84px clamp(18px, 7vw, 104px) 100px;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
    min-height: 0;
  }

  .media-viewer-stage.image-interactive {
    touch-action: none;
  }

  .media-viewer-media {
    display: block;
    min-width: 0;
    min-height: 0;
    max-width: 100%;
    max-height: 100%;
    width: auto;
    height: auto;
    object-fit: contain;
    user-select: none;
    -webkit-user-drag: none;
    transition: transform 150ms var(--ease-expo, ease-out);
  }

  .image-media {
    will-change: transform;
    transition: transform 150ms var(--ease-expo, ease-out);
  }

  .image-media.zoomed {
    cursor: grab;
    transition: none;
  }

  .image-media.zoomed:active {
    cursor: grabbing;
  }

  .image-media.is-swiping,
  .media-viewer-media.is-swiping,
  .viewer-file-state.is-swiping {
    transition: none !important;
  }

  .media-viewer-media:is(video) {
    width: min(100%, 1600px);
    max-height: 100%;
    background: #000;
  }

  :global(.viewer-nav) {
    position: absolute !important;
    top: 50%;
    z-index: 20;
    width: 52px !important;
    height: 52px !important;
    min-width: 52px !important;
    padding: 0 !important;
    border: 0 !important;
    border-radius: 50% !important;
    background: rgba(24, 24, 28, 0.78) !important;
    transform: translateY(-50%);
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
    transition: opacity 180ms ease, transform 180ms ease, background 150ms ease !important;
  }

  :global(.viewer-nav:hover) {
    background: rgba(48, 48, 54, 0.94) !important;
    transform: translateY(-50%) scale(1.06) !important;
  }

  :global(.viewer-nav:active) {
    transform: translateY(-50%) scale(0.96) !important;
  }

  :global(.viewer-nav svg) {
    width: 26px;
    height: 26px;
    transition: transform 150ms var(--ease-expo, ease-out);
  }

  :global(.viewer-prev:hover svg) { transform: translateX(-2px); }
  :global(.viewer-next:hover svg) { transform: translateX(2px); }

  :global(.viewer-prev) { left: max(18px, env(safe-area-inset-left)); }
  :global(.viewer-next) { right: max(18px, env(safe-area-inset-right)); }

  .viewer-filmstrip {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    max-width: min(760px, 80vw);
    box-sizing: border-box;
    padding: 4px 4px 0;
    overflow-x: hidden;
    overflow-y: hidden;
  }

  .viewer-filmstrip button {
    width: 48px;
    height: 48px;
    flex: 0 0 48px;
    display: grid;
    place-items: center;
    overflow: hidden;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.6);
    cursor: pointer;
    opacity: 0.62;
    transition: opacity 140ms ease, border-color 140ms ease, transform 140ms ease;
  }

  .viewer-filmstrip button:hover,
  .viewer-filmstrip button.active {
    opacity: 1;
    transform: translateY(-2px);
  }

  .viewer-filmstrip button.active {
    border-color: transparent;
    box-shadow: inset 0 0 0 2px var(--accent-primary);
  }

  .viewer-filmstrip img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .viewer-embed-state {
    width: 100%;
    height: 100%;
    max-width: min(1100px, 92vw);
    max-height: min(720px, 85vh);
    display: flex;
    align-items: center;
    justify-content: center;
    margin: auto;
    z-index: 10;
  }

  .viewer-embed-iframe-wrapper {
    width: 100%;
    aspect-ratio: 16 / 9;
    background: #000;
    border-radius: var(--radius-lg, 12px);
    overflow: hidden;
    box-shadow: var(--shadow-xl);
  }

  .viewer-embed-iframe-wrapper :global(iframe) {
    width: 100% !important;
    height: 100% !important;
    border: 0;
    display: block;
  }

  .viewer-file-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    max-width: min(520px, 90vw);
    padding: 32px;
    text-align: center;
  }

  .viewer-file-state > :global(svg) {
    width: 72px;
    height: 72px;
    color: rgba(255, 255, 255, 0.28);
  }

  .viewer-file-state strong {
    max-width: 100%;
    overflow-wrap: anywhere;
    font-size: 16px;
    user-select: text;
  }

  .viewer-file-state span { color: rgba(255, 255, 255, 0.55); font-size: 13px; }
  .viewer-file-state audio { width: min(420px, 80vw); }
  .mobile-zoom-controls { display: none; }
  .mobile-paging-controls { display: none; }

  :global(.media-viewer .viewer-download-icon.is-downloaded) {
    background: color-mix(in srgb, var(--accent-primary) 72%, #000 28%) !important;
    color: #fff !important;
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.18),
      0 8px 24px rgba(0, 0, 0, 0.28) !important;
  }

  :global(.media-viewer .viewer-download-icon.is-downloaded svg) {
    color: #fff !important;
    stroke-width: 2.25;
  }

  :global(.media-viewer .viewer-download-icon.is-downloading:disabled),
  :global(.media-viewer .viewer-download-pill.is-downloading:disabled) {
    opacity: 1 !important;
  }

  :global(.media-viewer .viewer-download-pill) {
    position: relative;
    width: 220px !important;
    max-width: 100%;
    height: 44px !important;
    border-radius: var(--radius-full) !important;
  }

  :global(.media-viewer .viewer-download-pill.is-downloaded) {
    color: var(--accent-primary) !important;
  }

  .controls-hidden .media-viewer-topbar {
    opacity: 0;
    pointer-events: none;
    transform: translateY(-12px);
  }

  .controls-hidden .media-viewer-bottom {
    opacity: 0;
    pointer-events: none;
    transform: translateY(12px);
  }

  .controls-hidden :global(.viewer-prev) {
    opacity: 0;
    pointer-events: none;
    transform: translate(-10px, -50%);
  }

  .controls-hidden :global(.viewer-next) {
    opacity: 0;
    pointer-events: none;
    transform: translate(10px, -50%);
  }

  @media (max-width: 700px), (pointer: coarse) {
    .media-viewer-topbar {
      min-height: 64px;
      padding-top: max(10px, env(safe-area-inset-top));
      padding-right: max(10px, env(safe-area-inset-right));
      padding-left: max(14px, env(safe-area-inset-left));
    }

    .media-viewer-stage {
      padding: 0;
    }

    .media-fit-frame { inset: 68px 0 92px; }

    .media-viewer-actions { gap: 4px; }
    .desktop-zoom-control { display: none !important; }
    :global(.viewer-nav) { display: none !important; }
    .mobile-zoom-controls { display: flex; }
    .mobile-paging-controls {
      display: flex;
      align-items: center;
      gap: 10px;
    }
    .mobile-paging-controls span {
      min-width: 54px;
      color: rgba(255, 255, 255, 0.7);
      font-size: 12px;
      font-weight: 600;
      text-align: center;
    }

    :global(.media-viewer .viewer-reset-btn) {
      min-width: 82px !important;
      padding: 0 14px !important;
      border-radius: var(--radius-full) !important;
    }

    .media-viewer-bottom {
      min-height: 76px;
      padding-top: 8px;
      gap: 8px;
    }

    .viewer-filmstrip { display: none; }
    .media-viewer-identity strong { font-size: 13px; }
    .media-viewer-identity { max-width: calc(100vw - 160px); }
  }

  @media (max-width: 380px) {
    .media-viewer-identity { max-width: calc(100vw - 112px); }
    .media-viewer-actions :global(.viewer-fullscreen-btn) { display: none !important; }
  }

  @media (prefers-reduced-motion: reduce) {
    .media-viewer,
    .media-viewer-controls,
    .image-media,
    :global(.viewer-nav),
    .viewer-filmstrip button {
      animation: none !important;
      transition: none !important;
    }
  }
</style>
