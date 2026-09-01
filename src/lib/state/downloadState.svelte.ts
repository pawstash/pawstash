import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { DownloadItem } from '$lib/types/download';
import {
  apiCancelDownload,
  apiListDownloads,
  apiPauseDownload,
  apiRemoveDownload,
  apiResumeDownload,
  apiRetryDownload,
  apiStartDownload,
  apiFetchPost,
  apiResolveCloudLink
} from '$lib/utils/ipc';
import type { Post } from '$lib/types/content';
import { getPostDownloadTargets, extractCloudLinks } from '$lib/utils/media';
import { contentState, postCacheKey } from '$lib/state/contentState.svelte';
import { serverPortState } from '$lib/state/serverPort.svelte';
import { logger } from '$lib/utils/logger';

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

  async resolveFullPost(post: Post): Promise<Post> {
    if (post.detail_fetched) return post;
    const key = postCacheKey(post.service, post.user, post.id);
    const cached = contentState.posts[key]?.post;
    if (cached?.detail_fetched) return cached;

    try {
      const detail = await apiFetchPost(String(post.service), String(post.user), String(post.id));
      if (detail) {
        const merged: Post = {
          ...post,
          ...detail,
          favorite_count: post.favorite_count ?? detail.favorite_count,
          attachment_count: detail.attachments?.length ?? post.attachment_count,
          detail_fetched: true
        };
        contentState.seedPost(merged);
        return merged;
      }
    } catch {
      // ignore and use fallback post
    }
    return post;
  }

  async downloadPost(post: Post): Promise<number> {
    const fullPost = await this.resolveFullPost(post);
    const targets = getPostDownloadTargets(fullPost);
    let count = 0;
    const seenMediaIds = new Set<string>();

    for (const target of targets) {
      if (seenMediaIds.has(target.mediaId)) continue;
      seenMediaIds.add(target.mediaId);
      await this.start(fullPost, target.mediaId, target.url, target.filename);
      count++;
    }

    const contentSources = [
      fullPost.content,
      fullPost.substring,
      (fullPost.embed as any)?.url,
      (fullPost.embed as any)?.description
    ].filter(Boolean).join(' ');

    const cloudLinks = extractCloudLinks(contentSources);
    if (cloudLinks.length > 0) {
      await serverPortState.ensurePort();
      const port = serverPortState.port || 0;
      for (const cloudUrl of cloudLinks) {
        try {
          const res = await apiResolveCloudLink(cloudUrl);
          if (res?.nodes && Array.isArray(res.nodes)) {
            const fileNodes = res.nodes.filter((n) => !n.is_folder);
            for (const node of fileNodes) {
              let targetUrl = node.download_url || node.stream_url || '';
              if (targetUrl.startsWith('/cloud_stream/') && port > 0) {
                targetUrl = `http://127.0.0.1:${port}${targetUrl}`;
              }
              const mediaId = node.stream_url || node.download_url || `cloud:${res.provider}:${node.id}`;
              if (targetUrl && !seenMediaIds.has(mediaId)) {
                seenMediaIds.add(mediaId);
                await this.start(fullPost, mediaId, targetUrl, node.name);
                count++;
              }
            }
          }
        } catch (err) {
          logger.warn(`Failed to resolve cloud link during post download: ${cloudUrl}`, err);
        }
      }
    }

    return count;
  }

  async downloadPosts(posts: Post[]): Promise<number> {
    let total = 0;
    for (const post of posts) {
      total += await this.downloadPost(post);
    }
    return total;
  }
}

export const downloadState = new DownloadState();
