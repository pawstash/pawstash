import { invoke } from '@tauri-apps/api/core';
import { playbackState } from '$lib/state/playbackState.svelte';

const thumbnailMemoryCache = new Map<string, string>();
const pendingRequests = new Map<string, Promise<string | undefined>>();

interface QueueItem {
  key: string;
  videoUrl: string;
  resolve: (val: string | undefined) => void;
}

const queue: QueueItem[] = [];
let isProcessing = false;

async function processQueue() {
  if (isProcessing || queue.length === 0) return;
  isProcessing = true;

  const item = queue.shift();
  if (!item) {
    isProcessing = false;
    return;
  }

  try {
    const dataUrl = await extractFrame(item.videoUrl, item.key);
    if (dataUrl) {
      thumbnailMemoryCache.set(item.key, dataUrl);
      try {
        await invoke('store_video_thumbnail', { key: item.key, dataUrl });
      } catch (err) {
        console.warn('[VideoThumbnail] Failed to persist thumbnail to backend:', err);
      }
      item.resolve(dataUrl);
    } else {
      item.resolve(undefined);
    }
  } catch (err) {
    console.warn('[VideoThumbnail] Extraction exception for key', item.key, err);
    item.resolve(undefined);
  } finally {
    isProcessing = false;
    setTimeout(processQueue, 32);
  }
}

function extractFrame(videoUrl: string, key?: string): Promise<string | undefined> {
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

        const targetWidth = 360;
        const targetHeight = Math.max(120, Math.round(targetWidth * (vh / vw)));
        const canvas = document.createElement('canvas');
        canvas.width = targetWidth;
        canvas.height = targetHeight;
        const ctx = canvas.getContext('2d');
        if (!ctx) {
          return false;
        }

        ctx.drawImage(video, 0, 0, targetWidth, targetHeight);
        let dataUrl = canvas.toDataURL('image/webp', 0.8);
        if (!dataUrl || dataUrl.length < 50) {
          dataUrl = canvas.toDataURL('image/jpeg', 0.8);
        }
        if (dataUrl && dataUrl.length > 50) {
          done(dataUrl);
          return true;
        }
      } catch (err) {
        console.warn('[VideoThumbnail] Canvas capture error:', err);
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

export async function getVideoThumbnail(key: string, videoUrl?: string): Promise<string | undefined> {
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
    } catch {
      // Backend cache miss
    }

    if (!videoUrl) return undefined;

    return new Promise<string | undefined>((resolve) => {
      queue.push({ key, videoUrl, resolve });
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
