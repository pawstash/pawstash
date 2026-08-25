<script lang="ts">
  import type { DownloadItem } from '$lib/types/download';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import { notify } from '$lib/utils/toast';
  import { ripple, tooltip } from '$lib/motion';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { apiOpenDownloadFile, apiShowInFolder } from '$lib/utils/ipc';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
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
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';

  interface Props {
    item: DownloadItem;
    previewUrl?: string;
    postTitle?: string;
    onopen?: (openViewer?: boolean) => void;
    orderedKeys?: string[];
    itemsMap?: Map<string, DownloadItem>;
  }
  let { item, previewUrl, postTitle, onopen, orderedKeys, itemsMap }: Props = $props();
  const ratios = { square: '1 / 1', portrait: '4 / 5', landscape: '3 / 2', widescreen: '16 / 9' } as const;
  let busy = $state(false);
  let previewFailed = $state(false);
  let playMenuOpen = $state(false);
  let ratio = $derived(ratios[configState.settings.grid_aspect_ratio]);
  let percent = $derived(item.total_bytes > 0 ? Math.min(100, Math.round(item.downloaded_bytes / item.total_bytes * 100)) : 0);
  let active = $derived(['queued', 'resolving', 'downloading', 'verifying'].includes(item.status));
  let extension = $derived((item.filename.split('.').pop() || '').toLowerCase());
  let mediaKind = $derived(/^(avif|bmp|gif|jpe?g|png|webp)$/.test(extension) ? 'image' : /^(m4v|mkv|mov|mp4|webm)$/.test(extension) ? 'video' : /^(aac|flac|m4a|mp3|ogg|opus|wav)$/.test(extension) ? 'audio' : 'file');

  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'downloads');
  let selected = $derived(isSelectionActive && selectionState.isSelected(item.id));

  function handleCardClick(event: MouseEvent) {
    if (event.ctrlKey || event.metaKey) {
      event.preventDefault();
      event.stopPropagation();
      selectionState.toggle('downloads', item.id, item, orderedKeys, false, itemsMap);
      return;
    }

    if (isSelectionActive) {
      event.preventDefault();
      event.stopPropagation();
      selectionState.toggle('downloads', item.id, item, orderedKeys, event.shiftKey, itemsMap);
      return;
    }

    if (onopen) onopen(false);
  }

  function handleSelectCheckbox(event: MouseEvent) {
    event.stopPropagation();
    selectionState.toggle('downloads', item.id, item, orderedKeys, event.shiftKey, itemsMap);
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
      notify.error(i18n.t('downloads.show_in_folder_failed') || 'Failed to reveal file', error);
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
>
  <button class="grid-tile-open" type="button" onclick={handleCardClick} aria-label={[postTitle, item.filename].filter(Boolean).join(' — ')}></button>

  {#if isSelectionActive}
    <button
      type="button"
      class="grid-tile-select-checkbox"
      class:checked={selected}
      onclick={handleSelectCheckbox}
      aria-label="Select download"
    >
      {#if selected}
        <IconCheckmark class="w-[14px] h-[14px]" />
      {/if}
    </button>
  {:else if item.status === 'completed'}
    <div class="download-actions-left">
      <PopoverMenu
        bind:open={playMenuOpen}
        align="left"
        menuClass="download-play-popover"
      >
        {#snippet trigger({ toggle })}
          <button
            class="grid-tile-action download-action download-action-play"
            class:active={playMenuOpen}
            type="button"
            onclick={(e) => { e.stopPropagation(); toggle(e); }}
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
                class="download-play-menu-item"
                use:ripple
                onclick={(e) => { e.stopPropagation(); close(); onopen?.(true); }}
              >
                <IconDocument class="download-play-menu-icon" />
                <div class="download-play-menu-text">
                  <span class="download-play-menu-title">{i18n.t('downloads.open_in_post')}</span>
                  <span class="download-play-menu-desc">{i18n.t('downloads.open_in_post_desc')}</span>
                </div>
              </button>
            {/if}
            <button
              type="button"
              class="download-play-menu-item"
              use:ripple
              onclick={(e) => { e.stopPropagation(); close(); void openFileExternally(); }}
            >
              <IconOpen class="download-play-menu-icon" />
              <div class="download-play-menu-text">
                <span class="download-play-menu-title">{i18n.t('downloads.open_in_system')}</span>
                <span class="download-play-menu-desc">{i18n.t('downloads.open_in_system_desc')}</span>
              </div>
            </button>
            <button
              type="button"
              class="download-play-menu-item"
              use:ripple
              onclick={(e) => { e.stopPropagation(); close(); void showFileInFolder(); }}
            >
              <IconFolder class="download-play-menu-icon" />
              <div class="download-play-menu-text">
                <span class="download-play-menu-title">{i18n.t('downloads.show_in_folder')}</span>
                <span class="download-play-menu-desc">{i18n.t('downloads.show_in_folder_desc')}</span>
              </div>
            </button>
          </div>
        {/snippet}
      </PopoverMenu>
    </div>
  {/if}

  {#if previewUrl && !previewFailed && mediaKind === 'image'}
    <img class="grid-tile-media" src={previewUrl} alt="" loading="lazy" decoding="async" onerror={() => previewFailed = true} />
  {:else if previewUrl && !previewFailed && mediaKind === 'video'}
    <video class="grid-tile-media" src={previewUrl} muted playsinline preload="metadata" onerror={() => previewFailed = true}></video>
  {:else}
    <div class="grid-tile-placeholder download-placeholder">
      {#if mediaKind === 'audio'}<IconMusic />{:else}<IconDocument />{/if}
      {#if extension}<span>{extension}</span>{/if}
    </div>
  {/if}

  <div class="grid-tile-shade"></div>

  <div class="download-actions">
    {#if ['downloading', 'resolving', 'verifying', 'queued'].includes(item.status)}
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

  <div class="download-copy">
    {#if postTitle}<h2 class="grid-tile-title">{postTitle}</h2>{/if}
    <p class="download-filename" title={item.filename}>{item.filename}</p>
  </div>
  <div class="grid-tile-footer">
    {#if item.status !== 'completed'}
      <div class="grid-tile-author download-status-row" data-status={item.status}>
        {#if busy}<IconLoading />{:else if item.status === 'failed' || item.status === 'missing'}<IconError />{:else}<IconDownload />{/if}
        <span class="grid-tile-author-name">{i18n.t(`downloads.status_${item.status}`)}</span>
      </div>
    {/if}
    <div class="grid-tile-meta">
      <span>{item.total_bytes > 0 ? formatBytes(item.total_bytes) : formatBytes(item.downloaded_bytes)}</span>
      <div class="grid-tile-meta-stats">
        {#if item.status !== 'completed' && item.total_bytes > 0}<span>{percent}%</span>{/if}
        {#if item.speed_bps > 0}<span>{formatBytes(item.speed_bps)}/s</span>{/if}
      </div>
    </div>
  </div>

  {#if item.status !== 'completed'}
    <div class="download-progress" class:indeterminate={active && item.total_bytes === 0}><span style:width={`${item.total_bytes > 0 ? percent : 0}%`}></span></div>
  {/if}
</article>

<style>
  .download-placeholder { gap: calc(8px * var(--grid-scale, 1)); color: rgba(255,255,255,.28); }
  .download-placeholder :global(svg) { width: calc(34px * var(--grid-scale, 1)); height: calc(34px * var(--grid-scale, 1)); }
  .download-placeholder span { font-size: calc(9px * var(--grid-scale, 1)); font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
  .download-copy { position: absolute; z-index: 5; left: calc(12px * var(--grid-scale, 1)); right: calc(12px * var(--grid-scale, 1)); bottom: calc(52px * var(--grid-scale, 1)); display: flex; min-width: 0; flex-direction: column; align-items: flex-start; gap: calc(3px * var(--grid-scale, 1)); pointer-events: none; }
  .download-tile.completed .download-copy { bottom: calc(32px * var(--grid-scale, 1)); }
  .download-copy .grid-tile-title { position: static; width: 100%; overflow-wrap: anywhere; word-break: break-word; }
  .download-filename { width: 100%; margin: 0; color: rgba(255,255,255,.62); font-size: calc(10.5px * var(--grid-scale, 1)); font-weight: 500; line-height: 1.25; text-align: left; overflow-wrap: anywhere; word-break: break-word; display: -webkit-box; line-clamp: 2; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .download-actions-left { position: absolute; z-index: 6; top: calc(8px * var(--grid-scale, 1)); left: calc(8px * var(--grid-scale, 1)); display: flex; gap: calc(5px * var(--grid-scale, 1)); }
  .download-actions-left .download-action { position: relative; inset: auto; flex: none; }
  .download-actions { position: absolute; z-index: 6; top: calc(8px * var(--grid-scale, 1)); right: calc(8px * var(--grid-scale, 1)); display: flex; gap: calc(5px * var(--grid-scale, 1)); }
  .download-actions .download-action { position: relative; inset: auto; flex: none; }
  .download-status-row { pointer-events: none !important; }
  .download-status-row > :global(svg) { width: calc(15px * var(--grid-scale, 1)); height: calc(15px * var(--grid-scale, 1)); flex: none; color: rgba(255,255,255,.7); }
  .download-status-row .grid-tile-author-name { cursor: default; pointer-events: none; }
  .download-progress { position: absolute; z-index: 7; inset: auto 0 0; height: calc(3px * var(--grid-scale, 1)); overflow: hidden; background: rgba(255,255,255,.16); pointer-events: none; }
  .download-progress > span { display: block; height: 100%; background: var(--accent-primary); transition: width 220ms var(--ease-expo); }
  .download-progress.indeterminate::after { position: absolute; inset: 0; width: 35%; content: ''; background: var(--accent-primary); animation: indeterminate 1.2s ease-in-out infinite; }
  @keyframes indeterminate { from { transform: translateX(-110%); } to { transform: translateX(330%); } }
  @media (prefers-reduced-motion: reduce) { .download-progress.indeterminate::after { animation: none; width: 100%; opacity: .45; } }

  .download-play-menu {
    display: flex;
    flex-direction: column;
    gap: var(--floating-gap);
    padding: 0;
  }
  .download-play-menu-item {
    display: flex;
    align-items: center;
    gap: var(--floating-item-gap);
    width: 100%;
    padding: 6px var(--floating-item-px);
    border: none;
    border-radius: var(--floating-item-radius) !important;
    background: transparent;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    box-sizing: border-box;
    transition: background var(--duration-fast) var(--ease-expo);
  }
  .download-play-menu-item:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .download-play-menu-item:active {
    background: rgba(255, 255, 255, 0.1);
  }
  :global(.download-play-menu-icon) {
    width: var(--floating-item-icon-size) !important;
    height: var(--floating-item-icon-size) !important;
    flex-shrink: 0;
    color: var(--accent);
  }
  .download-play-menu-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .download-play-menu-title {
    font-size: var(--floating-card-title-size, 13px);
    font-weight: var(--floating-card-title-weight, 600);
    color: var(--text-primary);
    line-height: var(--floating-card-title-line-height, 1.25);
    white-space: nowrap;
  }
  .download-play-menu-desc {
    font-size: var(--floating-card-desc-size, 11px);
    font-weight: var(--floating-card-desc-weight, 400);
    color: var(--floating-card-desc-color, var(--text-muted));
    line-height: var(--floating-card-desc-line-height, 1.25);
    white-space: nowrap;
  }
</style>
