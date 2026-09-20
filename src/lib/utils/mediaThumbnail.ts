import { invoke } from '@tauri-apps/api/core';
import { playbackState } from '$lib/state/playbackState.svelte';
import { logger } from '$lib/utils/logger';

const thumbnailMemoryCache = new Map<string, string>();
const pendingRequests = new Map<string, Promise<string | undefined>>();

export type MediaThumbnailKind = 'image' | 'video' | 'auto';

interface QueueItem {
  key: string;
  url: string;
  kind: 'image' | 'video';
  maxWidth: number;
  resolve: (val: string | undefined) => void;
}

const queue: QueueItem[] = [];
let isProcessing = false;

function detectKindFromUrl(url: string): 'image' | 'video' {
  const clean = url.split('?')[0].split('#')[0].toLowerCase();
  if (/\.(mp4|webm|mkv|mov|avi|flv|wmv|m4v)$/i.test(clean)) {
    return 'video';
  }
  return 'image';
}

async function processQueue() {
  if (isProcessing || queue.length === 0) return;
  isProcessing = true;

  const item = queue.shift();
  if (!item) {
    isProcessing = false;
    return;
  }

  try {
    const dataUrl = item.kind === 'video'
      ? await extractVideoThumbnail(item.url, item.key, item.maxWidth)
      : await extractImageThumbnail(item.url, item.maxWidth);

    if (dataUrl) {
      thumbnailMemoryCache.set(item.key, dataUrl);
      try {
        await invoke('store_video_thumbnail', { key: item.key, dataUrl });
      } catch (err) {
        logger.warn('Failed to persist thumbnail to backend', { key: item.key, error: err });
      }
      item.resolve(dataUrl);
    } else {
      item.resolve(undefined);
    }
  } catch (err) {
    logger.warn('Thumbnail extraction exception', { key: item.key, error: err });
    item.resolve(undefined);
  } finally {
    isProcessing = false;
    setTimeout(processQueue, 16);
  }
}

function extractImageThumbnail(imageUrl: string, maxWidth = 360): Promise<string | undefined> {
  return new Promise((resolve) => {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    let isResolved = false;

    const cleanup = () => {
      img.onload = null;
      img.onerror = null;
    };

    const done = (result?: string) => {
      if (isResolved) return;
      isResolved = true;
      clearTimeout(timeout);
      cleanup();
      resolve(result);
    };

    const timeout = setTimeout(() => {
      done(undefined);
    }, 5000);

    img.onload = () => {
      try {
        const w = img.naturalWidth || img.width;
        const h = img.naturalHeight || img.height;
        if (!w || !h) {
          done(undefined);
          return;
        }

        const targetWidth = Math.min(maxWidth, w);
        const targetHeight = Math.max(1, Math.round(targetWidth * (h / w)));
        const canvas = document.createElement('canvas');
        canvas.width = targetWidth;
        canvas.height = targetHeight;
        const ctx = canvas.getContext('2d');
        if (!ctx) {
          done(undefined);
          return;
        }

        ctx.drawImage(img, 0, 0, targetWidth, targetHeight);
        let dataUrl = canvas.toDataURL('image/webp', 0.82);
        if (!dataUrl || dataUrl.length < 50) {
          dataUrl = canvas.toDataURL('image/jpeg', 0.82);
        }
        if (dataUrl && dataUrl.length > 50) {
          done(dataUrl);
          return;
        }
      } catch (err) {
        logger.warn('Image canvas capture error', err);
      }
      done(undefined);
    };

    img.onerror = () => {
      done(undefined);
    };

    img.src = imageUrl;
  });
}

