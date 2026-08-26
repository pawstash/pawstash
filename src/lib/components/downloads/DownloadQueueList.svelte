<script lang="ts">
  import { onMount } from 'svelte';
  import { downloadState, type DownloadFilter } from '$lib/state/downloadState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { i18n } from '$lib/i18n';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { apiGetAxumPort, apiOpenDownloadsFolder, apiSaveSettings } from '$lib/utils/ipc';
  import type { DownloadItem } from '$lib/types/download';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import PageHeader from '$lib/components/layout/PageHeader.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import CountBadge from '$lib/components/ui/CountBadge.svelte';
  import { ripple } from '$lib/motion';
  import DownloadItemCard from './DownloadItemCard.svelte';
  import DownloadGroupCard from './DownloadGroupCard.svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import SelectionActionBar from '$lib/components/ui/SelectionActionBar.svelte';
  import type { FilterMap } from '$lib/types/filter';
  import { countActiveFilters, matchesTriStateFilter, toggleFilterKey } from '$lib/types/filter';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconOptions from '~icons/fluent/options-24-regular';
  import IconGrid from '~icons/fluent/grid-24-regular';
  import IconStack from '~icons/fluent/stack-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconText from '~icons/fluent/document-text-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconDraft from '~icons/fluent/drafts-24-regular';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconFolderOpen from '~icons/fluent/folder-open-24-regular';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconPause from '~icons/fluent/pause-20-regular';
  import IconPlay from '~icons/fluent/play-20-regular';
  import IconRetry from '~icons/fluent/arrow-counterclockwise-20-regular';
  import IconDelete from '~icons/fluent/delete-20-regular';
  import { notify } from '$lib/utils/toast';

  interface DownloadIdentity {
    service: string;
    creatorId: string;
    postId: string;
    key: string;
  }

  interface DownloadGroup {
    key: string;
    identity?: DownloadIdentity;
    items: DownloadItem[];
  }

  type DownloadSort = 'newest' | 'oldest' | 'name_asc' | 'name_desc' | 'size_desc' | 'size_asc';
  const filters: DownloadFilter[] = ['all', 'active', 'completed'];
  const sortOptions: DownloadSort[] = ['newest', 'oldest', 'name_asc', 'name_desc', 'size_desc', 'size_asc'];

  const savedState = navigationState.getViewState<{
    searchQuery?: string;
    searchOpen?: boolean;
    sortBy?: DownloadSort;
    groupByPosts?: boolean;
    formatFilters?: FilterMap;
  }>(navigationState.entryKey);

  let mediaPort = $state<number | null>(null);
  let groupByPosts = $state(savedState?.groupByPosts ?? false);
  let sortBy = $state<DownloadSort>(savedState?.sortBy ?? 'newest');
  let formatFilters = $state<FilterMap>(savedState?.formatFilters ?? {});
  let searchQuery = $state(savedState?.searchQuery ?? '');
  let searchOpen = $state(savedState?.searchOpen ?? Boolean(savedState?.searchQuery));
  let filterOpen = $state(false);
  let stickyFilterOpen = $state(false);
  let scaleVisible = $state(false);
  let scaleTimer: ReturnType<typeof setTimeout> | undefined;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let scale = $derived(configState.settings.grid_scale / 100);
  let gap = $derived(Math.round((layoutState.isMobile ? 8 : 10) * scale));
  let targetCardWidth = $derived((layoutState.isMobile ? 155 : 245) * scale);

  let activeFilterCount = $derived((groupByPosts ? 1 : 0) + countActiveFilters([formatFilters]));
  let completedCount = $derived(downloadState.downloads.filter((item) => item.status === 'completed').length);
  let totalCount = $derived(downloadState.downloads.length);

  $effect(() => {
    navigationState.saveViewState(navigationState.entryKey, {
      searchQuery,
      searchOpen,
      sortBy,
      groupByPosts,
      formatFilters: $state.snapshot(formatFilters)
    });
  });

  $effect(() => {
    if (downloadState.filter === 'completed' && completedCount === totalCount) {
      downloadState.filter = 'all';
    }
  });

  function getDownloadFormats(item: DownloadItem): string[] {
    const filename = (item.filename || '').toLowerCase();
    const formats: string[] = [];
    if (/\.(jpe?g|png|gif|webp|bmp|avif)$/i.test(filename)) formats.push('image');
    if (/\.(mp4|mkv|webm|mov|avi|m4v)$/i.test(filename)) formats.push('video');
    if (/\.(mp3|wav|ogg|flac|m4a|aac)$/i.test(filename)) formats.push('audio');
    if (/\.(txt|md|pdf|doc|docx|epub)$/i.test(filename)) formats.push('text');
    if (/\.(zip|rar|7z|tar|gz)$/i.test(filename)) formats.push('archive');
    if (/\b(wip|sketch|sketches|rough|draft|preview|doodle|lineart)\b/i.test(filename) || /\.(psd|clip)$/i.test(filename)) formats.push('wip');
    if (formats.length === 0) formats.push('archive');
    return formats;
  }

  function toggleFormat(fmt: string) {
    formatFilters = toggleFilterKey(formatFilters, fmt);
  }

  function resetFilters() {
    formatFilters = {};
    groupByPosts = false;
  }

  const formatList = [
    { id: 'image', label: () => i18n.t('feed.format_photo') || 'Photo', icon: IconImage },
    { id: 'video', label: () => i18n.t('feed.format_video') || 'Video', icon: IconVideo },
    { id: 'audio', label: () => i18n.t('feed.format_audio') || 'Audio', icon: IconMusic },
    { id: 'text', label: () => i18n.t('feed.format_text') || 'Text', icon: IconText },
    { id: 'archive', label: () => i18n.t('feed.format_archive') || 'Files', icon: IconDocument },
    { id: 'wip', label: () => i18n.t('feed.format_wip') || 'WIP / Sketch', icon: IconDraft }
  ];

  let visibleDownloads = $derived.by(() => {
    const query = searchQuery.trim().toLocaleLowerCase();
    return downloadState.filteredDownloads.filter((item) => {
      if (Object.keys(formatFilters).length > 0 && !matchesTriStateFilter(getDownloadFormats(item), formatFilters)) {
        return false;
      }
      if (!query) return true;
      return [item.filename, item.post_title, item.creator_name, item.service, item.creator_id, item.post_id]
        .some((value) => value?.toLocaleLowerCase().includes(query));
    });
  });
  let sortedDownloads = $derived.by(() => [...visibleDownloads].sort((a, b) => {
    if (sortBy === 'oldest') return (a.created_at || '').localeCompare(b.created_at || '') || a.id.localeCompare(b.id);
    if (sortBy === 'name_asc') return (a.filename || '').localeCompare(b.filename || '');
    if (sortBy === 'name_desc') return (b.filename || '').localeCompare(a.filename || '');
    if (sortBy === 'size_desc') return (b.total_bytes || 0) - (a.total_bytes || 0);
    if (sortBy === 'size_asc') return (a.total_bytes || 0) - (b.total_bytes || 0);
    return (b.created_at || '').localeCompare(a.created_at || '') || b.id.localeCompare(a.id);
  }));

  let groupedDownloads = $derived.by(() => {
    const groups = new Map<string, DownloadGroup>();
    for (const item of sortedDownloads) {
      const identity = parseIdentity(item);
      const key = identity?.key || `file:${item.id}`;
      const current = groups.get(key);
      if (current) current.items.push(item);
      else groups.set(key, { key, identity, items: [item] });
    }
    return [...groups.values()];
  });

  function parseIdentity(item: DownloadItem): DownloadIdentity | undefined {
    if (!item.service || !item.creator_id || !item.post_id) return undefined;
    return { service: item.service, creatorId: item.creator_id, postId: item.post_id, key: `${item.service}:${item.creator_id}:${item.post_id}` };
  }

  function openPost(identity?: DownloadIdentity, initialMedia?: string, openViewer?: boolean) {
    if (identity) navigationState.openPost(identity.service, identity.creatorId, identity.postId, initialMedia, openViewer);
  }

  function previewItem(items: DownloadItem[]) {
    return items.find((item) => /\.(avif|bmp|gif|jpe?g|png|webp|m4v|mkv|mov|mp4|webm)(?:\?.*)?$/i.test(item.filename)) || items[0];
  }



  function previewUrl(item?: DownloadItem) {
    if (!item) return undefined;
    if (item.status === 'completed' && item.final_path) {
      if (mediaPort) {
        const path = item.final_path.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
        return `http://127.0.0.1:${mediaPort}/media/${path}`;
      }
      return convertFileSrc(item.final_path);
    }
    return item.url;
  }

  function localPathUrl(path?: string) {
    if (!path) return undefined;
    if (mediaPort) {
      const encoded = path.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
      return `http://127.0.0.1:${mediaPort}/media/${encoded}`;
    }
    return convertFileSrc(path);
  }

  $effect(() => {
    if (downloadState.filter === 'active' && downloadState.downloads.length > 0 && downloadState.activeDownloadsCount === 0) {
      downloadState.filter = 'all';
    }
  });

  $effect(() => {
    if (!searchOpen) return;
    const input = document.querySelector('.downloads-search-input') as HTMLInputElement | null;
    input?.focus();
  });

  onMount(() => {
    void downloadState.init().catch((error) => downloadState.error = String(error));
    void libraryState.init();
    void apiGetAxumPort().then((port) => mediaPort = port).catch(() => mediaPort = null);

    return () => {
      if (scaleTimer) clearTimeout(scaleTimer);
      if (saveTimer) clearTimeout(saveTimer);
    };
  });



  function closeSearch() {
    searchQuery = '';
    searchOpen = false;
  }

  async function openDownloadsFolder() {
    try {
      await apiOpenDownloadsFolder();
    } catch (error) {
      notify.error(i18n.t('downloads.open_folder_failed'), error);
    }
  }

  function setScale(next: number) {
    configState.settings.grid_scale = Math.max(60, Math.min(160, Math.round(next / 5) * 5));
    scaleVisible = true;
    if (scaleTimer) clearTimeout(scaleTimer);
    if (saveTimer) clearTimeout(saveTimer);
    scaleTimer = setTimeout(() => scaleVisible = false, 900);
    saveTimer = setTimeout(() => void apiSaveSettings(configState.settings), 300);
  }

  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'downloads');
  let downloadKeys = $derived(sortedDownloads.map((d) => d.id));
  let downloadsMap = $derived(new Map(sortedDownloads.map((d) => [d.id, d])));

  $effect(() => {
    selectionState.setContext('downloads', downloadKeys, downloadsMap);
  });

  function handleSelectAll() {
    selectionState.selectAll(sortedDownloads.map((d) => ({ key: d.id, item: d })));
  }

  async function batchPause() {
    const items = selectionState.getItems<DownloadItem>();
    if (items.length === 0) return;
    try {
      for (const item of items) {
        if (['queued', 'resolving', 'downloading'].includes(item.status)) {
          await downloadState.pause(item.id);
        }
      }
      notify.success(
        i18n.t('downloads.pause') || 'Paused',
        `${items.length} ${items.length === 1 ? 'download' : 'downloads'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('downloads.action_error') || 'Failed to pause downloads', err);
    }
  }

  async function batchResume() {
    const items = selectionState.getItems<DownloadItem>();
    if (items.length === 0) return;
    try {
      for (const item of items) {
        if (item.status === 'paused') {
          await downloadState.resume(item.id);
        }
      }
      notify.success(
        i18n.t('downloads.resume') || 'Resumed',
        `${items.length} ${items.length === 1 ? 'download' : 'downloads'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('downloads.action_error') || 'Failed to resume downloads', err);
    }
  }

  async function batchRetry() {
    const items = selectionState.getItems<DownloadItem>();
    if (items.length === 0) return;
    try {
      for (const item of items) {
        if (['failed', 'cancelled', 'missing'].includes(item.status)) {
          await downloadState.retry(item.id);
        }
      }
      notify.success(
        i18n.t('downloads.retry') || 'Retrying',
        `${items.length} ${items.length === 1 ? 'download' : 'downloads'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('downloads.action_error') || 'Failed to retry downloads', err);
    }
  }

  async function batchRemove() {
    const items = selectionState.getItems<DownloadItem>();
    if (items.length === 0) return;
    try {
      for (const item of items) {
        await downloadState.remove(item.id);
      }
      notify.success(
        i18n.t('downloads.remove') || 'Removed',
        `${items.length} ${items.length === 1 ? 'download' : 'downloads'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('downloads.action_error') || 'Failed to remove downloads', err);
    }
  }

  function handleGridWheel(event: WheelEvent) {
    if (!event.ctrlKey) return;
    event.preventDefault();
    setScale(configState.settings.grid_scale + (event.deltaY < 0 ? 5 : -5));
  }

  function handleGridKeydown(event: KeyboardEvent) {
    if (!event.ctrlKey || event.key !== '0') return;
    event.preventDefault();
    setScale(100);
  }
</script>

<svelte:window onkeydown={handleGridKeydown} />

{#snippet downloadTabs()}
  <nav class="downloads-tabs" aria-label={i18n.t('downloads.title')}>
    {#each filters as filter}
      {#if (filter !== 'active' || downloadState.activeDownloadsCount > 0) && (filter !== 'completed' || completedCount < totalCount)}
        <Button variant={downloadState.filter === filter ? 'accent' : 'ghost'} onclick={() => { downloadState.filter = filter; if (selectionState.active) selectionState.clear(); }} class="downloads-tab">
          <span>{i18n.t(`downloads.${filter}`)}</span>
          {#if filter === 'active'}
            <CountBadge count={downloadState.activeDownloadsCount} />
          {:else if filter === 'completed'}
            <CountBadge count={completedCount} />
          {:else if filter === 'all'}
            <CountBadge count={totalCount} />
          {/if}
        </Button>
      {/if}
    {/each}
  </nav>
{/snippet}

{#snippet downloadSort()}
  <Select
    variant="ghost"
    options={sortOptions.map((option) => ({ value: option, label: i18n.t(`downloads.sort_${option}`) }))}
    value={sortBy}
    onchange={(value) => sortBy = value as DownloadSort}
    class="downloads-sort-select"
    style="height: 44px;"
  />
{/snippet}

{#snippet downloadFilterInnerContent()}
  <button
    type="button"
    class="view-option"
    class:active={groupByPosts}
    use:ripple
    onclick={() => groupByPosts = !groupByPosts}
  >
    <Checkbox checked={groupByPosts} onchange={(checked) => groupByPosts = checked} />
    <span>
      <strong>{i18n.t('downloads.group_by_posts')}</strong>
      <small>{i18n.t('downloads.group_by_posts_desc')}</small>
    </span>
    <IconStack class="view-option-icon w-[20px] h-[20px]" />
  </button>

  <div class="floating-divider"></div>

  <span class="filter-label section-label">{i18n.t('feed.format')}</span>
  <div class="service-options">
    {#each formatList as fmt}
      {@const state = formatFilters[fmt.id] ?? 'neutral'}
      {@const IconComponent = fmt.icon}
      <Button
        variant="ghost"
        size="sm"
        onclick={() => toggleFormat(fmt.id)}
        class="filter-chip {state === 'include' ? 'state-include' : state === 'exclude' ? 'state-exclude' : ''}"
      >
        <IconComponent class="w-5 h-5" />
        <span>{fmt.label()}</span>
        {#if state === 'include'}
          <IconSearch class="w-3.5 h-3.5 ml-auto text-[#4ade80] shrink-0" />
        {:else if state === 'exclude'}
          <IconDismiss class="w-3.5 h-3.5 ml-auto text-[#f87171] shrink-0" />
        {/if}
      </Button>
    {/each}
  </div>
{/snippet}

{#snippet filterControl(source: 'main' | 'sticky')}
  {#if source === 'sticky'}
    <PopoverMenu
      bind:open={stickyFilterOpen}
      title={i18n.t('downloads.filters')}
      badge={activeFilterCount}
      active={activeFilterCount > 0}
      icon={IconOptions}
    >
      {@render downloadFilterInnerContent()}
    </PopoverMenu>
  {:else}
    <PopoverMenu
      bind:open={filterOpen}
      title={i18n.t('downloads.filters')}
      badge={activeFilterCount}
      active={activeFilterCount > 0}
      icon={IconOptions}
    >
      {@render downloadFilterInnerContent()}
    </PopoverMenu>
  {/if}
{/snippet}

{#snippet actionsCluster(source: 'main' | 'sticky')}
  <HeaderActions
    bind:searchOpen
    bind:searchQuery
    searchPlaceholder={i18n.t('downloads.search_placeholder')}
  >
    <Button
      variant={isSelectionActive ? 'accent' : 'ghost'}
      class="btn-icon"
      onclick={() => (isSelectionActive ? selectionState.exit() : selectionState.enter('downloads'))}
      title={i18n.t('selection.select_mode') || 'Select mode'}
      aria-label="Select mode"
    >
      <IconCheckboxChecked class="w-5 h-5" />
    </Button>
    <Button variant="ghost" class="btn-icon" onclick={openDownloadsFolder} title={i18n.t('downloads.open_folder')} aria-label={i18n.t('downloads.open_folder')}>
      <IconFolderOpen class="w-5 h-5" />
    </Button>
    {@render filterControl(source)}
  </HeaderActions>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey} onrefresh={() => downloadState.refresh()}>
  {#snippet overlay()}
    <StickyHeader threshold={120} title={i18n.t('downloads.title') || 'Downloads'}>
      {#snippet center()}
        <div class="flex items-center gap-2">
          {@render downloadTabs()}
          {@render downloadSort()}
        </div>
      {/snippet}
      {#snippet trailing()}
        {@render actionsCluster('sticky')}
      {/snippet}
    </StickyHeader>
  {/snippet}

  <PageHeader>
    {#snippet tabs()}
      <div class="flex items-center gap-2">
        {@render downloadTabs()}
        {@render downloadSort()}
      </div>
    {/snippet}
    {#snippet actions()}
      {@render actionsCluster('main')}
    {/snippet}
  </PageHeader>

  {#if sortedDownloads.length > 0}
    {#if scaleVisible}<div class="scale-indicator">{configState.settings.grid_scale}%</div>{/if}
    <div class="downloads-grid" onwheel={handleGridWheel} style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-gap: ${gap}px;`}>
      {#if groupByPosts}
        {#each groupedDownloads as group (group.key)}
          {@const media = previewItem(group.items)}
          <DownloadGroupCard items={group.items} previewUrl={localPathUrl(media.post_preview_path) || media.post_preview_url || previewUrl(media)} avatarUrl={localPathUrl(media.creator_avatar_path)} title={media.post_title || i18n.t('downloads.unknown_post')} creatorName={media.creator_name} onopen={group.identity ? () => openPost(group.identity) : undefined} oncreator={group.identity ? () => navigationState.openCreator(group.identity!.service, group.identity!.creatorId) : undefined} />
        {/each}
      {:else}
        {#each sortedDownloads as item (item.id)}
          {@const identity = parseIdentity(item)}
          <DownloadItemCard
            {item}
            previewUrl={previewUrl(item)}
            thumbnailUrl={localPathUrl(item.post_preview_path) || item.post_preview_url}
            postTitle={item.post_title}
            onopen={identity ? (openViewer) => openPost(identity, item.media_id || item.filename || item.url, openViewer) : undefined}
            orderedKeys={downloadKeys}
            itemsMap={downloadsMap}
          />
        {/each}
      {/if}
    </div>
  {:else if !downloadState.loading}
    <div class="empty-state">
      <IconDownload class="w-[36px] h-[36px] mb-2" />
      <strong>{i18n.t('downloads.empty')}</strong>
      <span>{i18n.t('downloads.empty_desc')}</span>
    </div>
  {/if}

  {#if downloadState.error}<p class="page-error">{downloadState.error}</p>{/if}
</PageShell>

<SelectionActionBar
  totalCount={sortedDownloads.length}
  onSelectAll={handleSelectAll}
>
  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={batchPause}
    title={i18n.t('downloads.pause')}
  >
    <IconPause class="w-[16px] h-[16px]" />
    <span>{i18n.t('downloads.pause')}</span>
  </Button>

  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={batchResume}
    title={i18n.t('downloads.resume')}
  >
    <IconPlay class="w-[16px] h-[16px]" />
    <span>{i18n.t('downloads.resume')}</span>
  </Button>

  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={batchRetry}
    title={i18n.t('downloads.retry')}
  >
    <IconRetry class="w-[16px] h-[16px]" />
    <span>{i18n.t('downloads.retry')}</span>
  </Button>

  <Button
    variant="danger"
    size="sm"
    class="selection-btn"
    onclick={batchRemove}
    title={i18n.t('downloads.remove')}
  >
    <IconDelete class="w-[16px] h-[16px]" />
    <span>{i18n.t('downloads.remove')}</span>
  </Button>
</SelectionActionBar>

<style>
  .downloads-tabs { display: flex; align-items: center; gap: 8px; min-width: 0; overflow-x: auto; scrollbar-width: none; }
  .downloads-tabs::-webkit-scrollbar { display: none; }

  :global(.select-root.downloads-sort-select) { height: 44px !important; width: auto !important; min-width: 170px !important; max-width: none !important; flex: none !important; }
  :global(.select-root.downloads-sort-select .select-trigger.variant-ghost) { height: 44px !important; width: 100% !important; padding: 0 14px !important; border-radius: var(--radius-full) !important; font-size: 13px !important; }
  .downloads-grid { position: relative; display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--grid-card-width)), 1fr)); align-items: start; gap: var(--grid-gap); width: 100%; }
  .scale-indicator { position: fixed; z-index: 80; left: 50%; bottom: 34px; transform: translateX(-50%); padding: 7px 12px; border: 1px solid rgba(255,255,255,.14); border-radius: 999px; background: rgba(10,10,14,.82); color: white; font-size: 12px; font-weight: 650; backdrop-filter: blur(14px); pointer-events: none; }
  .empty-state { min-height: 310px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 6px; color: rgba(255, 255, 255, 0.42); text-align: center; }
  .empty-state :global(svg) { width: 34px; height: 34px; color: rgba(255, 255, 255, 0.42); margin-bottom: 5px; }
  .empty-state strong { color: rgba(255, 255, 255, 0.76); font-size: 14px; font-weight: 600; }
  .empty-state span { max-width: 360px; font-size: 12px; color: rgba(255, 255, 255, 0.42); line-height: 1.5; }
  .page-error { margin-top: 16px; color: rgba(255,130,130,.8); font-size: 12px; text-align: center; }
</style>
