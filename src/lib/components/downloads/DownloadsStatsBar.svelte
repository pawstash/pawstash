<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import type { DownloadItem } from '$lib/types/download';

  type MediaType = 'image' | 'video' | 'audio' | 'file';

  interface Props {
    downloads: DownloadItem[];
  }

  let { downloads }: Props = $props();

  function getMediaType(item: DownloadItem): MediaType {
    const extension = (item.filename.split('.').pop() || '').toLowerCase();
    if (/^(avif|bmp|gif|jpe?g|png|webp)$/.test(extension)) return 'image';
    if (/^(m4v|mkv|mov|mp4|webm)$/.test(extension)) return 'video';
    if (/^(aac|flac|m4a|mp3|ogg|opus|wav)$/.test(extension)) return 'audio';
    return 'file';
  }

  let stats = $derived.by(() => {
    let totalBytes = 0;
    let videoBytes = 0;
    let imageBytes = 0;
    let audioBytes = 0;
    let fileBytes = 0;

    let videoCount = 0;
    let imageCount = 0;
    let audioCount = 0;
    let fileCount = 0;

    let completedCount = 0;
    let activeCount = 0;
    let activeBytesDownloaded = 0;
    let activeBytesTotal = 0;

    for (const d of downloads) {
      const bytes = Math.max(d.downloaded_bytes, d.total_bytes);
      totalBytes += bytes;

      if (d.status === 'completed') completedCount++;
      if (['queued', 'resolving', 'downloading'].includes(d.status)) {
        activeCount++;
        activeBytesDownloaded += d.downloaded_bytes;
        activeBytesTotal += d.total_bytes;
      }

      const type = getMediaType(d);
      if (type === 'video') { videoBytes += bytes; videoCount++; }
      else if (type === 'image') { imageBytes += bytes; imageCount++; }
      else if (type === 'audio') { audioBytes += bytes; audioCount++; }
      else { fileBytes += bytes; fileCount++; }
    }

    const categories = [
      {
        id: 'video' as MediaType,
        label: i18n.t('feed.format_video'),
        bytes: videoBytes,
        count: videoCount,
        color: '#38bdf8'
      },
      {
        id: 'image' as MediaType,
        label: i18n.t('feed.format_photo'),
        bytes: imageBytes,
        count: imageCount,
        color: '#a855f7'
      },
      {
        id: 'audio' as MediaType,
        label: i18n.t('feed.format_audio'),
        bytes: audioBytes,
        count: audioCount,
        color: '#34d399'
      },
      {
        id: 'file' as MediaType,
        label: i18n.t('feed.format_archive'),
        bytes: fileBytes,
        count: fileCount,
        color: '#fbbf24'
      }
    ]
      .filter((c) => c.bytes > 0)
      .map((c) => ({
        ...c,
        share: totalBytes > 0 ? (c.bytes / totalBytes) * 100 : 0
      }));

    return {
      totalBytes,
      completedCount,
      activeCount,
      activeBytesDownloaded,
      activeBytesTotal,
      totalCount: downloads.length,
      categories
    };
  });
</script>

<div class="flex flex-col gap-3 w-full">
  <div class="flex items-baseline justify-between gap-2">
    <div class="flex items-baseline gap-2">
      <span class="text-base font-semibold font-outfit text-white tracking-tight">
        {formatBytes(stats.totalBytes)}
      </span>
      <span class="text-xs text-white/40 font-mono">
        {#if stats.activeCount > 0}
          ({stats.completedCount} / {stats.totalCount} {i18n.t('settings.cache_files').toLowerCase()})
        {:else}
          ({stats.totalCount} {i18n.t('settings.cache_files').toLowerCase()})
        {/if}
      </span>
    </div>

    {#if stats.activeCount > 0}
      <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-accent/15 border border-accent/30 text-accent text-xs font-medium">
        <span class="w-1.5 h-1.5 rounded-full bg-accent animate-pulse"></span>
        <span>{stats.activeCount} {i18n.t('downloads.active').toLowerCase()}</span>
      </div>
    {/if}
  </div>

  <div class="storage-bar-track">
    {#if stats.categories.length === 0}
      <div class="h-full w-full bg-white/[0.04]"></div>
    {:else}
      <div class="flex h-full w-full">
        {#each stats.categories as cat (cat.id)}
          <div
            class="storage-bar-segment"
            style="width: {cat.share}%; background-color: {cat.color};"
            title="{cat.label}: {formatBytes(cat.bytes)} ({cat.count} files, {cat.share.toFixed(1)}%)"
          ></div>
        {/each}
      </div>
    {/if}
  </div>

  {#if stats.categories.length > 0}
    <div class="flex flex-wrap items-center gap-x-4 gap-y-1.5 pt-0.5">
      {#each stats.categories as cat (cat.id)}
        <div class="flex items-center gap-1.5 text-[11.5px] text-white/70">
          <span class="w-2 h-2 rounded-full shrink-0" style="background-color: {cat.color};"></span>
          <span class="text-white/40">{cat.label}:</span>
          <span class="font-mono text-white/90">{formatBytes(cat.bytes)}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .storage-bar-track {
    width: 100%;
    height: 9px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.07);
    overflow: hidden;
    display: flex;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.4);
  }

  .storage-bar-segment {
    height: 100%;
    transition: width 350ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .storage-bar-segment:first-child {
    border-top-left-radius: 9999px;
    border-bottom-left-radius: 9999px;
  }

  .storage-bar-segment:last-child {
    border-top-right-radius: 9999px;
    border-bottom-right-radius: 9999px;
  }
</style>
