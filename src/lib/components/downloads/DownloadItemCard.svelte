<script lang="ts">
  import { haptic } from '$lib/utils/haptics';
  import { videoPoster } from '$lib/utils/media';
  import { isAutoRetryPending, type DownloadItem } from '$lib/types/download';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes, formatDuration, formatSpeed } from '$lib/utils/formatters';
  import { notify } from '$lib/utils/toast';
  import { ripple, tooltip } from '$lib/motion';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { contextMenuState } from '$lib/state/contextMenuState.svelte';
  import { downloadMenuItems } from '$lib/utils/contextMenus';
  import { apiOpenDownloadFile, apiShowInFolder } from '$lib/utils/ipc';
  import IconCheckmark from '~icons/fluent/checkmark-20-filled';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconError from '~icons/fluent/error-circle-24-filled';
  import IconPause from '~icons/fluent/pause-20-regular';
  import IconPlay from '~icons/fluent/play-20-regular';
  import IconRetry from '~icons/fluent/arrow-counterclockwise-20-regular';
  import IconDelete from '~icons/fluent/delete-20-regular';
  import IconDismiss from '~icons/fluent/dismiss-20-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import { getMediaThumbnail } from '$lib/utils/mediaThumbnail';
  import { thumbnailKey } from '$lib/utils/cacheKey';
  import { playbackState } from '$lib/state/playbackState.svelte';

  interface Props {
    item: DownloadItem;
    previewUrl?: string;
    thumbnailUrl?: string;
    postTitle?: string;
    onopen?: (openViewer?: boolean) => void;
    orderedKeys?: string[];
    itemsMap?: Map<string, DownloadItem>;
  }
  let { item, previewUrl, thumbnailUrl, postTitle, onopen, orderedKeys, itemsMap }: Props = $props();
  const ratios = { square: '1 / 1', portrait: '4 / 5', landscape: '3 / 2', widescreen: '16 / 9' } as const;
  let busy = $state(false);
  let lastThumbnailUrl = $state<string | undefined>(undefined);
  let thumbnailFailed = $state(false);
  let previewFailed = $state(false);
  let generatedThumbnail = $state<string | undefined>(undefined);
  let effectiveThumbnail = $derived(generatedThumbnail || (!thumbnailFailed && thumbnailUrl ? thumbnailUrl : undefined));
  let playMenuOpen = $state(false);
  let ratio = $derived(ratios[configState.settings.grid_aspect_ratio]);
  let percent = $derived(item.total_bytes > 0 ? Math.min(100, Math.round(item.downloaded_bytes / item.total_bytes * 100)) : 0);
  let active = $derived(['queued', 'resolving', 'downloading', 'verifying'].includes(item.status));
  let retryPending = $derived(isAutoRetryPending(item));
  let extension = $derived(item.filename.includes('.') ? (item.filename.split('.').pop() || '').toLowerCase() : '');
  let mediaKind = $derived(/^(avif|bmp|gif|heic|heif|ico|jpe?g|png|svg|tiff?|webp)$/.test(extension) ? 'image' : /^(avi|flv|m4v|mkv|mov|mp4|webm|wmv)$/.test(extension) ? 'video' : /^(aac|flac|m4a|mp3|ogg|opus|wav)$/.test(extension) ? 'audio' : 'file');


  let videoDuration = $derived.by(() => {
    if (mediaKind !== 'video') return undefined;
    const key = item.media_id || item.id || item.filename;
    return playbackState.getDuration(key);
  });

  let metaBadge = $derived.by(() => {
    const ext = extension ? extension.toUpperCase() : '';
    if (mediaKind === 'video' && videoDuration && videoDuration > 0) {
      const dur = formatDuration(videoDuration);
      return ext ? `${ext} · ${dur}` : dur;
    }
    return ext;
  });

  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'downloads');
  let selected = $derived(isSelectionActive && selectionState.isSelected(item.id));
  let isHovered = $state(false);
  let showVideo = $state(false);
  let videoEl = $state<HTMLVideoElement | null>(null);
  let playTimeout: ReturnType<typeof setTimeout> | undefined;

  function handleMouseEnter() {
    isHovered = true;
    if (mediaKind === 'video' && !previewFailed && previewUrl) {
      if (playTimeout) clearTimeout(playTimeout);
      playTimeout = setTimeout(() => {
        if (isHovered) {
          showVideo = true;
        }
      }, 180);
    }
  }

  function handleMouseLeave() {
    isHovered = false;
    if (playTimeout) clearTimeout(playTimeout);
    showVideo = false;
    if (videoEl) {
      videoEl.pause();
    }
  }

  function requestMediaThumbnail() {
    if (generatedThumbnail || !previewUrl) return;
    if (mediaKind !== 'video' && mediaKind !== 'image') return;
    const key = thumbnailKey(item.media_id || item.id || item.filename, {
      service: item.service ?? '',
      creator_id: item.creator_id ?? '',
      id: item.post_id ?? ''
    });
    getMediaThumbnail(key, previewUrl, mediaKind).then((thumb) => {
      if (thumb) {
        generatedThumbnail = thumb;
      }
    });
  }

  $effect(() => {
    if (thumbnailUrl !== lastThumbnailUrl) {
      lastThumbnailUrl = thumbnailUrl;
      thumbnailFailed = false;
    }
    if (previewUrl && (mediaKind === 'video' || mediaKind === 'image')) {
      if (!generatedThumbnail && (!thumbnailUrl || thumbnailFailed || (item.status === 'completed' && !item.file_preview_path))) {
        requestMediaThumbnail();
      }
    }
  });

  let lastContextTime = 0;

  function toggleSelected(range = false) {
    selectionState.toggle('downloads', item.id, item, orderedKeys, range, itemsMap);
  }

  function handleCardClick(event: MouseEvent) {
    if (Date.now() - lastContextTime < 500) return;

    if (event.ctrlKey || event.metaKey || isSelectionActive) {
      event.preventDefault();
      event.stopPropagation();
      toggleSelected(event.shiftKey && !event.ctrlKey && !event.metaKey);
      return;
    }

    if (onopen) onopen(false);
  }

  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();
    lastContextTime = Date.now();
    haptic();
    if (isSelectionActive) toggleSelected();
    else contextMenuState.show(event, item.filename, downloadMenuItems(item, { onselect: () => toggleSelected(), onopen, hasPost: Boolean(item.post_id) }));
  }

  function handleSelectCheckbox(event: MouseEvent) {
    event.stopPropagation();
    toggleSelected(event.shiftKey);
  }

  async function openFileExternally() {
    try {
      await apiOpenDownloadFile(item.final_path);
    } catch (error) {
      notify.error(i18n.t('downloads.open_file_failed'), error);
    }
  }

  async function showFileInFolder() {
    try {
      await apiShowInFolder(item.final_path);
    } catch (error) {
      notify.error(i18n.t('downloads.show_in_folder_failed'), error);
    }
  }

  async function action(event: MouseEvent, run: () => Promise<void>) {
    event.stopPropagation();
    if (busy) return;
    busy = true;
    try { await run(); }
    catch (error) { notify.error(i18n.t('downloads.action_error'), error); }
    finally { busy = false; }
  }