function extractVideoThumbnail(videoUrl: string, key?: string, maxWidth = 360): Promise<string | undefined> {
  return new Promise((resolve) => {
    const video = document.createElement('video');
    video.muted = true;
    video.playsInline = true;
    video.preload = 'auto';
    video.crossOrigin = 'anonymous';

    let isResolved = false;
    const cleanup = () => {
      video.removeAttribute('src');
      video.load();
      video.remove();
    };

    const done = (result?: string) => {
      if (isResolved) return;
      isResolved = true;
      clearTimeout(timeout);
      cleanup();
      resolve(result);
    };

    const timeout = setTimeout(() => {
      done(undefined);
    }, 6000);

    const tryCapture = () => {
      try {
        const vw = video.videoWidth;
        const vh = video.videoHeight;
        if (!vw || !vh) {
          return false;
        }

        const targetWidth = Math.min(maxWidth, vw);
        const targetHeight = Math.max(120, Math.round(targetWidth * (vh / vw)));
        const canvas = document.createElement('canvas');
        canvas.width = targetWidth;
        canvas.height = targetHeight;
        const ctx = canvas.getContext('2d');
        if (!ctx) {
          return false;
        }

        ctx.drawImage(video, 0, 0, targetWidth, targetHeight);
        let dataUrl = canvas.toDataURL('image/webp', 0.82);
        if (!dataUrl || dataUrl.length < 50) {
          dataUrl = canvas.toDataURL('image/jpeg', 0.82);
        }
        if (dataUrl && dataUrl.length > 50) {
          done(dataUrl);
          return true;
        }
      } catch (err) {
        logger.warn('Video canvas capture error', err);
      }
      return false;
    };

    video.onloadedmetadata = () => {
      if (key && video.duration && isFinite(video.duration) && video.duration > 0) {
        playbackState.saveDuration(key, video.duration);
      }
      const seekTime = video.duration > 1 ? 0.5 : Math.max(0.1, (video.duration || 1) / 2);
      video.currentTime = seekTime;
    };

    video.onseeked = () => {
      if (!tryCapture()) {
        setTimeout(tryCapture, 100);
      }
    };

    video.onloadeddata = () => {
      if (!tryCapture()) {
        const seekTime = video.duration > 1 ? 0.5 : Math.max(0.1, (video.duration || 1) / 2);
        video.currentTime = seekTime;
      }
    };

    video.oncanplay = () => {
      tryCapture();
    };

    video.onerror = () => {
      done(undefined);
    };

    video.src = videoUrl;
  });
}

export async function getMediaThumbnail(
  key: string,
  url?: string,
  kind: MediaThumbnailKind = 'auto',
  maxWidth = 360
): Promise<string | undefined> {
  if (!key) return undefined;

  if (thumbnailMemoryCache.has(key)) {
    return thumbnailMemoryCache.get(key);
  }

  if (pendingRequests.has(key)) {
    return pendingRequests.get(key);
  }

  const promise = (async () => {
    try {
      const cached = await invoke<string | null>('get_video_thumbnail', { key });
      if (cached) {
        thumbnailMemoryCache.set(key, cached);
        return cached;
      }
    } catch {}

    if (!url) return undefined;

    const resolvedKind = kind === 'auto' ? detectKindFromUrl(url) : kind;

    return new Promise<string | undefined>((resolve) => {
      queue.push({
        key,
        url,
        kind: resolvedKind,
        maxWidth,
        resolve
      });
      processQueue();
    });
  })();

  pendingRequests.set(key, promise);
  try {
    return await promise;
  } finally {
    pendingRequests.delete(key);
  }
}

export async function getVideoThumbnail(key: string, videoUrl?: string): Promise<string | undefined> {
  return getMediaThumbnail(key, videoUrl, 'video', 360);
}

export async function getPostThumbnail(
  post: { service?: string; user?: string; id?: string; thumbnail_url?: string; file?: { path?: string; thumbnail_url?: string } },
  targetUrl?: string
): Promise<string | undefined> {
  if (!post?.service || !post?.user || !post?.id) return undefined;
  const key = `post:${post.service}:${post.user}:${post.id}`;
  const url = targetUrl || post.thumbnail_url || post.file?.thumbnail_url || post.file?.path;
  return getMediaThumbnail(key, url, 'auto', 360);
}
