<script lang="ts">
  import { paintSelect } from '$lib/actions/paintSelect';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import { onMount, onDestroy } from 'svelte';
  import { countDownloadPosts, downloadPostKey, downloadState, type DownloadFilter } from '$lib/state/downloadState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { resolveLocalMediaUrl } from '$lib/utils/media';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { i18n } from '$lib/i18n';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { apiOpenDownloadsFolder, apiShowInFolder } from '$lib/utils/ipc';
  import { gridScaleWheel } from '$lib/state/scaleState.svelte';
  import { serverPortState } from '$lib/state/serverPort.svelte';
  import type { DownloadItem } from '$lib/types/download';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import ChoiceGroup, { type ChoiceOption } from '$lib/components/ui/ChoiceGroup.svelte';
  import SortControl from '$lib/components/layout/SortControl.svelte';
  import { sortOption } from '$lib/utils/sort';
  import type { MenuItem } from '$lib/components/ui/MenuList.svelte';
  import { ripple } from '$lib/motion';
  import DownloadItemCard from './DownloadItemCard.svelte';
  import DownloadGroupCard from './DownloadGroupCard.svelte';
  import DownloadQueueTile from './DownloadQueueTile.svelte';
  import SkeletonGrid from '$lib/components/ui/SkeletonGrid.svelte';
  import { GridHandoff } from '$lib/motion/gridHandoff.svelte';
  import MediaViewer, { type MediaViewerItem, type MediaViewerKind } from '$lib/components/content/MediaViewer.svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import type { FilterMap } from '$lib/types/filter';
  import { countActiveFilters, matchesTriStateFilter, toggleFilterKey } from '$lib/types/filter';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconOptions from '~icons/fluent/options-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconText from '~icons/fluent/document-text-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconDraft from '~icons/fluent/drafts-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconFolderOpen from '~icons/fluent/folder-open-24-regular';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconPause from '~icons/fluent/pause-20-regular';
  import IconPlay from '~icons/fluent/play-20-regular';
  import IconRetry from '~icons/fluent/arrow-counterclockwise-20-regular';
  import IconDelete from '~icons/fluent/delete-20-regular';
  import { notify } from '$lib/utils/toast';
  import { ctrlWheel } from '$lib/utils/ctrlWheel';

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
  const sortOptions = $derived([
    sortOption<DownloadSort>('newest', 'added', 'desc'),
    sortOption<DownloadSort>('oldest', 'added', 'asc'),
    sortOption<DownloadSort>('name_asc', 'name', 'asc'),
    sortOption<DownloadSort>('name_desc', 'name', 'desc'),
    sortOption<DownloadSort>('size_desc', 'size', 'desc'),
    sortOption<DownloadSort>('size_asc', 'size', 'asc')
  ]);

  const savedState = navigationState.getViewState<{
    searchQuery?: string;
    searchOpen?: boolean;
    sortBy?: DownloadSort;
    groupByPosts?: boolean;
    formatFilters?: FilterMap;
  }>(navigationState.entryKey);

  const GROUP_BY_POSTS_KEY = 'pawstash_downloads_group_by_posts';
  const storedGroupByPosts = typeof localStorage !== 'undefined' && localStorage.getItem(GROUP_BY_POSTS_KEY) === 'true';
  let groupByPosts = $state(savedState?.groupByPosts ?? storedGroupByPosts);
  let sortBy = $state<DownloadSort>(savedState?.sortBy ?? 'newest');
  let formatFilters = $state<FilterMap>(savedState?.formatFilters ?? {});
  let searchQuery = $state(savedState?.searchQuery ?? '');
  let searchOpen = $state(savedState?.searchOpen ?? Boolean(savedState?.searchQuery));
  let filterOpen = $state(false);
  let scale = $derived(configState.settings.grid_scale / 100);
  let gap = $derived(Math.round((layoutState.isMobile ? 8 : 10) * scale));
  let targetCardWidth = $derived((layoutState.isMobile ? 155 : 245) * scale);

  let activeFilterCount = $derived((groupByPosts ? 1 : 0) + countActiveFilters([formatFilters]));
  const isCompleted = (item: DownloadItem) => item.status === 'completed';
  let completedCount = $derived(downloadState.downloads.filter(isCompleted).length);
  let completedPostsCount = $derived(countDownloadPosts(downloadState.downloads, isCompleted));
  let totalCount = $derived(downloadState.downloads.length);
  let totalPostsCount = $derived(countDownloadPosts(downloadState.downloads));

  function tabOption(value: DownloadFilter, label: string, posts: number, files: number): ChoiceOption<DownloadFilter> {
    return { value, label, count: posts, title: i18n.t('downloads.tab_count_title', { posts, files }) };
  }

  let downloadTabOptions = $derived<ChoiceOption<DownloadFilter>[]>([
    tabOption('all', i18n.t('downloads.all'), totalPostsCount, totalCount),
    ...(downloadState.activeDownloadsCount > 0
      ? [tabOption('active', i18n.t('downloads.active'), downloadState.activePostsCount, downloadState.activeDownloadsCount)]
      : []),
    ...(completedCount < totalCount
      ? [tabOption('completed', i18n.t('downloads.completed'), completedPostsCount, completedCount)]
      : []),
    ...(downloadState.failedDownloadsCount > 0
      ? [tabOption('failed', i18n.t('downloads.failed'), downloadState.failedPostsCount, downloadState.failedDownloadsCount)]
      : [])
  ]);

  let anyRunning = $derived(downloadState.queueStats.downloading + downloadState.queueStats.queued > 0);
  let anyPaused = $derived(downloadState.queueStats.paused > 0);
  let showQueueTile = $derived(
    downloadState.filter !== 'completed' &&
      (downloadState.activeDownloadsCount > 0 || downloadState.failedDownloadsCount > 0)
  );
  let bulkBusy = $state(false);

  async function runBulk(action: () => Promise<void>) {
    if (bulkBusy) return;
    bulkBusy = true;
    try {
      await action();
    } catch (err) {
      notify.error(i18n.t('downloads.action_error'), err);
    } finally {
      bulkBusy = false;
    }
  }

  const toggleAllDownloads = () =>
    runBulk(() => (anyRunning ? downloadState.pauseAll() : downloadState.resumeAll()));
  const retryAllFailed = () => runBulk(() => downloadState.retryFailed());

  $effect(() => {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(GROUP_BY_POSTS_KEY, String(groupByPosts));
    }
  });

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

  $effect(() => {
    if (downloadState.filter === 'failed' && downloadState.failedDownloadsCount === 0) {
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
    { id: 'image', label: () => i18n.t('feed.format_photo'), icon: IconImage },
    { id: 'video', label: () => i18n.t('feed.format_video'), icon: IconVideo },
    { id: 'audio', label: () => i18n.t('feed.format_audio'), icon: IconMusic },
    { id: 'text', label: () => i18n.t('feed.format_text'), icon: IconText },
    { id: 'archive', label: () => i18n.t('feed.format_archive'), icon: IconDocument },
    { id: 'wip', label: () => i18n.t('feed.format_wip'), icon: IconDraft }
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
  const handoff = new GridHandoff();
  $effect(() => handoff.sync(sortedDownloads.length > 0, downloadState.loading));
  onDestroy(() => handoff.destroy());

  let groupedDownloads = $derived.by(() => {
    const groups = new Map<string, DownloadGroup>();
    for (const item of sortedDownloads) {
      const identity = parseIdentity(item);
      const key = downloadPostKey(item);
      const current = groups.get(key);
      if (current) current.items.push(item);
      else groups.set(key, { key, identity, items: [item] });
    }
    return [...groups.values()];
  });

  function parseIdentity(item: DownloadItem): DownloadIdentity | undefined {
    if (!item.service || !item.creator_id || !item.post_id) return undefined;
    return { service: item.service, creatorId: item.creator_id, postId: item.post_id, key: downloadPostKey(item) };
  }

  let viewerIndex = $state<number | null>(null);

  function mediaViewerKindFromFilename(filename: string): MediaViewerKind {
    const ext = filename.includes('.') ? (filename.split('.').pop() || '').toLowerCase() : '';
    if (/^(avif|bmp|gif|jpe?g|png|webp)$/.test(ext)) return 'image';
    if (/^(m4v|mkv|mov|mp4|webm)$/.test(ext)) return 'video';
    if (/^(aac|flac|m4a|mp3|ogg|opus|wav)$/.test(ext)) return 'audio';
    return 'file';
  }

  let viewerDownloads = $derived.by(() =>
    sortedDownloads.filter((item) => {
      const kind = mediaViewerKindFromFilename(item.filename);
      return kind === 'image' || kind === 'video' || kind === 'audio';
    })
  );

  let viewerItems = $derived.by((): MediaViewerItem[] =>
    viewerDownloads.map((item) => ({
      id: item.media_id || item.id || item.filename,
      name: item.filename,
      kind: mediaViewerKindFromFilename(item.filename),
      url: previewUrl(item) || '',
      poster: itemThumbnailUrl(item),
      size: item.total_bytes || item.downloaded_bytes,
      downloadStatus: item.status,
      downloadedBytes: item.downloaded_bytes,
      totalBytes: item.total_bytes,
      downloadedPath: item.final_path
    }))
  );

  function openPost(identity?: DownloadIdentity) {
    if (identity) navigationState.openPost(identity.service, identity.creatorId, identity.postId);
  }

  function handleItemOpen(item: DownloadItem, openInPost?: boolean) {
    const identity = parseIdentity(item);

    if (openInPost) {
      openPost(identity);
      return;
    }

    const kind = mediaViewerKindFromFilename(item.filename);
    const isMedia = kind === 'image' || kind === 'video' || kind === 'audio';

    if (isMedia) {
      const targetIdx = viewerDownloads.findIndex((d) => d.id === item.id);
      if (targetIdx >= 0) {
        viewerIndex = targetIdx;
        return;
      }
    }

    openPost(identity);
  }

  function previewItem(items: DownloadItem[]) {
    return items.find((item) => /\.(avif|bmp|gif|jpe?g|png|webp|m4v|mkv|mov|mp4|webm)(?:\?.*)?$/i.test(item.filename)) || items[0];
  }

  function previewUrl(item?: DownloadItem) {
    if (!item) return undefined;
    const port = serverPortState.port || 0;
    if (item.status === 'completed' && item.final_path) {
      if (port > 0) {
        const path = item.final_path.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
        return serverPortState.mediaUrl(`/media/${path}`);
      }
      return convertFileSrc(item.final_path);
    }
    return item.url;
  }

  function itemThumbnailUrl(item: DownloadItem): string | undefined {
    return resolveLocalMediaUrl(item.file_preview_path) || item.file_preview_url || undefined;
  }

  function groupThumbnailUrl(items: DownloadItem[]): string | undefined {
    const withPostThumb = items.find((i) => i.post_preview_path);
    if (withPostThumb?.post_preview_path) return resolveLocalMediaUrl(withPostThumb.post_preview_path);
    const withPostUrl = items.find((i) => i.post_preview_url);
    if (withPostUrl?.post_preview_url) return withPostUrl.post_preview_url;
    const withFileThumb = items.find((i) => i.file_preview_path);
    if (withFileThumb?.file_preview_path) return resolveLocalMediaUrl(withFileThumb.file_preview_path);
    const withFileUrl = items.find((i) => i.file_preview_url);
    if (withFileUrl?.file_preview_url) return withFileUrl.file_preview_url;
    return undefined;
  }

  $effect(() => {
    if (downloadState.filter === 'active' && downloadState.downloads.length > 0 && downloadState.activeDownloadsCount === 0) {
      downloadState.filter = 'all';
    }
  });

  onMount(() => {
    void downloadState.init().then(() => downloadState.refresh()).catch((error) => downloadState.error = String(error));
    void libraryState.init();
    void serverPortState.ensurePort();
  });

  async function openDownloadsFolder() {
    try {
      await apiOpenDownloadsFolder();
    } catch (error) {
      notify.error(i18n.t('downloads.open_folder_failed'), error);
    }
  }


  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'downloads');

  let pageMenu = $derived<MenuItem[]>([
    ...(anyRunning || anyPaused
      ? [
          {
            group: 'queue',
            label: i18n.t(anyRunning ? 'downloads.pause_all' : 'downloads.resume_all'),
            icon: anyRunning ? IconPause : IconPlay,
            disabled: bulkBusy,
            onclick: () => void toggleAllDownloads()
          }
        ]
      : []),
    ...(downloadState.failedDownloadsCount > 0
      ? [{ group: 'queue', label: i18n.t('downloads.retry_failed'), icon: IconRetry, disabled: bulkBusy, onclick: () => void retryAllFailed() }]
      : []),
    {
      group: 'page',
      label: i18n.t(isSelectionActive ? 'selection.exit' : 'selection.select_mode'),
      icon: IconCheckboxChecked,
      active: isSelectionActive,
      onclick: () => (isSelectionActive ? selectionState.exit() : selectionState.enter('downloads'))
    },
    { group: 'page', label: i18n.t('downloads.open_folder'), icon: IconFolderOpen, onclick: () => void openDownloadsFolder() },
    {
      group: 'page',
      label: i18n.t('common.refresh'),
      icon: IconArrowClockwise,
      disabled: downloadState.loading,
      onclick: () => void downloadState.refresh()
    }
  ]);
  let downloadKeys = $derived(sortedDownloads.map((d) => d.id));
  let downloadsMap = $derived(new Map(sortedDownloads.map((d) => [d.id, d])));
  let selectedDownloads = $derived(isSelectionActive ? selectionState.getItems<DownloadItem>() : []);
  let batchTargets = $derived(selectedDownloads.length > 0 ? selectedDownloads : sortedDownloads);

  const isRunning = (item: DownloadItem) => ['queued', 'resolving', 'downloading', 'verifying'].includes(item.status);
  const isPaused = (item: DownloadItem) => item.status === 'paused';
  const isRetryable = (item: DownloadItem) => ['failed', 'cancelled', 'missing'].includes(item.status);
  const isOnDisk = (item: DownloadItem) => isCompleted(item) && Boolean(item.final_path);

  let hasPausable = $derived(batchTargets.some(isRunning));
  let hasResumable = $derived(batchTargets.some(isPaused));
  let hasRetryable = $derived(batchTargets.some(isRetryable));
  let hasCompleted = $derived(batchTargets.some(isOnDisk));
  let allActiveSelected = $derived(selectedDownloads.length > 0 && selectedDownloads.every(isRunning));

  $effect(() => {
    selectionState.setContext('downloads', downloadKeys, downloadsMap);
  });

  function handleSelectAll() {
    selectionState.selectAll(sortedDownloads.map((d) => ({ key: d.id, item: d })));
  }

  async function runBatch(match: (item: DownloadItem) => boolean, operation: (id: string) => Promise<void>, title: string) {
    const targets = batchTargets.filter(match);
    if (targets.length === 0) return;
    try {
      for (const item of targets) {
        await operation(item.id);
      }
      notify.success(title, i18n.t('counts.downloads', { count: targets.length }));
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('downloads.action_error'), err);
    }
  }

  const batchPause = () => runBatch(isRunning, (id) => downloadState.pause(id), i18n.t('downloads.pause'));
  const batchResume = () => runBatch(isPaused, (id) => downloadState.resume(id), i18n.t('downloads.resume'));
  const batchRetry = () => runBatch(isRetryable, (id) => downloadState.retry(id), i18n.t('downloads.retry'));
  const batchRemove = () =>
    runBatch(() => true, (id) => downloadState.remove(id), i18n.t(allActiveSelected ? 'downloads.cancel' : 'downloads.remove'));

  async function batchShowInFolder() {
    const completed = batchTargets.filter(isOnDisk);
    if (completed.length === 0) return;
    try {
      const paths = Array.from(new Set(completed.map((d) => d.final_path)));
      for (const p of paths.slice(0, 3)) {
        await apiShowInFolder(p);
      }
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('downloads.show_in_folder_failed'), err);
    }
  }


