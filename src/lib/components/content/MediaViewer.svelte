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
    duration?: number;
    html?: string;
    downloadStatus?: 'queued' | 'resolving' | 'downloading' | 'paused' | 'verifying' | 'completed' | 'failed' | 'cancelled' | 'missing';
    downloadedBytes?: number;
    totalBytes?: number;
    downloadedPath?: string;
    isUnavailable?: boolean;
    isUnarchived?: boolean;
  }
</script>

<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { portal } from '$lib/actions/portal';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import { tooltip } from '$lib/motion';
  import { playbackState } from '$lib/state/playbackState.svelte';
  import { handleGlobalPanicKey, panicCapture } from '$lib/utils/panic';
  import { logMediaError } from '$lib/utils/logger';
  import { diagnoseVideoFailure, diagnoseVideoFailureAsync, getUnsupportedContainerFormat, getFileExtension, type MediaFailureState } from '$lib/utils/media';
  import { apiOpenDownloadFile } from '$lib/utils/ipc';
  import { getVideoThumbnail } from '$lib/utils/videoThumbnail';
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
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconVideoOff from '~icons/fluent/video-off-24-regular';
  import IconPlay from '~icons/fluent/play-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';

  interface Props {
    items: MediaViewerItem[];
    initialIndex?: number;
    initialTime?: number;
    onclose: (finalIndex?: number, finalTime?: number) => void;
    ondownload?: (item: MediaViewerItem, index: number) => void | Promise<void>;
    onopenpost?: (item: MediaViewerItem, index: number) => void;
    ondelete?: (item: MediaViewerItem, index: number) => void | Promise<void>;
  }

  let { items, initialIndex = 0, initialTime = 0, onclose, ondownload, onopenpost, ondelete }: Props = $props();

  const MIN_SCALE = 1;
  const MAX_SCALE = 8;
  let index = $state(0);
  let lastPropIndex = $state<number | null>(null);

  $effect(() => {
    if (initialIndex !== lastPropIndex) {
      lastPropIndex = initialIndex;
      index = Math.max(0, Math.min(items.length - 1, initialIndex));
    }
  });

  $effect(() => {
    if (items.length === 0) {
      close();
    } else if (index >= items.length) {
      index = items.length - 1;
    }
  });

  async function handleDelete() {
    if (!current || !ondelete) return;
    const itemToDelete = current;
    const idxToDelete = index;
    await ondelete(itemToDelete, idxToDelete);
  }

  let scale = $state(1);
  let translateX = $state(0);
  let translateY = $state(0);
  let swipeOffset = $state(0);
  let swipeOpacity = $state(1);
  let dismissOffsetY = $state(0);
  let isDismissing = $state(false);
  let isSwiping = $state(false);
  let slidePhase = $state<'idle' | 'out' | 'in'>('idle');
  let slideTimer: ReturnType<typeof setTimeout> | undefined;
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
  let videoErrors = $state<Record<number, MediaFailureState>>({});

  function handleVideoMetadata(e: Event) {
    const video = e.currentTarget as HTMLVideoElement;
    if (video.videoWidth > 0 && video.videoHeight > 0) {
      loadedWidth = video.videoWidth;
      loadedHeight = video.videoHeight;
      aspectRatios[index] = video.videoWidth / video.videoHeight;
      if (!current?.poster && !videoThumbnails[index]) {
        captureVideoFrame(video, index);
      }
    }
    if (video.duration && isFinite(video.duration) && video.duration > 0) {
      activeVideoDuration = video.duration;
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
    if (slideTimer) clearTimeout(slideTimer);
    if (scrollRafId) cancelAnimationFrame(scrollRafId);
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

  let filmstripContainer = $state<HTMLDivElement>();
  let thumbnailRefs = $state<Record<number, HTMLButtonElement>>({});
  let aspectRatios = $state<Record<number, number>>({});
  let posterFailed = $state<Record<number, boolean>>({});
  let videoThumbnails = $state<Record<number, string>>({});
  let activeVideoDuration = $state(0);

  function formatVideoDuration(seconds?: number): string {
    if (!seconds || isNaN(seconds) || !isFinite(seconds) || seconds <= 0) return '';
    const total = Math.floor(seconds);
    const hrs = Math.floor(total / 3600);
    const mins = Math.floor((total % 3600) / 60);
    const secs = total % 60;
    if (hrs > 0) {
      return `${hrs}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    }
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  function getVideoDuration(item: MediaViewerItem, itemIndex: number): number | undefined {
    if (item.duration && item.duration > 0) return item.duration;
    if (itemIndex === index && activeVideoDuration > 0) return activeVideoDuration;
    const key = item.id || item.name || item.url;
    return playbackState.getDuration(key);
  }

  function requestVideoThumbnail(itemIndex: number) {
    const it = items[itemIndex];
    if (!it || it.kind !== 'video' || videoThumbnails[itemIndex]) return;
    const key = it.id || it.name || it.url;
    const videoUrl = it.url;
    if (!videoUrl) return;
    getVideoThumbnail(key, videoUrl).then((thumb) => {
      if (thumb) {
        videoThumbnails = { ...videoThumbnails, [itemIndex]: thumb };
      }
    });
  }

  $effect(() => {
    for (let i = 0; i < items.length; i++) {
      const it = items[i];
      if (it.kind === 'video') {
        if (!it.poster || posterFailed[i]) {
          requestVideoThumbnail(i);
        }
      }
    }
  });

  function captureVideoFrame(video: HTMLVideoElement, itemIndex: number) {
    try {
      if (!video.videoWidth || !video.videoHeight) return;
      const canvas = document.createElement('canvas');
      const w = 240;
      const h = Math.max(80, Math.round(w * (video.videoHeight / video.videoWidth)));
      canvas.width = w;
      canvas.height = h;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.drawImage(video, 0, 0, w, h);
      const dataUrl = canvas.toDataURL('image/jpeg', 0.8);
      if (dataUrl && dataUrl.length > 100) {
        videoThumbnails = { ...videoThumbnails, [itemIndex]: dataUrl };
        const it = items[itemIndex];
        const key = it?.id || it?.name || it?.url;
        if (key) {
          invoke('store_video_thumbnail', { key, dataUrl }).catch(() => {});
        }
      }
    } catch {
      // ignore
    }
  }

  function handleMainVideoLoaded(event: Event) {
    const vid = event.currentTarget as HTMLVideoElement;
    if (vid.videoWidth && vid.videoHeight) {
      aspectRatios[index] = vid.videoWidth / vid.videoHeight;
      if (!current?.poster && !videoThumbnails[index]) {
        captureVideoFrame(vid, index);
      }
    }
  }
  let isFilmstripPointerDown = false;
  let isFilmstripDragging = false;
  let filmstripStartX = 0;
  let filmstripStartScrollLeft = 0;
  let filmstripActivePointerId: number | null = null;
  let scrollRafId: number | null = null;
  let targetScrollLeft = 0;

  $effect(() => {
    const activeBtn = thumbnailRefs[index];
    if (activeBtn && filmstripContainer && !isFilmstripDragging) {
      activeBtn.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest',
        inline: 'center'
      });
    }
  });

  function handleThumbImageLoad(event: Event, itemIndex: number) {
    const img = event.currentTarget as HTMLImageElement;
    if (img.naturalWidth && img.naturalHeight) {
      aspectRatios[itemIndex] = img.naturalWidth / img.naturalHeight;
    }
  }

  function getThumbAspectRatio(item: MediaViewerItem, itemIndex: number): string {
    const ratio = (item.width && item.height)
      ? (item.width / item.height)
      : (aspectRatios[itemIndex] ?? 1);
    // Clamp between 0.4 (tall portrait) and 2.4 (panoramic landscape)
    const clamped = Math.max(0.4, Math.min(2.4, ratio));
    return `${clamped} / 1`;
  }

  function handleFilmstripWheel(event: WheelEvent) {
    if (!filmstripContainer) return;
    const delta = Math.abs(event.deltaY) > Math.abs(event.deltaX) ? event.deltaY : event.deltaX;
    if (delta !== 0) {
      event.preventDefault();
      event.stopPropagation();
      filmstripContainer.scrollLeft += delta;
    }
  }

  function onFilmstripPointerDown(e: PointerEvent) {
    if (!filmstripContainer || e.button !== 0) return;
    isFilmstripPointerDown = true;
    isFilmstripDragging = false;
    filmstripStartX = e.clientX;
    filmstripStartScrollLeft = filmstripContainer.scrollLeft;
    targetScrollLeft = filmstripStartScrollLeft;
    filmstripActivePointerId = e.pointerId;
  }

  function onFilmstripPointerMove(e: PointerEvent) {
    if (!isFilmstripPointerDown || !filmstripContainer) return;
    const deltaX = e.clientX - filmstripStartX;

    if (!isFilmstripDragging) {
      if (Math.abs(deltaX) > 6) {
        isFilmstripDragging = true;
        if (filmstripActivePointerId !== null) {
          try {
            filmstripContainer.setPointerCapture(filmstripActivePointerId);
          } catch {
          }
        }
      } else {
        return;
      }
    }

    targetScrollLeft = filmstripStartScrollLeft - deltaX;
    if (!scrollRafId) {
      scrollRafId = requestAnimationFrame(() => {
        if (filmstripContainer) {
          filmstripContainer.scrollLeft = targetScrollLeft;
        }
        scrollRafId = null;
      });
    }
  }

  function onFilmstripPointerUp(e: PointerEvent) {
    if (!isFilmstripPointerDown) return;
    isFilmstripPointerDown = false;

    if (isFilmstripDragging) {
      if (filmstripContainer && filmstripActivePointerId !== null) {
        try {
          filmstripContainer.releasePointerCapture(filmstripActivePointerId);
        } catch {
        }
      }
      if (scrollRafId) {
        cancelAnimationFrame(scrollRafId);
        scrollRafId = null;
      }
      if (filmstripContainer) {
        filmstripContainer.scrollLeft = targetScrollLeft;
      }
      setTimeout(() => {
        isFilmstripDragging = false;
      }, 50);
    } else {
      isFilmstripDragging = false;
    }

    filmstripActivePointerId = null;
  }

  function handleThumbnailClick(itemIndex: number) {
    if (isFilmstripDragging) return;
    select(itemIndex);
  }

  function resetTransform() {
    scale = MIN_SCALE;
    translateX = 0;
    translateY = 0;
    swipeOffset = 0;
    swipeOpacity = 1;
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

  function transitionSlide(direction: 1 | -1) {
    if (items.length < 2) return;

    if (slideTimer) {
      clearTimeout(slideTimer);
      slideTimer = undefined;
    }

    const stageWidth = stage?.clientWidth || window.innerWidth;
    const exitDist = Math.min(stageWidth * 0.45, 360);
    const enterDist = Math.min(stageWidth * 0.35, 280);

    slidePhase = 'out';
    isSwiping = false;
    swipeOffset = direction === 1 ? -exitDist : exitDist;
    swipeOpacity = 0;

    slideTimer = setTimeout(() => {
      index = (index + direction + items.length) % items.length;
      resetTransform();

      // Instantly position incoming item at opposite edge
      isSwiping = true;
      slidePhase = 'idle';
      swipeOffset = direction === 1 ? enterDist : -enterDist;
      swipeOpacity = 0;

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          isSwiping = false;
          slidePhase = 'in';
          swipeOffset = 0;
          swipeOpacity = 1;

          slideTimer = setTimeout(() => {
            slidePhase = 'idle';
            slideTimer = undefined;
          }, 180);
        });
      });
    }, 120);
  }

  function navigate(delta: number, animate = false) {
    if (items.length < 2) return;
    if (animate) {
      transitionSlide(delta > 0 ? 1 : -1);
    } else {
      if (slideTimer) {
        clearTimeout(slideTimer);
        slideTimer = undefined;
      }
      slidePhase = 'idle';
      isSwiping = false;
      swipeOffset = 0;
      swipeOpacity = 1;
      index = (index + delta + items.length) % items.length;
      resetTransform();
    }
  }

  function select(nextIndex: number) {
    if (nextIndex === index) return;
    if (slideTimer) {
      clearTimeout(slideTimer);
      slideTimer = undefined;
    }
    slidePhase = 'idle';
    isSwiping = false;
    swipeOffset = 0;
    swipeOpacity = 1;
    index = nextIndex;
    resetTransform();
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
        swipeOpacity = 1;
        event.preventDefault();
        return;
      }

      if (dismissOffsetY === 0 && items.length > 1) {
        // Allow horizontal swipe drag
        if (Math.abs(totalDeltaX) > 4 || Math.abs(swipeOffset) > 0) {
          isSwiping = true;
          slidePhase = 'idle';
          swipeOffset = totalDeltaX;
          swipeOpacity = Math.max(0.4, 1 - Math.abs(totalDeltaX) / 800);
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

    if (scale <= MIN_SCALE + 0.05 && items.length > 1 && swipeOffset !== 0) {
      const deltaX = event.clientX - point.startX;
      const deltaY = event.clientY - point.startY;
      const elapsed = Math.max(1, performance.now() - point.startedAt);
      const velocityX = Math.abs(deltaX) / elapsed;

      const isSwipe = (Math.abs(deltaX) > 36 || (Math.abs(deltaX) > 16 && velocityX > 0.18)) && Math.abs(deltaX) > Math.abs(deltaY) * 0.7;

      if (isSwipe) {
        transitionSlide(deltaX < 0 ? 1 : -1);
      } else {
        // Snap back to center
        isSwiping = false;
        swipeOffset = 0;
        swipeOpacity = 1;
      }
    } else {
      isSwiping = false;
      swipeOffset = 0;
      swipeOpacity = 1;
    }

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
      navigate(-1, false);
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      navigate(1, false);
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
      <strong use:tooltip={current?.name || i18n.t('post.file')}>{current?.name || i18n.t('post.file')}</strong>
      <span>{currentResolution ? `${currentResolution} · ` : ''}{index + 1} / {items.length}{current?.size ? ` · ${formatBytes(current.size)}` : ''}</span>
    </div>

    <div class="media-viewer-actions">
      {#if current?.kind === 'image'}
        <Button variant="ghost" class="viewer-icon-btn viewer-zoom-btn" onclick={() => setScale(scale / 1.25)} disabled={scale <= MIN_SCALE} title={i18n.t('post.viewer_zoom_out')} aria-label={i18n.t('post.viewer_zoom_out')}><IconZoomOut /></Button>
        <button class="zoom-value viewer-zoom-btn" type="button" onclick={resetTransform} use:tooltip={i18n.t('post.viewer_reset')} aria-label={i18n.t('post.viewer_reset')}>{Math.round(scale * 100)}%</button>
        <Button variant="ghost" class="viewer-icon-btn viewer-zoom-btn" onclick={() => setScale(scale * 1.25)} disabled={scale >= MAX_SCALE} title={i18n.t('post.viewer_zoom_in')} aria-label={i18n.t('post.viewer_zoom_in')}><IconZoomIn /></Button>
      {/if}
      {#if ondownload && current}
        <Button
          variant="ghost"
          class={`viewer-icon-btn viewer-download-icon${currentDownloaded ? ' is-downloaded' : ''}${currentDownloadActive ? ' is-downloading' : ''}`}
          onclick={requestDownload}
          disabled={currentDownloadActive}
          title={i18n.t(currentDownloaded ? 'post.downloaded' : currentDownloadActive ? 'post.downloading' : 'post.download')}
          aria-label={i18n.t(currentDownloaded ? 'post.downloaded' : currentDownloadActive ? 'post.downloading' : 'post.download')}
        >
          {#if currentDownloaded}<IconCheck />{:else if currentDownloadActive}<IconLoading />{:else}<IconDownload />{/if}
        </Button>
      {/if}
      {#if onopenpost && current}
        <Button
          variant="ghost"
          class="viewer-icon-btn"
          onclick={() => onopenpost(current, index)}
          tooltip={i18n.t('downloads.open_in_post')}
          aria-label={i18n.t('downloads.open_in_post')}
        >
          <IconOpen />
        </Button>
      {/if}
      {#if ondelete && current}
        <Button
          variant="ghost"
          class="viewer-icon-btn viewer-delete-btn"
          onclick={handleDelete}
          tooltip={i18n.t('downloads.remove')}
          aria-label={i18n.t('downloads.remove')}
        >
          <IconDelete />
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
      <div
        bind:this={fitFrame}
        class="media-fit-frame"
        class:is-swiping={isSwiping}
        class:is-sliding-out={slidePhase === 'out'}
        class:is-sliding-in={slidePhase === 'in'}
        class:file-frame={!['image', 'video'].includes(current.kind)}
        style:opacity={isDismissing ? dismissOpacity : swipeOpacity}
      >
        {#if current.kind === 'image'}
          <img
            bind:this={mediaElement}
            class="media-viewer-media image-media"
            class:zoomed={scale > MIN_SCALE}
            src={current.url || current.poster}
            alt={current.name}
            draggable="false"
            style:transform
            onload={handleImageLoad}
            onerror={(e) => {
              const target = e.currentTarget as HTMLImageElement;
              logMediaError('image', target.src, current.name);
              if (current?.poster && target.src !== current.poster) {
                target.src = current.poster;
              }
            }}
          />
        {:else if current.html}
          <div
            class="viewer-embed-state"
            style:transform={dismissOffsetY !== 0 ? `translate3d(0, ${dismissOffsetY}px, 0) scale(${dismissScale})` : undefined}
          >
            <div class="viewer-embed-iframe-wrapper">
              {@html current.html}
            </div>
          </div>
        {:else if current.kind === 'video' && (current.isUnarchived || current.isUnavailable || videoErrors[index] || getUnsupportedContainerFormat(current?.name, current?.url))}
          {@const failure = current.isUnarchived ? { preset: 'unarchived' as const } : current.isUnavailable ? { preset: 'unavailable' as const } : (videoErrors[index] || { preset: 'unsupported_format' as const, format: getUnsupportedContainerFormat(current?.name, current?.url) || undefined })}
          <div
            class="viewer-file-state"
            style:transform={dismissOffsetY !== 0 ? `translate3d(0, ${dismissOffsetY}px, 0) scale(${dismissScale})` : undefined}
          >
            <IconVideoOff class="w-12 h-12 text-white/50 mb-2" />
            <strong class="text-white text-base font-semibold">{current.name}</strong>
            {#if current.size}<span>{formatBytes(current.size)}</span>{/if}
            {#if failure.preset === 'unsupported_format'}
              <p class="text-white/80 text-sm max-w-md text-center mt-2">
                {i18n.t('post.unsupported_format_desc', { format: failure.format || getFileExtension(current.name) })}
              </p>
              {#if currentDownloaded && current?.downloadedPath}
                <Button variant="ghost" class="mt-3 viewer-ghost-action" onclick={() => void apiOpenDownloadFile(current.downloadedPath!)}>
                  <IconPlay class="w-4 h-4 mr-1.5" />
                  <span>{i18n.t('post.open_in_player')}</span>
                </Button>
              {:else}
                <p class="text-white/50 text-xs mt-1">{i18n.t('post.unsupported_format_hint')}</p>
              {/if}
            {:else if failure.preset === 'unsupported_codec'}
              <p class="text-white/80 text-sm max-w-md text-center mt-2">{i18n.t('post.unsupported_codec_desc')}</p>
              {#if currentDownloaded && current?.downloadedPath}
                <Button variant="ghost" class="mt-3 viewer-ghost-action" onclick={() => void apiOpenDownloadFile(current.downloadedPath!)}>
                  <IconPlay class="w-4 h-4 mr-1.5" />
                  <span>{i18n.t('post.open_in_player')}</span>
                </Button>
              {/if}
            {:else if failure.preset === 'network'}
              <p class="text-white/80 text-sm max-w-md text-center mt-2">{i18n.t('post.network_stream_error')}</p>
            {:else if failure.preset === 'decode'}
              <p class="text-white/80 text-sm max-w-md text-center mt-2">{i18n.t('post.decode_error')}</p>
              {#if failure.message}
                <p class="text-white/50 font-mono text-xs mt-1">{failure.message}</p>
              {/if}
              {#if currentDownloaded && current?.downloadedPath}
                <Button variant="ghost" class="mt-3 viewer-ghost-action" onclick={() => void apiOpenDownloadFile(current.downloadedPath!)}>
                  <IconPlay class="w-4 h-4 mr-1.5" />
                  <span>{i18n.t('post.open_in_player')}</span>
                </Button>
              {/if}
            {:else if failure.preset === 'forbidden' || failure.httpStatus === 403}
              <p class="text-amber-400 font-medium text-base text-center mt-2">{i18n.t('post.error_forbidden') || 'HTTP 403 Forbidden'}</p>
              <p class="text-white/70 text-xs max-w-md text-center mt-1">{i18n.t('post.error_forbidden_hint')}</p>
            {:else if failure.preset === 'not_found' || failure.httpStatus === 404}
              <p class="text-red-400 font-medium text-base text-center mt-2">{i18n.t('post.error_not_found') || 'HTTP 404 Not Found'}</p>
              <p class="text-white/70 text-xs max-w-md text-center mt-1">{i18n.t('post.error_not_found_hint')}</p>
            {:else if failure.preset === 'rate_limited' || failure.httpStatus === 429}
              <p class="text-amber-400 font-medium text-base text-center mt-2">{i18n.t('post.error_rate_limited') || 'HTTP 429 Rate Limited'}</p>
              <p class="text-white/70 text-xs max-w-md text-center mt-1">{i18n.t('post.error_rate_limited_hint')}</p>
            {:else if failure.preset === 'server_error' || (failure.httpStatus && failure.httpStatus >= 500)}
              <p class="text-red-400 font-medium text-base text-center mt-2">{failure.message || i18n.t('post.error_server')}</p>
              <p class="text-white/70 text-xs max-w-md text-center mt-1">{i18n.t('post.error_server_hint')}</p>
            {:else if failure.preset === 'unarchived'}
              <p class="text-white/80 text-sm max-w-md text-center mt-2">{i18n.t('post.file_not_archived')}</p>
            {:else if failure.preset === 'unavailable'}
              <p class="text-white/80 text-sm max-w-md text-center mt-2">{i18n.t('post.cloud_file_unavailable')}</p>
            {:else}
              <p class="text-white/80 text-sm max-w-md text-center mt-2">{failure.message || i18n.t('post.video_load_failed')}</p>
            {/if}
          </div>
        {:else if current.kind === 'video'}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video
            bind:this={videoElement}
            class="media-viewer-media"
            style:transform={dismissOffsetY !== 0 ? `translate3d(0, ${dismissOffsetY}px, 0) scale(${dismissScale})` : undefined}
            src={current.url}
            poster={current.poster || videoThumbnails[index]}
            controls
            autoplay
            playsinline
            preload="auto"
            use:panicCapture
            onkeydown={handleGlobalPanicKey}
            onplaying={handleMainVideoLoaded}
            onerror={async (e) => {
              const el = e.currentTarget as HTMLVideoElement;
              logMediaError('video', el.src, current.name, el.error);
              const syncDiag = diagnoseVideoFailure({ name: current.name, path: current.url } as any, el, {
                isLocal: currentDownloaded
              });
              videoErrors = { ...videoErrors, [index]: syncDiag };
              if (!currentDownloaded && (syncDiag.preset === 'unavailable' || syncDiag.preset === 'network')) {
                const asyncDiag = await diagnoseVideoFailureAsync({ name: current.name, path: current.url } as any, el, {
                  isLocal: currentDownloaded
                });
                if (asyncDiag && asyncDiag.preset !== syncDiag.preset) {
                  videoErrors = { ...videoErrors, [index]: asyncDiag };
                }
              }
            }}
            onloadedmetadata={handleVideoMetadata}
            ontimeupdate={handleVideoTimeUpdate}
            onended={handleVideoEnded}
          ></video>
        {:else if current.kind === 'audio'}
          <div
            class="viewer-file-state"
            style:transform={dismissOffsetY !== 0 ? `translate3d(0, ${dismissOffsetY}px, 0) scale(${dismissScale})` : undefined}
          >
            <IconMusic />
            <strong>{current.name}</strong>
            {#if current.size}<span>{formatBytes(current.size)}</span>{/if}
            <audio
              src={current.url}
              controls
              autoplay
              onerror={(e) => {
                const el = e.currentTarget as HTMLAudioElement;
                logMediaError('audio', el.src, current.name, el.error);
              }}
            ></audio>
          </div>
        {:else}
          <div
            class="viewer-file-state"
            style:transform={dismissOffsetY !== 0 ? `translate3d(0, ${dismissOffsetY}px, 0) scale(${dismissScale})` : undefined}
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
      {/if}
  </div>

  {#if items.length > 1}
    <Button variant="ghost" class="viewer-nav viewer-prev media-viewer-controls" onclick={() => navigate(-1, false)} title={i18n.t('post.previous')} aria-label={i18n.t('post.previous')}><IconChevronLeft /></Button>
    <Button variant="ghost" class="viewer-nav viewer-next media-viewer-controls" onclick={() => navigate(1, false)} title={i18n.t('post.next')} aria-label={i18n.t('post.next')}><IconChevronRight /></Button>
  {/if}

  {#if current?.kind === 'image' || items.length > 1}
    <footer class="media-viewer-bottom media-viewer-controls">
      {#if current?.kind === 'image'}
        <div class="mobile-zoom-controls">
          <Button variant="ghost" class="viewer-icon-btn" onclick={() => setScale(scale / 1.25)} disabled={scale <= MIN_SCALE} aria-label={i18n.t('post.viewer_zoom_out')}><IconZoomOut /></Button>
          <Button variant="ghost" class="viewer-reset-btn" onclick={resetTransform}><IconArrowReset /><span>{Math.round(scale * 100)}%</span></Button>
          <Button variant="ghost" class="viewer-icon-btn" onclick={() => setScale(scale * 1.25)} disabled={scale >= MAX_SCALE} aria-label={i18n.t('post.viewer_zoom_in')}><IconZoomIn /></Button>
        </div>
      {/if}

      {#if items.length > 1}
        <div class="viewer-filmstrip-wrapper">
          <div
            bind:this={filmstripContainer}
            class="viewer-filmstrip"
            role="region"
            aria-label={i18n.t('post.viewer_thumbnails')}
            onwheel={handleFilmstripWheel}
            onpointerdown={onFilmstripPointerDown}
            onpointermove={onFilmstripPointerMove}
            onpointerup={onFilmstripPointerUp}
            onpointercancel={onFilmstripPointerUp}
          >
            {#each items as item, itemIndex (item.id || itemIndex)}
              <button
                bind:this={thumbnailRefs[itemIndex]}
                class="filmstrip-item"
                class:active={itemIndex === index}
                type="button"
                onclick={() => handleThumbnailClick(itemIndex)}
                aria-label={`${itemIndex + 1}: ${item.name}`}
                use:tooltip={item.name || i18n.t('post.file')}
              >
                <div
                  class="filmstrip-thumb"
                  style:aspect-ratio={getThumbAspectRatio(item, itemIndex)}
                >
                  {#if item.kind === 'image'}
                    <img
                      src={item.poster || item.url}
                      alt=""
                      loading="lazy"
                      draggable="false"
                      onload={(e) => handleThumbImageLoad(e, itemIndex)}
                      onerror={(e) => {
                        const target = e.currentTarget as HTMLImageElement;
                        if (item.url && target.src !== item.url) {
                          target.src = item.url;
                        }
                      }}
                    />
                  {:else if (!posterFailed[itemIndex] && item.poster) || videoThumbnails[itemIndex]}
                    {@const thumbSrc = (!posterFailed[itemIndex] && item.poster) ? item.poster : videoThumbnails[itemIndex]}
                    {#if thumbSrc}
                      <img
                        src={thumbSrc}
                        alt=""
                        loading="lazy"
                        draggable="false"
                        onload={(e) => handleThumbImageLoad(e, itemIndex)}
                        onerror={() => {
                          if (!posterFailed[itemIndex] && item.poster) {
                            posterFailed = { ...posterFailed, [itemIndex]: true };
                            requestVideoThumbnail(itemIndex);
                          }
                        }}
                      />
                      {@const duration = getVideoDuration(item, itemIndex)}
                      <span class="filmstrip-badge" aria-hidden="true">
                        <IconVideo />
                        {#if duration}
                          <span class="filmstrip-duration">{formatVideoDuration(duration)}</span>
                        {/if}
                      </span>
                    {:else}
                      <div class="filmstrip-fallback">
                        <IconVideo />
                      </div>
                    {/if}
                  {:else if item.kind === 'video'}
                    {@const duration = getVideoDuration(item, itemIndex)}
                    <div class="filmstrip-fallback">
                      <IconVideo />
                      {#if duration}
                        <span class="filmstrip-badge" aria-hidden="true">
                          <IconVideo />
                          <span class="filmstrip-duration">{formatVideoDuration(duration)}</span>
                        </span>
                      {/if}
                    </div>
                  {:else if item.kind === 'audio'}
                    <div class="filmstrip-fallback">
                      <IconMusic />
                    </div>
                  {:else}
                    <div class="filmstrip-fallback">
                      <IconDocument />
                    </div>
                  {/if}
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/if}
    </footer>
  {/if}
</div>

<style>
  .media-viewer {
    position: fixed;
    inset: 0;
    z-index: var(--z-viewer, 2147483000);
    overflow: hidden;
    background: rgba(0, 0, 0, 0.96);
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
    min-height: 72px;
    justify-content: space-between;
    gap: 16px;
    padding: max(14px, env(safe-area-inset-top)) max(18px, env(safe-area-inset-right)) 14px max(18px, env(safe-area-inset-left));
    background: linear-gradient(to bottom, rgba(0, 0, 0, 0.6) 0%, rgba(0, 0, 0, 0.15) 60%, transparent 100%);
  }

  .media-viewer-bottom {
    bottom: 0;
    min-height: 84px;
    justify-content: center;
    flex-direction: column;
    gap: 10px;
    padding: 12px max(18px, env(safe-area-inset-right)) max(14px, env(safe-area-inset-bottom)) max(18px, env(safe-area-inset-left));
    background: linear-gradient(to top, rgba(0, 0, 0, 0.6) 0%, rgba(0, 0, 0, 0.15) 60%, transparent 100%);
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
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.7);
  }

  .media-viewer-identity span {
    color: rgba(255, 255, 255, 0.7);
    font-size: 12px;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.7);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .media-viewer-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .mobile-zoom-controls {
    display: none !important;
  }

  :global(.media-viewer .viewer-icon-btn),
  :global(.media-viewer .viewer-reset-btn) {
    height: 44px !important;
    min-height: 44px !important;
    border: 0 !important;
    background: transparent !important;
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
    box-shadow: none !important;
    color: rgba(255, 255, 255, 0.85) !important;
    transition: color 150ms ease, background 150ms ease, transform 150ms ease !important;
  }

  :global(.media-viewer .viewer-icon-btn) {
    display: inline-flex !important;
    align-items: center !important;
    justify-content: center !important;
    width: 44px !important;
    min-width: 44px !important;
    padding: 0 !important;
    border-radius: var(--radius-full, 9999px) !important;
  }

  :global(.media-viewer .viewer-icon-btn:hover),
  :global(.media-viewer .viewer-reset-btn:hover) {
    background: rgba(255, 255, 255, 0.12) !important;
    color: #fff !important;
  }

  :global(.media-viewer .viewer-delete-btn:hover) {
    color: var(--color-danger, #ef4444) !important;
    background: rgba(239, 68, 68, 0.15) !important;
  }

  :global(.media-viewer .viewer-icon-btn:active),
  :global(.media-viewer .viewer-reset-btn:active) {
    background: rgba(255, 255, 255, 0.2) !important;
    transform: scale(0.94) !important;
  }

  :global(.media-viewer .viewer-icon-btn svg),
  :global(.media-viewer .viewer-reset-btn svg) {
    width: 22px !important;
    height: 22px !important;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.6));
  }

  .zoom-value {
    height: 36px;
    min-width: 48px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: rgba(255, 255, 255, 0.85);
    font-family: var(--font-sans);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
    transition: color 150ms ease, background 150ms ease;
  }

  .zoom-value:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
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

  .media-viewer-stage.can-swipe:active {
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
    transition: transform 180ms cubic-bezier(0.2, 0, 0, 1), opacity 180ms ease-out;
    will-change: transform, opacity;
  }

  .media-fit-frame.is-swiping {
    transition: none !important;
  }

  .media-fit-frame.is-sliding-out {
    transition: transform 120ms ease-in, opacity 120ms ease-in !important;
  }

  .media-fit-frame.is-sliding-in {
    transition: transform 160ms cubic-bezier(0.16, 1, 0.3, 1), opacity 160ms ease-out !important;
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



  .media-viewer-media:is(video) {
    width: min(100%, 1600px);
    max-height: 100%;
    background: #000;
  }

  :global(.viewer-nav) {
    position: absolute !important;
    top: 50%;
    z-index: 20;
    width: 56px !important;
    height: 56px !important;
    min-width: 56px !important;
    padding: 0 !important;
    border: 0 !important;
    border-radius: 50% !important;
    background: transparent !important;
    box-shadow: none !important;
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
    color: rgba(255, 255, 255, 0.75) !important;
    transform: translateY(-50%);
    transition: opacity 180ms ease, transform 180ms ease, background 150ms ease, color 150ms ease !important;
  }

  :global(.viewer-nav:hover) {
    background: rgba(255, 255, 255, 0.12) !important;
    color: #fff !important;
    transform: translateY(-50%) scale(1.08) !important;
  }

  :global(.viewer-nav:active) {
    background: rgba(255, 255, 255, 0.2) !important;
    transform: translateY(-50%) scale(0.96) !important;
  }

  :global(.viewer-nav svg) {
    width: 32px;
    height: 32px;
    filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.7));
    transition: transform 150ms var(--ease-expo, ease-out);
  }

  :global(.viewer-prev:hover svg) { transform: translateX(-2px); }
  :global(.viewer-next:hover svg) { transform: translateX(2px); }

  :global(.viewer-prev) { left: max(18px, env(safe-area-inset-left)); }
  :global(.viewer-next) { right: max(18px, env(safe-area-inset-right)); }

  .viewer-filmstrip-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    max-width: 100%;
    margin: 0;
    mask-image: linear-gradient(to right, transparent 0%, black 28px, black calc(100% - 28px), transparent 100%);
    -webkit-mask-image: linear-gradient(to right, transparent 0%, black 28px, black calc(100% - 28px), transparent 100%);
  }

  .viewer-filmstrip {
    display: flex;
    align-items: center;
    justify-content: safe center;
    gap: 6px;
    width: 100%;
    box-sizing: border-box;
    padding: 6px 36px 10px;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    -ms-overflow-style: none;
    user-select: none;
    -webkit-overflow-scrolling: touch;
    cursor: grab;
    will-change: scroll-position;
    contain: content;
  }

  .viewer-filmstrip:active {
    cursor: grabbing;
  }

  .viewer-filmstrip::-webkit-scrollbar {
    display: none;
  }

  .filmstrip-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    flex: 0 0 auto;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
    user-select: none;
    outline: none;
    -webkit-tap-highlight-color: transparent;
  }

  .filmstrip-thumb {
    position: relative;
    height: 58px;
    min-width: 36px;
    max-width: 140px;
    display: grid;
    place-items: center;
    overflow: hidden;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: rgba(20, 20, 24, 0.7);
    color: rgba(255, 255, 255, 0.65);
    opacity: 0.55;
    transition: opacity 120ms ease, transform 120ms ease, box-shadow 120ms ease;
    contain: layout paint;
  }

  .filmstrip-item:hover .filmstrip-thumb {
    opacity: 0.88;
    background: rgba(255, 255, 255, 0.1);
    transform: translateY(-2px);
  }

  .filmstrip-item.active .filmstrip-thumb {
    opacity: 1;
    box-shadow: 0 0 0 2px var(--accent-primary), 0 0 16px color-mix(in srgb, var(--accent-primary) 60%, transparent);
    transform: translateY(-2px) scale(1.04);
    z-index: 2;
  }

  .filmstrip-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    pointer-events: none;
    display: block;
  }

  .filmstrip-fallback {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    color: rgba(255, 255, 255, 0.5);
    pointer-events: none;
  }

  .filmstrip-fallback :global(svg) {
    width: 24px;
    height: 24px;
  }

  .filmstrip-badge {
    position: absolute;
    bottom: 3px;
    right: 3px;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 2px 6px;
    height: 18px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.82);
    backdrop-filter: blur(4px);
    color: #fff;
    font-size: 11px;
    font-weight: 550;
    line-height: 1;
    pointer-events: none;
    letter-spacing: 0.02em;
    user-select: none;
    white-space: nowrap;
  }

  .filmstrip-badge :global(svg) {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }

  .filmstrip-duration {
    font-variant-numeric: tabular-nums;
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

  :global(.media-viewer .viewer-download-icon.is-downloaded) {
    background: transparent !important;
    color: var(--accent-primary) !important;
    box-shadow: none !important;
  }

  :global(.media-viewer .viewer-download-icon.is-downloaded svg) {
    color: var(--accent-primary) !important;
    stroke-width: 2.25;
    filter: drop-shadow(0 1px 4px rgba(0, 0, 0, 0.6));
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
    background: transparent !important;
    border: 1px solid rgba(255, 255, 255, 0.2) !important;
    color: #fff !important;
    box-shadow: none !important;
    transition: background 150ms ease, border-color 150ms ease !important;
  }

  :global(.media-viewer .viewer-download-pill:hover) {
    background: rgba(255, 255, 255, 0.12) !important;
    border-color: rgba(255, 255, 255, 0.35) !important;
  }

  :global(.media-viewer .viewer-download-pill.is-downloaded) {
    border-color: var(--accent-primary) !important;
    color: var(--accent-primary) !important;
  }

  :global(.media-viewer .viewer-ghost-action) {
    border: 1px solid rgba(255, 255, 255, 0.25) !important;
    background: transparent !important;
    color: #fff !important;
    box-shadow: none !important;
  }

  :global(.media-viewer .viewer-ghost-action:hover) {
    background: rgba(255, 255, 255, 0.12) !important;
    border-color: rgba(255, 255, 255, 0.4) !important;
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
      min-height: 58px;
      padding-top: max(8px, env(safe-area-inset-top));
      padding-right: max(8px, env(safe-area-inset-right));
      padding-left: max(12px, env(safe-area-inset-left));
    }

    .media-viewer-stage {
      padding: 0;
    }

    .media-fit-frame { inset: 60px 0 102px; }

    .media-viewer-actions { gap: 3px; }
    :global(.viewer-nav) { display: none !important; }
    :global(.media-viewer .viewer-zoom-btn) { display: none !important; }

    :global(.media-viewer .viewer-icon-btn) {
      width: 38px !important;
      min-width: 38px !important;
      height: 38px !important;
      min-height: 38px !important;
    }

    :global(.media-viewer .viewer-icon-btn svg) {
      width: 20px !important;
      height: 20px !important;
    }

    .mobile-zoom-controls {
      display: flex !important;
      align-items: center;
      justify-content: center;
      gap: 8px;
    }

    :global(.media-viewer .viewer-reset-btn) {
      display: inline-flex !important;
      align-items: center !important;
      justify-content: center !important;
      gap: 6px !important;
      min-width: 80px !important;
      height: 38px !important;
      padding: 0 12px !important;
      border-radius: var(--radius-full) !important;
      font-size: 12.5px !important;
      font-weight: 600 !important;
    }

    .media-viewer-bottom {
      min-height: auto;
      padding: 4px max(8px, env(safe-area-inset-right)) max(10px, env(safe-area-inset-bottom)) max(8px, env(safe-area-inset-left));
      gap: 6px;
    }

    .viewer-filmstrip-wrapper {
      max-width: 100vw;
      mask-image: linear-gradient(to right, transparent 0%, black 16px, black calc(100% - 16px), transparent 100%);
      -webkit-mask-image: linear-gradient(to right, transparent 0%, black 16px, black calc(100% - 16px), transparent 100%);
    }

    .viewer-filmstrip {
      gap: 8px;
      padding: 6px 16px 8px;
    }

    .filmstrip-thumb {
      height: 64px;
      min-width: 40px;
      max-width: 140px;
      flex: 0 0 auto;
      border-radius: 6px;
    }

    .media-viewer-identity strong { font-size: 13px; }
    .media-viewer-identity { max-width: calc(100vw - 145px); }
  }

  @media (max-width: 380px) {
    .media-viewer-identity { max-width: calc(100vw - 110px); }
    :global(.media-viewer .viewer-fullscreen-btn) { display: none !important; }
    .filmstrip-thumb {
      height: 56px;
      min-width: 34px;
      max-width: 120px;
      border-radius: 6px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .media-viewer,
    .media-viewer-controls,
    .media-fit-frame,
    .image-media,
    :global(.viewer-nav),
    .filmstrip-item,
    .filmstrip-thumb {
      animation: none !important;
      transition: none !important;
    }
  }
</style>