</script>

<article
  class="grid-tile download-tile"
  class:selected={selected}
  class:completed={item.status === 'completed'}
  style:aspect-ratio={ratio}
  data-download-id={item.id}
  data-select-key={item.id}
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
>
  <button
    class="grid-tile-open"
    type="button"
    onclick={handleCardClick}
    oncontextmenu={handleContextMenu}
    aria-label={[item.filename, postTitle].filter(Boolean).join(', ')}
  ></button>

  {#if isSelectionActive}
    <button
      type="button"
      class="grid-tile-select-checkbox"
      class:checked={selected}
      onclick={handleSelectCheckbox}
      aria-label={i18n.t('selection.select_download')}
      aria-checked={selected}
      role="checkbox"
    >
      {#if selected}
        <IconCheckmark class="w-[14px] h-[14px]" />
      {/if}
    </button>
  {:else if item.status === 'completed'}
    <div class="download-actions-left tile-toolbar">
      <PopoverMenu
        bind:open={playMenuOpen}
        align="left"
        width="auto"
        menuClass="download-play-popover"
      >
        {#snippet trigger({ toggle })}
          <button
            class="grid-tile-action download-action download-action-play"
            class:active={playMenuOpen}
            type="button"
            onclick={(e) => { e.stopPropagation(); toggle(e); }}
            use:ripple
            use:tooltip={i18n.t('downloads.open_externally')}
            aria-label={i18n.t('downloads.open_externally')}
          >
            <IconPlay />
          </button>
        {/snippet}
        {#snippet children({ close })}
          <div class="download-play-menu">
            {#if onopen}
              <button
                type="button"
                class="floating-item is-two-line"
                use:ripple
                onclick={(e) => { e.stopPropagation(); close(); onopen?.(true); }}
              >
                <IconDocument />
                <div class="floating-item-text">
                  <span>{i18n.t('downloads.open_in_post')}</span>
                  <span class="floating-item-desc">{i18n.t('downloads.open_in_post_desc')}</span>
                </div>
              </button>
            {/if}
            <button
              type="button"
              class="floating-item is-two-line"
              use:ripple
              onclick={(e) => { e.stopPropagation(); close(); void openFileExternally(); }}
            >
              <IconOpen />
              <div class="floating-item-text">
                <span>{i18n.t('downloads.open_in_system')}</span>
                <span class="floating-item-desc">{i18n.t('downloads.open_in_system_desc')}</span>
              </div>
            </button>
            <button
              type="button"
              class="floating-item is-two-line"
              use:ripple
              onclick={(e) => { e.stopPropagation(); close(); void showFileInFolder(); }}
            >
              <IconFolder />
              <div class="floating-item-text">
                <span>{i18n.t('downloads.show_in_folder')}</span>
                <span class="floating-item-desc">{i18n.t('downloads.show_in_folder_desc')}</span>
              </div>
            </button>
          </div>
        {/snippet}
      </PopoverMenu>
    </div>
  {/if}

  {#if effectiveThumbnail}
    <img
      class="grid-tile-media"
      src={effectiveThumbnail}
      alt=""
      loading="lazy"
      decoding="async"
      onerror={() => {
        if (!thumbnailFailed && effectiveThumbnail === thumbnailUrl) {
          thumbnailFailed = true;
          requestMediaThumbnail();
        }
      }}
    />
  {:else}
    <div class="grid-tile-placeholder download-placeholder">
      {#if mediaKind === 'audio'}<IconMusic />{:else if mediaKind === 'video'}<IconVideo />{:else if mediaKind === 'image'}<IconImage />{:else}<IconDocument />{/if}
      {#if extension}<span>{extension}</span>{/if}
    </div>
  {/if}

  {#if showVideo && mediaKind === 'video' && previewUrl && !previewFailed}
    <video
      bind:this={videoEl}
      class="grid-tile-media grid-tile-hover-video"
      src={previewUrl}
      poster={videoPoster()}
      muted
      loop
      playsinline
      disablepictureinpicture
      disableremoteplayback
      autoplay
      onloadedmetadata={(e) => {
        const vid = e.currentTarget as HTMLVideoElement;
        if (vid.duration && isFinite(vid.duration) && vid.duration > 0) {
          const key = item.media_id || item.id || item.filename;
          if (key) playbackState.saveDuration(key, vid.duration);
        }
      }}
      onerror={() => { previewFailed = true; showVideo = false; }}
    ></video>
  {/if}

  <div class="grid-tile-shade"></div>

  <div class="download-actions tile-toolbar">
    {#if active}
      <button class="grid-tile-action download-action" onclick={(event) => action(event, () => downloadState.pause(item.id))} use:tooltip={i18n.t('downloads.pause')} aria-label={i18n.t('downloads.pause')}><IconPause /></button>
      <button class="grid-tile-action download-action grid-tile-action-danger" onclick={(event) => action(event, () => downloadState.remove(item.id))} use:tooltip={i18n.t('downloads.cancel')} aria-label={i18n.t('downloads.cancel')}><IconDismiss /></button>
    {:else if item.status === 'paused'}
      <button class="grid-tile-action download-action" onclick={(event) => action(event, () => downloadState.resume(item.id))} use:tooltip={i18n.t('downloads.resume')} aria-label={i18n.t('downloads.resume')}><IconPlay /></button>
      <button class="grid-tile-action download-action grid-tile-action-danger" onclick={(event) => action(event, () => downloadState.remove(item.id))} use:tooltip={i18n.t('downloads.remove')} aria-label={i18n.t('downloads.remove')}><IconDelete /></button>
    {:else if ['failed', 'cancelled', 'missing'].includes(item.status)}
      <button class="grid-tile-action download-action" onclick={(event) => action(event, () => downloadState.retry(item.id))} use:tooltip={i18n.t('downloads.retry')} aria-label={i18n.t('downloads.retry')}><IconRetry /></button>
      <button class="grid-tile-action download-action grid-tile-action-danger" onclick={(event) => action(event, () => downloadState.remove(item.id))} use:tooltip={i18n.t('downloads.remove')} aria-label={i18n.t('downloads.remove')}><IconDelete /></button>
    {:else if item.status === 'completed'}
      <button class="grid-tile-action download-action grid-tile-action-danger" onclick={(event) => action(event, () => downloadState.remove(item.id))} use:tooltip={i18n.t('downloads.remove')} aria-label={i18n.t('downloads.remove')}><IconDelete /></button>
    {/if}
  </div>

  <div class="grid-tile-footer">
    <h2 class="grid-tile-title" use:tooltip={item.filename}>{item.filename}</h2>
    {#if postTitle}
      <p class="download-post-title" use:tooltip={postTitle}>{postTitle}</p>
    {/if}
    {#if item.status !== 'completed'}
      <div
        class="grid-tile-author download-status-row"
        data-status={retryPending ? 'retrying' : item.status}
        use:tooltip={item.status === 'failed' ? item.error_message : undefined}
      >
        {#if busy || retryPending}<IconLoading />{:else if item.status === 'failed' || item.status === 'missing'}<IconError />{:else}<IconDownload />{/if}
        <span class="grid-tile-author-name">{i18n.t(retryPending ? 'downloads.status_retrying' : `downloads.status_${item.status}`)}</span>
      </div>
    {/if}
    <div class="grid-tile-meta">
      <span>{item.total_bytes > 0 ? formatBytes(item.total_bytes) : formatBytes(item.downloaded_bytes)}</span>
      <div class="grid-tile-meta-stats">
        {#if item.status !== 'completed' && item.total_bytes > 0}<span>{percent}%</span>{/if}
        {#if item.speed_bps > 0}<span>{formatSpeed(item.speed_bps)}</span>{/if}
        {#if metaBadge}
          <span class="download-format-badge">{metaBadge}</span>
        {/if}
      </div>
    </div>
  </div>

  {#if item.status !== 'completed'}
    <div class="download-progress" class:indeterminate={active && item.total_bytes === 0}><span style:width={`${item.total_bytes > 0 ? percent : 0}%`}></span></div>
  {/if}
</article>

<style>
  :global(.grid-tile-hover-video) { pointer-events: none !important; }
  .download-format-badge {
    font-variant-numeric: tabular-nums;
    font-size: calc(var(--type-label-sm-size) * var(--grid-scale, 1));
    font-weight: 600;
    color: var(--on-media-muted);
    text-transform: uppercase;
  }
  .download-placeholder { gap: calc(8px * var(--grid-scale, 1)); color: var(--text-muted); }
  .download-placeholder :global(svg) { width: calc(34px * var(--grid-scale, 1)); height: calc(34px * var(--grid-scale, 1)); }
  .download-placeholder span { font-size: calc(9px * var(--grid-scale, 1)); font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
  .download-tile .grid-tile-title { word-break: break-word; }
  .download-post-title { width: 100%; margin: 0; color: var(--on-media-secondary); font-size: calc(var(--type-label-sm-size) * var(--grid-scale, 1)); font-weight: 500; line-height: 1.25; text-align: left; overflow-wrap: anywhere; word-break: break-word; display: -webkit-box; line-clamp: 1; -webkit-line-clamp: 1; -webkit-box-orient: vertical; overflow: hidden; }
  .download-actions-left { position: absolute; z-index: 6; top: var(--tile-chip-inset); left: var(--tile-chip-inset); }
  .download-actions-left .download-action { position: relative; inset: auto; flex: none; }
  .download-actions { position: absolute; z-index: 6; top: var(--tile-chip-inset); right: var(--tile-chip-inset); }
  .download-actions .download-action { position: relative; inset: auto; flex: none; }
  .download-status-row { pointer-events: none !important; }
  .download-status-row > :global(svg) { width: calc(15px * var(--grid-scale, 1)); height: calc(15px * var(--grid-scale, 1)); flex: none; color: var(--on-media-secondary); }
  .download-status-row .grid-tile-author-name { cursor: default; pointer-events: none; }

  .download-play-menu {
    display: flex;
    flex-direction: column;
    min-width: 260px;
    max-width: min(360px, calc(100vw - 32px));
  }
</style>