</script>


{#snippet downloadTabs()}
  <ChoiceGroup
    options={downloadTabOptions}
    value={downloadState.filter}
    onchange={(filter) => {
      downloadState.filter = filter as DownloadFilter;
      if (selectionState.active) selectionState.clear();
    }}
    align="left"
    class="downloads-mode-choice"
  >
    {#snippet addon()}{@render sortControl('addon')}{/snippet}
  </ChoiceGroup>
{/snippet}

{#snippet sortControl(variant: 'addon' | 'standalone')}
  <SortControl
    {variant}
    options={sortOptions}
    value={sortBy}
    onchange={(value) => (sortBy = value)}
  />
{/snippet}

{#snippet downloadFilterInnerContent()}
  <div class="filter-section-title">
    <div class="filter-section-icon"><IconOptions class="w-3.5 h-3.5" /></div>
    <span>{i18n.t('feed.filters')}</span>
  </div>
  <div class="settings-list">
    <div
      role="switch"
      aria-checked={groupByPosts}
      tabindex="0"
      class="view-option"
      class:active={groupByPosts}
      use:ripple
      onclick={() => (groupByPosts = !groupByPosts)}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          groupByPosts = !groupByPosts;
        }
      }}
    >
      <span>
        <strong>{i18n.t('downloads.group_by_posts')}</strong>
        <small>{i18n.t('downloads.group_by_posts_desc')}</small>
      </span>
      <Toggle
        checked={groupByPosts}
        tabindex={-1}
        class="pointer-events-none"
      />
    </div>
  </div>

  <div class="filter-section-title">
    <div class="filter-section-icon"><IconDocument class="w-3.5 h-3.5" /></div>
    <span>{i18n.t('feed.format')}</span>
  </div>
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
      </Button>
    {/each}
  </div>
{/snippet}

{#snippet filterControl()}
  <PopoverMenu
    bind:open={filterOpen}
    title={i18n.t('downloads.filters')}
    badge={activeFilterCount}
    active={activeFilterCount > 0}
    icon={IconOptions}
  >
    {@render downloadFilterInnerContent()}
  </PopoverMenu>
{/snippet}

{#snippet search()}
  <HeaderActions
    pinOpen bind:searchOpen bind:searchQuery searchPlaceholder={i18n.t('downloads.search_placeholder')} />
{/snippet}

<PageShell title={i18n.t('nav.downloads')} menu={pageMenu} scrollKey={navigationState.entryKey} onrefresh={() => downloadState.refresh()}
  selectionTotal={sortedDownloads.length}
  onSelectAll={handleSelectAll}
  {selectionActions}
  views={downloadTabOptions.length > 1 ? downloadTabs : undefined}
>
  {#snippet filter()}{@render filterControl()}{/snippet}
  {#snippet actions()}
    {#if downloadTabOptions.length <= 1}{@render sortControl('standalone')}{/if}
    {@render search()}
  {/snippet}

  {#if sortedDownloads.length > 0 || handoff.skeletonMounted}
    <div class="grid-stack">
      <div class="downloads-grid" use:paintSelect={{ scope: 'downloads', keys: downloadKeys, items: downloadsMap }} {@attach ctrlWheel(gridScaleWheel)} style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-gap: ${gap}px;`}>
        {#if showQueueTile}
          <DownloadQueueTile onshowfailed={() => (downloadState.filter = 'failed')} />
        {/if}
        {#if groupByPosts}
          {#each groupedDownloads as group (group.key)}
            {@const media = previewItem(group.items)}
            <DownloadGroupCard
              items={group.items}
              previewUrl={previewUrl(media)}
              thumbnailUrl={groupThumbnailUrl(group.items)}
              avatarUrl={resolveLocalMediaUrl(media.creator_avatar_path)}
              title={media.post_title || i18n.t('downloads.unknown_post')}
              creatorName={media.creator_name}
              onopen={group.identity ? () => openPost(group.identity) : undefined}
              oncreator={group.identity ? () => navigationState.openCreator(group.identity!.service, group.identity!.creatorId) : undefined}
            />
          {/each}
        {:else}
          {#each sortedDownloads as item (item.id)}
            <DownloadItemCard
              {item}
              previewUrl={previewUrl(item)}
              thumbnailUrl={itemThumbnailUrl(item)}
              postTitle={item.post_title}
              onopen={(openInPost) => handleItemOpen(item, openInPost)}
              orderedKeys={downloadKeys}
              itemsMap={downloadsMap}
            />
          {/each}
        {/if}
      </div>

      {#if handoff.skeletonMounted}
        <SkeletonGrid
          cardWidth={targetCardWidth}
          {gap}
          fading={handoff.skeletonFading}
        />
      {/if}
    </div>
  {:else if !downloadState.loading}
    <EmptyState icon={IconDownload} title={i18n.t('downloads.empty')} description={i18n.t('downloads.empty_desc')} />
  {/if}

  {#if downloadState.error}<p class="page-error">{downloadState.error}</p>{/if}
</PageShell>

{#snippet selectionActions()}
  {#if hasPausable}
    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchPause}
      title={i18n.t('downloads.pause')}
    >
      <IconPause />
    </Button>
  {/if}

  {#if hasResumable}
    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchResume}
      title={i18n.t('downloads.resume')}
    >
      <IconPlay />
    </Button>
  {/if}

  {#if hasRetryable}
    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchRetry}
      title={i18n.t('downloads.retry')}
    >
      <IconRetry />
    </Button>
  {/if}

  {#if hasCompleted}
    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchShowInFolder}
      title={i18n.t('downloads.show_in_folder')}
    >
      <IconFolderOpen />
    </Button>
  {/if}

  <Button
    variant="ghost"
    class="btn-icon"
    onclick={batchRemove}
    title={allActiveSelected ? i18n.t('downloads.cancel') : i18n.t('downloads.remove')}
  >
    {#if allActiveSelected}
      <IconDismiss />
    {:else}
      <IconDelete />
    {/if}
  </Button>
{/snippet}

{#if viewerIndex !== null && viewerItems.length > 0}
  <MediaViewer
    items={viewerItems}
    initialIndex={viewerIndex}
    onclose={() => viewerIndex = null}
    onopenpost={(_, idx) => {
      viewerIndex = null;
      const d = viewerDownloads[idx];
      if (d) {
        openPost(parseIdentity(d));
      }
    }}
    ondelete={async (_, idx) => {
      const d = viewerDownloads[idx];
      if (!d) return;
      try {
        await downloadState.remove(d.id);
      } catch (error) {
        notify.error(i18n.t('downloads.action_error'), error);
        return;
      }
      notify.success(i18n.t('downloads.remove'), { description: d.filename, glyph: 'removed' });
      if (viewerDownloads.length <= 1) {
        viewerIndex = null;
      }
    }}
  />
{/if}

<style>
  :global(.downloads-mode-choice) {
    flex-shrink: 0 !important;
  }

  .downloads-grid { position: relative; display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--grid-card-width)), 1fr)); align-items: start; gap: var(--grid-gap); width: 100%; }
  .page-error { margin-top: 16px; color: rgba(255,130,130,.8); font-size: 12px; text-align: center; }
</style>
