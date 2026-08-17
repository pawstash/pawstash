<script lang="ts">
  import type { DownloadItem } from '$lib/types/download';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import { toast } from 'svelte-sonner';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconPause from '~icons/fluent/pause-20-regular';
  import IconPlay from '~icons/fluent/play-20-regular';
  import IconRetry from '~icons/fluent/arrow-counterclockwise-20-regular';
  import IconDelete from '~icons/fluent/delete-20-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  interface Props { items: DownloadItem[]; previewUrl?: string; avatarUrl?: string; title: string; creatorName: string; onopen?: () => void; oncreator?: () => void; }
  let { items, previewUrl, avatarUrl, title, creatorName, onopen, oncreator }: Props = $props();
  const ratios = { square: '1 / 1', portrait: '4 / 5', landscape: '3 / 2', widescreen: '16 / 9' } as const;
  let busy = $state(false);
  let previewFailed = $state(false);
  let ratio = $derived(ratios[configState.settings.grid_aspect_ratio]);
  let representative = $derived(items[0]);
  let previewExtension = $derived.by(() => {
    const source = previewUrl || representative?.filename || '';
    const withoutQuery = source.split(/[?#]/, 1)[0];
    const encodedExtension = withoutQuery.split('.').pop() || '';
    try { return decodeURIComponent(encodedExtension).toLowerCase(); }
    catch { return encodedExtension.toLowerCase(); }
  });
  let extension = $derived(previewExtension || (representative?.filename.split('.').pop() || '').toLowerCase());
  let isImage = $derived(/^(avif|bmp|gif|jpe?g|png|webp)$/.test(extension));
  let isVideo = $derived(/^(m4v|mkv|mov|mp4|webm)$/.test(extension));
  let pausableItems = $derived(items.filter((item) => ['resolving', 'downloading'].includes(item.status)));
  let pausedItems = $derived(items.filter((item) => item.status === 'paused'));
  let retryItems = $derived(items.filter((item) => ['failed', 'cancelled', 'missing'].includes(item.status)));
  let completedItems = $derived(items.filter((item) => item.status === 'completed'));
  let totalBytes = $derived(items.reduce((sum, item) => sum + Math.max(item.total_bytes, item.downloaded_bytes), 0));
  let downloadedBytes = $derived(items.reduce((sum, item) => sum + item.downloaded_bytes, 0));
  let progress = $derived(totalBytes > 0 ? Math.min(100, Math.round(downloadedBytes / totalBytes * 100)) : 0);

  async function runGroup(event: MouseEvent, jobs: DownloadItem[], operation: (id: string) => Promise<void>) {
    event.stopPropagation();
    if (busy || !jobs.length) return;
    busy = true;
    try { await Promise.all(jobs.map((item) => operation(item.id))); }
    catch (error) { toast.error(i18n.t('downloads.action_error'), { description: String(error) }); }
    finally { busy = false; }
  }
</script>

<article class="grid-tile download-group-tile" style:aspect-ratio={ratio}>
  {#if onopen}<button class="grid-tile-open" type="button" onclick={onopen} aria-label={title}></button>{/if}
  {#if previewUrl && !previewFailed && isImage}
    <img class="grid-tile-media" src={previewUrl} alt="" loading="lazy" decoding="async" onerror={() => previewFailed = true} />
  {:else if previewUrl && !previewFailed && isVideo}
    <video class="grid-tile-media" src={previewUrl} muted playsinline preload="metadata" onerror={() => previewFailed = true}></video>
  {:else}
    <div class="grid-tile-placeholder group-placeholder"><IconDocument /></div>
  {/if}
  <div class="grid-tile-shade"></div>

  <div class="group-actions">
    {#if busy}
      <span class="grid-tile-action group-action busy"><IconLoading /></span>
    {:else}
      {#if pausableItems.length}
        <button class="grid-tile-action group-action" onclick={(event) => runGroup(event, pausableItems, (id) => downloadState.pause(id))} title={i18n.t('downloads.pause')} aria-label={i18n.t('downloads.pause')}><IconPause /></button>
      {:else if pausedItems.length}
        <button class="grid-tile-action group-action" onclick={(event) => runGroup(event, pausedItems, (id) => downloadState.resume(id))} title={i18n.t('downloads.resume')} aria-label={i18n.t('downloads.resume')}><IconPlay /></button>
      {:else if retryItems.length}
        <button class="grid-tile-action group-action" onclick={(event) => runGroup(event, retryItems, (id) => downloadState.retry(id))} title={i18n.t('downloads.retry')} aria-label={i18n.t('downloads.retry')}><IconRetry /></button>
      {:else if completedItems.length === items.length}
        <button class="grid-tile-action group-action" onclick={(event) => runGroup(event, items, (id) => downloadState.remove(id))} title={i18n.t('downloads.remove')} aria-label={i18n.t('downloads.remove')}><IconDelete /></button>
      {/if}
    {/if}
  </div>

  <h2 class="grid-tile-title">{title}</h2>
  <div class="grid-tile-footer">
    <button class="grid-tile-author group-media-row" type="button" onclick={(event) => { event.stopPropagation(); oncreator?.(); }}>
      {#if avatarUrl}<img class="group-avatar" src={avatarUrl} alt="" />{:else}<IconDownload />{/if}
      <span class="grid-tile-author-name">{creatorName}</span>
    </button>
    <div class="grid-tile-meta">
      <span>{formatBytes(totalBytes)}</span>
      <div class="grid-tile-meta-stats">
        <span>{completedItems.length}/{items.length}</span>
        {#if completedItems.length !== items.length}<span>{progress}%</span>{/if}
      </div>
    </div>
  </div>

  {#if completedItems.length !== items.length}<div class="group-progress"><span style:width={`${progress}%`}></span></div>{/if}
</article>

<style>
  .group-placeholder { color: rgba(255,255,255,.25); }
  .group-placeholder :global(svg) { width: calc(38px * var(--grid-scale, 1)); height: calc(38px * var(--grid-scale, 1)); }
  .download-group-tile .grid-tile-title { overflow-wrap: anywhere; word-break: break-word; }
  .group-actions { position: absolute; z-index: 6; top: calc(8px * var(--grid-scale, 1)); right: calc(8px * var(--grid-scale, 1)); display: flex; gap: calc(5px * var(--grid-scale, 1)); }
  .group-actions .group-action { position: relative; inset: auto; flex: none; }
  .group-action.busy { opacity: 1; cursor: wait; }
  .group-media-row { pointer-events: auto !important; justify-content: flex-start; text-align: left; cursor: pointer; }
  .group-media-row > :global(svg) { width: calc(15px * var(--grid-scale, 1)); height: calc(15px * var(--grid-scale, 1)); flex: none; color: rgba(255,255,255,.7); }
  .group-media-row .grid-tile-author-name { cursor: pointer; pointer-events: none; text-align: left; }
  .group-media-row { position: relative; z-index: 8; border: 0; background: transparent; color: inherit; padding: 0; }
  .group-avatar { width: calc(18px * var(--grid-scale, 1)); height: calc(18px * var(--grid-scale, 1)); border-radius: 50%; object-fit: cover; }
  .group-progress { position: absolute; z-index: 7; inset: auto 0 0; height: calc(3px * var(--grid-scale, 1)); overflow: hidden; background: rgba(255,255,255,.16); pointer-events: none; }
  .group-progress span { display: block; height: 100%; background: var(--accent-primary); transition: width 220ms var(--ease-expo); }
</style>
