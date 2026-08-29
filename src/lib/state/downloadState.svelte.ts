import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { DownloadItem } from '$lib/types/download';
import {
  apiCancelDownload,
  apiListDownloads,
  apiPauseDownload,
  apiRemoveDownload,
  apiResumeDownload,
  apiRetryDownload,
  apiStartDownload
} from '$lib/utils/ipc';
import type { Post } from '$lib/types/content';

export type DownloadFilter = 'active' | 'completed' | 'all';

export class DownloadState {
  downloads = $state<DownloadItem[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);
  filter = $state<DownloadFilter>('active');
  private initPromise: Promise<void> | null = null;
  private unlisten: UnlistenFn | null = null;

  activeDownloadsCount = $derived(
    this.downloads.filter((item) =>
      ['queued', 'resolving', 'downloading', 'paused', 'verifying'].includes(item.status)
    ).length
  );

  filteredDownloads = $derived.by(() => {
    if (this.filter === 'completed') {
      return this.downloads.filter((item) => item.status === 'completed');
    }
    if (this.filter === 'active') {
      return this.downloads.filter((item) => item.status !== 'completed');
    }
    return this.downloads;
  });

  init() {
    if (this.initPromise) return this.initPromise;
    this.initPromise = this.initialize().catch((error) => {
      this.initPromise = null;
      throw error;
    });
    return this.initPromise;
  }

  private async initialize() {
    this.loading = true;
    try {
      this.unlisten = await listen<DownloadItem>('download-job-updated', (event) => {
        this.upsert(event.payload);
      });
      this.downloads = await apiListDownloads();
    } finally {
      this.loading = false;
    }
  }

  destroy() {
    this.unlisten?.();
    this.unlisten = null;
    this.initPromise = null;
  }

  upsert(item: DownloadItem) {
    const index = this.downloads.findIndex((download) => download.id === item.id);
    if (index < 0) {
      this.downloads = [item, ...this.downloads];
      return;
    }
    const target = this.downloads[index];
    Object.assign(target, item);
  }

  async refresh() {
    this.loading = true;
    this.error = null;
    try {
      this.downloads = await apiListDownloads();
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
      throw error;
    } finally {
      this.loading = false;
    }
  }

  async start(post: Post, mediaId: string, url: string, filename: string) {
    const item = await apiStartDownload(post, mediaId, url, filename);
    if (!this.downloads.some((download) => download.id === item.id)) {
      this.upsert(item);
    }
    return item;
  }

  async pause(id: string) {
    this.upsert(await apiPauseDownload(id));
  }

  async resume(id: string) {
    this.upsert(await apiResumeDownload(id));
  }

  async retry(id: string) {
    this.upsert(await apiRetryDownload(id));
  }

  async cancel(id: string) {
    this.upsert(await apiCancelDownload(id));
  }

  async remove(id: string) {
    if (await apiRemoveDownload(id)) {
      this.downloads = this.downloads.filter((item) => item.id !== id);
    }
  }
}

export const downloadState = new DownloadState();
