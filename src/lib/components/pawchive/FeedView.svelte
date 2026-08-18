<script lang="ts">
  import { onMount, getContext, tick } from 'svelte';
  import { feedState, type FeedMode, type PopularPeriod } from '$lib/state/feedState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import { i18n } from '$lib/i18n';
  import { accountState } from '$lib/state/accountState.svelte';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import PageHeader from '$lib/components/layout/PageHeader.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import PostGrid from './PostGrid.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import { ripple } from '$lib/motion';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { getPostDownloadTargets } from '$lib/utils/media';
  import { apiSetPostFavorite } from '$lib/utils/ipc';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { notify } from '$lib/utils/toast';
  import type { PawchivePost } from '$lib/types/pawchive';
  import SelectionActionBar from '$lib/components/ui/SelectionActionBar.svelte';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconOptions from '~icons/fluent/options-24-regular';
  import IconCalendar from '~icons/fluent/calendar-ltr-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconText from '~icons/fluent/document-text-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconStar from '~icons/fluent/star-24-filled';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconBookmarkAdd from '~icons/fluent/bookmark-add-24-regular';
  import IconHeartFilled from '~icons/fluent/heart-24-filled';

  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'posts');
  let selectedPosts = $derived(isSelectionActive ? selectionState.getItems<PawchivePost>() : []);
  let stashes = $derived(libraryState.collections.filter((c) => c.kind === 'stash'));
  let stashOptions = $derived(stashes.map((s) => ({ value: s.id, label: s.name })));

  let batchSelectedStashes = $derived.by(() => {
    if (selectedPosts.length === 0) return [];
    const stashCounts = new Map<string, number>();
    for (const post of selectedPosts) {
      const ids = libraryState.getCustomPostStashes(post);
      for (const id of ids) {
        stashCounts.set(id, (stashCounts.get(id) || 0) + 1);
      }
    }
    const result: string[] = [];
    for (const [id, count] of stashCounts.entries()) {
      if (count === selectedPosts.length) {
        result.push(id);
      }
    }
    return result;
  });

  async function handleBatchToggleStash(collectionId: string) {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0 || !collectionId) return;
    const isAllIn = batchSelectedStashes.includes(collectionId);
    try {
      if (isAllIn) {
        for (const p of items) {
          await libraryState.removeFromStash(collectionId, p);
        }
        notify.success(i18n.t('library.removed_from_stash') || 'Removed from stash');
      } else {
        for (const p of items) {
          await libraryState.save(p, collectionId);
        }
        notify.success(i18n.t('library.added_to_stash') || 'Added to stash');
      }
    } catch (error) {
      notify.error(i18n.t('library.save_error') || 'Stash operation failed', error);
    }
  }

  async function handleBatchCreateAndAddToStash(name: string) {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0 || !name.trim()) return;
    try {
      const newStash = await libraryState.createStash(name.trim());
      for (const p of items) {
        await libraryState.save(p, newStash.id);
      }
      notify.success(i18n.t('library.added_to_stash') || 'Added to stash', newStash.name);
    } catch (error) {
      notify.error(i18n.t('library.save_error') || 'Failed to create stash', error);
    }
  }

  function handleSelectAllPosts() {
    selectionState.selectAll(feedState.filteredPosts.map((p) => ({
      key: `${p.service}:${p.user}:${p.id}`,
      item: p
    })));
  }

  async function batchSaveToLibrary() {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0) return;
    try {
      for (const post of items) {
        await libraryState.save(post);
      }
      notify.success(
        i18n.t('selection.save_to_library') || 'Saved to library',
        `${items.length} ${items.length === 1 ? 'post' : 'posts'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('library.save_error') || 'Failed to save to library', err);
    }
  }

  async function batchDownloadPosts() {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0) return;
    let count = 0;
    try {
      for (const post of items) {
        const targets = getPostDownloadTargets(post);
        for (const target of targets) {
          await downloadState.start(post, target.mediaId, target.url, target.filename);
          count++;
        }
      }
      notify.success(
        i18n.t('selection.download_all') || 'Queued downloads',
        `${count} ${count === 1 ? 'file' : 'files'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('downloads.action_error') || 'Download failed', err);
    }
  }

  async function batchFavoritePosts(isFav: boolean) {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0) return;
    try {
      for (const post of items) {
        await apiSetPostFavorite(post.service, post.user, post.id, isFav);
      }
      notify.success(
        i18n.t(isFav ? 'selection.favorite' : 'selection.unfavorite') || 'Updated favorites',
        `${items.length} ${items.length === 1 ? 'post' : 'posts'}`
      );
      await accountState.fetchFavorites('post');
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('post.favorite_failed') || 'Failed to update favorites', err);
    }
  }

  let filtersOpen = $state(false);
  let stickyFiltersOpen = $state(false);
  let services = $derived([...new Set(feedState.posts.map((post) => post.service))].sort());
  let activeFilterCount = $derived(
    feedState.selectedServices.length +
    feedState.selectedFormats.length +
    (feedState.onlyWithAttachments ? 1 : 0) +
    (feedState.favoritesOnly ? 1 : 0)
  );

  function toggleService(service: string) {
    if (feedState.selectedServices.includes(service)) {
      feedState.selectedServices = feedState.selectedServices.filter((s) => s !== service);
    } else {
      feedState.selectedServices = [...feedState.selectedServices, service];
    }
  }

  function toggleFormat(fmt: string) {
    if (feedState.selectedFormats.includes(fmt)) {
      feedState.selectedFormats = feedState.selectedFormats.filter((f) => f !== fmt);
    } else {
      feedState.selectedFormats = [...feedState.selectedFormats, fmt];
    }
  }

  function toggleFavoritesFilter() {
    feedState.favoritesOnly = !feedState.favoritesOnly;
    if (feedState.favoritesOnly && accountState.favoriteCreators === null) {
      void accountState.fetchFavorites('creator');
    }
  }

  const savedState = navigationState.getViewState<{
    searchQuery?: string;
    searchOpen?: boolean;
    mode?: FeedMode;
    popularPeriod?: PopularPeriod;
    popularDate?: string;
    selectedServices?: string[];
    selectedFormats?: string[];
    favoritesOnly?: boolean;
    onlyWithAttachments?: boolean;
  }>(navigationState.entryKey);

  if (savedState) {
    if (savedState.searchQuery !== undefined) feedState.searchQuery = savedState.searchQuery;
    if (savedState.mode !== undefined) feedState.mode = savedState.mode;
    if (savedState.popularPeriod !== undefined) feedState.popularPeriod = savedState.popularPeriod;
    if (savedState.popularDate !== undefined) feedState.popularDate = savedState.popularDate;
    if (savedState.selectedServices !== undefined) feedState.selectedServices = savedState.selectedServices;
    if (savedState.selectedFormats !== undefined) feedState.selectedFormats = savedState.selectedFormats;
    if (savedState.favoritesOnly !== undefined) feedState.favoritesOnly = savedState.favoritesOnly;
    if (savedState.onlyWithAttachments !== undefined) feedState.onlyWithAttachments = savedState.onlyWithAttachments;
  }

  let searchOpen = $state(savedState?.searchOpen ?? Boolean(feedState.searchQuery));
  const scrollContext = getContext<ScrollableContext | undefined>(SCROLLABLE_CONTEXT);

  $effect(() => {
    navigationState.saveViewState(navigationState.entryKey, {
      searchQuery: feedState.searchQuery,
      searchOpen,
      mode: feedState.mode,
      popularPeriod: feedState.popularPeriod,
      popularDate: feedState.popularDate,
      selectedServices: feedState.selectedServices,
      selectedFormats: feedState.selectedFormats,
      favoritesOnly: feedState.favoritesOnly,
      onlyWithAttachments: feedState.onlyWithAttachments
    });
  });

  let customDateInput = $state<HTMLInputElement>();
  let customMonthInput = $state<HTMLInputElement>();

  let popularSelectOptions = $derived([
    { value: 'day:today', label: i18n.t('feed.today') || 'Today' },
    { value: 'day:yesterday', label: i18n.t('feed.yesterday') || 'Yesterday' },
    { value: 'week:current', label: i18n.t('feed.this_week') || 'This Week' },
    { value: 'week:last', label: i18n.t('feed.last_week') || 'Last Week' },
    { value: 'month:current', label: i18n.t('feed.this_month') || 'This Month' },
    { value: 'month:last', label: i18n.t('feed.last_month') || 'Last Month' },
    {
      value: 'custom:day',
      label: feedState.popularPeriod === 'day' && feedState.popularDate
        ? `${i18n.t('feed.day') || 'Day'}: ${feedState.popularDate}`
        : `${i18n.t('feed.custom_day') || 'Custom Day...'}`
    },
    {
      value: 'custom:month',
      label: feedState.popularPeriod === 'month' && feedState.popularDate
        ? `${i18n.t('feed.month') || 'Month'}: ${feedState.popularDate}`
        : `${i18n.t('feed.custom_month') || 'Custom Month...'}`
    }
  ]);

  function getSelectedOptionValue(): string {
    const period = feedState.popularPeriod;
    const date = feedState.popularDate;

    if (!date) {
      if (period === 'day') return 'day:today';
      if (period === 'week') return 'week:current';
      if (period === 'month') return 'month:current';
      return 'day:today';
    }

    const today = new Date();

    const yesterday = new Date(today);
    yesterday.setDate(today.getDate() - 1);
    const yesterdayStr = yesterday.toISOString().split('T')[0];
    if (period === 'day' && date === yesterdayStr) return 'day:yesterday';

    const lastWeek = new Date(today);
    lastWeek.setDate(today.getDate() - 7);
    const lastWeekStr = lastWeek.toISOString().split('T')[0];
    if (period === 'week' && date === lastWeekStr) return 'week:last';

    const lastMonth = new Date(today);
    lastMonth.setMonth(today.getMonth() - 1);
    const lastMonthStr = lastMonth.toISOString().split('T')[0].substring(0, 7);
    if (period === 'month' && date.startsWith(lastMonthStr)) return 'month:last';

    return `custom:${period}`;
  }

  function handlePeriodChange(val: string) {
    const today = new Date();

    if (val === 'day:today') {
      feedState.popularPeriod = 'day';
      feedState.popularDate = '';
      void feedState.refresh();
    } else if (val === 'day:yesterday') {
      const yesterday = new Date(today);
      yesterday.setDate(today.getDate() - 1);
      feedState.popularPeriod = 'day';
      feedState.popularDate = yesterday.toISOString().split('T')[0];
      void feedState.refresh();
    } else if (val === 'week:current') {
      feedState.popularPeriod = 'week';
      feedState.popularDate = '';
      void feedState.refresh();
    } else if (val === 'week:last') {
      const lastWeek = new Date(today);
      lastWeek.setDate(today.getDate() - 7);
      feedState.popularPeriod = 'week';
      feedState.popularDate = lastWeek.toISOString().split('T')[0];
      void feedState.refresh();
    } else if (val === 'month:current') {
      feedState.popularPeriod = 'month';
      feedState.popularDate = '';
      void feedState.refresh();
    } else if (val === 'month:last') {
      const lastMonth = new Date(today);
      lastMonth.setMonth(today.getMonth() - 1);
      feedState.popularPeriod = 'month';
      feedState.popularDate = lastMonth.toISOString().split('T')[0].substring(0, 7);
      void feedState.refresh();
    } else if (val === 'custom:day') {
      if (customDateInput) {
        customDateInput.showPicker();
      }
    } else if (val === 'custom:month') {
      if (customMonthInput) {
        customMonthInput.showPicker();
      }
    }
  }

  function handleCustomDateChange(e: Event) {
    const el = e.target as HTMLInputElement;
    if (el.value) {
      feedState.popularPeriod = 'day';
      feedState.popularDate = el.value;
      void feedState.refresh();
    }
  }

  function handleCustomMonthChange(e: Event) {
    const el = e.target as HTMLInputElement;
    if (el.value) {
      feedState.popularPeriod = 'month';
      feedState.popularDate = el.value;
      void feedState.refresh();
    }
  }

  $effect(() => {
    if (searchOpen) {
      const el = document.querySelector('.search-input-wrapper input') as HTMLInputElement | null;
      if (el) {
        el.focus();
      }
    }
  });

  function resetFilters() {
    feedState.selectedServices = [];
    feedState.selectedFormats = [];
    feedState.onlyWithAttachments = false;
    feedState.favoritesOnly = false;
  }

  onMount(() => {
    if (!feedState.current.loaded) void feedState.refresh();
  });
</script>

{#snippet filterInnerContent()}
  <div class="filter-heading">
    <strong>{i18n.t('feed.filters')}</strong>
    {#if activeFilterCount > 0}
      <button type="button" use:ripple onclick={resetFilters}>{i18n.t('feed.reset_filters')}</button>
    {/if}
  </div>

  <span class="filter-label">{i18n.t('feed.platform')}</span>
  <div class="service-options">
    <Button
      variant={feedState.selectedServices.length === 0 ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => feedState.selectedServices = []}
      class="filter-chip"
    >
      <IconGlobe class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.all_platforms')}</span>
    </Button>
    {#each services as service}
      <Button
        variant={feedState.selectedServices.includes(service) ? 'accent' : 'ghost'}
        size="sm"
        onclick={() => toggleService(service)}
        class="filter-chip"
      >
        <ServiceIcon service={service} class="w-[14px] h-[14px]" />
        <span>{service}</span>
      </Button>
    {/each}
  </div>

  <span class="filter-label">{i18n.t('feed.format') || 'Format'}</span>
  <div class="service-options">
    <Button
      variant={feedState.selectedFormats.includes('image') ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => toggleFormat('image')}
      class="filter-chip"
    >
      <IconImage class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.format_photo') || 'Photo'}</span>
    </Button>

    <Button
      variant={feedState.selectedFormats.includes('video') ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => toggleFormat('video')}
      class="filter-chip"
    >
      <IconVideo class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.format_video') || 'Video'}</span>
    </Button>

    <Button
      variant={feedState.selectedFormats.includes('audio') ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => toggleFormat('audio')}
      class="filter-chip"
    >
      <IconMusic class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.format_audio') || 'Audio'}</span>
    </Button>

    <Button
      variant={feedState.selectedFormats.includes('text') ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => toggleFormat('text')}
      class="filter-chip"
    >
      <IconText class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.format_text') || 'Text'}</span>
    </Button>

    <Button
      variant={feedState.selectedFormats.includes('archive') ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => toggleFormat('archive')}
      class="filter-chip"
    >
      <IconDocument class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.format_archive') || 'Files'}</span>
    </Button>
  </div>

  <span class="filter-label section-label">{i18n.t('feed.filters')}</span>
  <div class="view-option" class:active={feedState.onlyWithAttachments}>
    <Checkbox
      checked={feedState.onlyWithAttachments}
      onchange={(v) => feedState.onlyWithAttachments = v}
    />
    <button onclick={() => feedState.onlyWithAttachments = !feedState.onlyWithAttachments}>
      <strong>{i18n.t('feed.with_attachments')}</strong>
      <small>{i18n.t('feed.with_attachments_desc')}</small>
    </button>
    <IconDocument class="view-option-icon w-[20px] h-[20px]" />
  </div>

  <div class="view-option" class:active={feedState.favoritesOnly}>
    <Checkbox
      checked={feedState.favoritesOnly}
      onchange={toggleFavoritesFilter}
    />
    <button onclick={toggleFavoritesFilter}>
      <strong>{i18n.t('feed.favorite_creators_only')}</strong>
      <small>{i18n.t('feed.favorite_creators_desc')}</small>
    </button>
    <IconStar class="view-option-icon w-[20px] h-[20px] text-amber-500" />
  </div>
{/snippet}

{#snippet feedFilter(sticky = false)}
  {#if sticky}
    <PopoverMenu
      bind:open={stickyFiltersOpen}
      title={i18n.t('feed.filters')}
      badge={activeFilterCount}
      active={activeFilterCount > 0}
      icon={IconOptions}
    >
      {@render filterInnerContent()}
    </PopoverMenu>
  {:else}
    <PopoverMenu
      bind:open={filtersOpen}
      title={i18n.t('feed.filters')}
      badge={activeFilterCount}
      active={activeFilterCount > 0}
      icon={IconOptions}
    >
      {@render filterInnerContent()}
    </PopoverMenu>
  {/if}
{/snippet}

{#snippet feedTabs()}
  <nav class="feed-tabs" aria-label="Feed mode">
    <Button
      variant={feedState.mode === 'recent' ? 'accent' : 'ghost'}
      onclick={() => void feedState.setMode('recent' as FeedMode)}
    >
      <span>{i18n.t('feed.recent')}</span>
    </Button>

    <Button
      variant={feedState.mode === 'popular' ? 'accent' : 'ghost'}
      onclick={() => void feedState.setMode('popular' as FeedMode)}
    >
      <span>{i18n.t('feed.popular')}</span>
    </Button>
  </nav>
{/snippet}

{#snippet popularFilter()}
  {#if feedState.mode === 'popular'}
    <Select
      variant="ghost"
      options={popularSelectOptions}
      value={getSelectedOptionValue()}
      onchange={handlePeriodChange}
      class="popular-period-select"
      style="height: 44px;"
      icon={IconCalendar}
      iconOnly={layoutState.isMobile}
      ariaLabel={i18n.t('feed.popular_period') || 'Period'}
    />
  {/if}
{/snippet}

{#snippet actionsCluster(sticky = false)}
  <HeaderActions
    bind:searchOpen
    bind:searchQuery={feedState.searchQuery}
    searchPlaceholder={i18n.t('feed.search_placeholder') || 'Search posts...'}
  >
    <Button
      variant={isSelectionActive ? 'accent' : 'ghost'}
      class="btn-icon"
      onclick={() => (isSelectionActive ? selectionState.exit() : selectionState.enter('posts'))}
      title={i18n.t('selection.select_mode') || 'Select mode'}
      aria-label="Select mode"
    >
      <IconCheckboxChecked class="w-5 h-5" />
    </Button>

    <Button
      variant="ghost"
      class="btn-icon"
      disabled={feedState.isLoading}
      aria-label={i18n.t('feed.refresh')}
      title={i18n.t('feed.refresh')}
      onclick={() => void feedState.refresh()}
    >
      {#if feedState.isLoading}<IconLoading class="w-5 h-5" />{:else}<IconArrowClockwise class="w-5 h-5" />{/if}
    </Button>

    {@render feedFilter(sticky)}
  </HeaderActions>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey} onrefresh={() => feedState.refresh()}>
  {#snippet overlay()}
    <StickyHeader threshold={120} title={i18n.t('feed.title') || 'Feed'}>
      {#snippet center()}
        <div class="flex items-center gap-2">
          {@render feedTabs()}
          {@render popularFilter()}
        </div>
      {/snippet}
      {#snippet trailing()}
        {@render actionsCluster(true)}
      {/snippet}
    </StickyHeader>
  {/snippet}

  <PageHeader>
    {#snippet tabs()}
      <div class="flex items-center gap-2">
        {@render feedTabs()}
        {@render popularFilter()}
      </div>
    {/snippet}
    {#snippet actions()}
      {@render actionsCluster(false)}
    {/snippet}
  </PageHeader>

  {#if feedState.error && feedState.posts.length === 0}
    <div class="min-h-80 flex flex-col items-center justify-center gap-4 text-center">
      <div class="max-w-md">
        <p class="text-sm font-semibold text-white/85">{i18n.t('feed.load_error')}</p>
        <p class="mt-1 text-xs leading-relaxed text-white/45 break-words">{feedState.error}</p>
      </div>
      <Button variant="accent" size="sm" onclick={() => void feedState.refresh()}>
        <IconArrowClockwise class="h-4 w-4" /> {i18n.t('feed.retry')}
      </Button>
    </div>
  {:else}
    <PostGrid
      posts={feedState.filteredPosts}
      loading={feedState.isLoading}
      hasMore={feedState.hasMore}
      onLoadMore={() => feedState.loadMore()}
      stateKey={`${feedState.isSearchActive ? `feed:search:${feedState.searchQuery.trim()}` : (feedState.mode === 'recent' ? 'feed:recent' : `feed:popular:${feedState.popularPeriod}:${feedState.popularDate}`)}:services=${feedState.selectedServices.join(',')}:formats=${feedState.selectedFormats.join(',')}:attachments=${feedState.onlyWithAttachments}:favs=${feedState.favoritesOnly}`}
      paginationKey={`${feedState.mode}:${feedState.isSearchActive ? `search:${feedState.searchQuery.trim()}` : feedState.popularPeriod}:${feedState.popularDate}:${feedState.posts.length}`}
      emptyTitle={feedState.isSearchActive ? (i18n.t('favorites.no_results') || 'Nothing found') : (i18n.t('feed.empty') || 'No posts')}
      emptyDescription={feedState.isSearchActive ? (i18n.t('favorites.no_results_desc') || 'Try adjusting your search query.') : (i18n.t('feed.empty_desc') || 'The current feed is empty.')}
    />
    {#if feedState.error}
      <p class="mt-5 text-center text-xs text-red-300/80">{feedState.error}</p>
    {/if}
  {/if}
</PageShell>

<SelectionActionBar
  totalCount={feedState.filteredPosts.length}
  onSelectAll={handleSelectAllPosts}
>
  <Select
    options={stashOptions}
    selectedValues={batchSelectedStashes}
    placeholder={i18n.t('library.add_to_stash')}
    onchange={handleBatchToggleStash}
    createLabel={i18n.t('library.new_stash')}
    onCreate={handleBatchCreateAndAddToStash}
    variant={batchSelectedStashes.length > 0 ? 'accent' : 'ghost'}
    multi={true}
    closeOnChange={false}
    icon={IconFolder}
    class="selection-stash-select"
  />

  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={batchSaveToLibrary}
    title={i18n.t('selection.save_to_library')}
  >
    <IconBookmarkAdd class="w-[16px] h-[16px]" />
    <span>{i18n.t('selection.save_to_library')}</span>
  </Button>

  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={batchDownloadPosts}
    title={i18n.t('selection.download_all')}
  >
    <IconArrowDownload class="w-[16px] h-[16px]" />
    <span>{i18n.t('selection.download_all')}</span>
  </Button>

  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={() => void batchFavoritePosts(true)}
    title={i18n.t('selection.favorite')}
  >
    <IconHeartFilled class="w-[16px] h-[16px] text-accent" />
    <span>{i18n.t('selection.favorite')}</span>
  </Button>
</SelectionActionBar>

<input
  bind:this={customDateInput}
  type="date"
  class="hidden-picker"
  onchange={handleCustomDateChange}
/>
<input
  bind:this={customMonthInput}
  type="month"
  class="hidden-picker"
  onchange={handleCustomMonthChange}
/>

<style>
  :global(.btn-icon) {
    width: 44px !important;
    height: 44px !important;
    padding: 0 !important;
    border-radius: var(--radius-full) !important;
    flex-shrink: 0;
    display: inline-flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  :global(.btn-icon svg) {
    width: 20px !important;
    height: 20px !important;
    flex-shrink: 0 !important;
  }

  .feed-tabs {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  :global(.feed-tabs .btn) {
    height: 44px !important;
    padding: 0 18px !important;
    font-size: 13.5px !important;
    border-radius: var(--radius-full) !important;
    gap: 8px !important;
  }

  :global(.popular-period-select) {
    width: auto !important;
    max-width: none !important;
    flex-shrink: 0 !important;
  }

  :global(.popular-period-select .select-trigger.variant-ghost) {
    height: 44px !important;
    font-size: 13px !important;
    padding: 0 14px !important;
    border-radius: var(--radius-full) !important;
    min-width: 80px !important;
    width: auto !important;
    max-width: none !important;
  }

  .hidden-picker {
    position: absolute;
    width: 0;
    height: 0;
    opacity: 0;
    pointer-events: none;
  }

  :global(.popular-period-select) {
    width: auto !important;
    flex-shrink: 0 !important;
  }

  :global(.popular-period-select .select-trigger) {
    height: 44px !important;
    font-size: 13.5px !important;
    padding: 0 16px !important;
    border-radius: var(--radius-full) !important;
    width: auto !important;
    min-width: 180px !important;
  }
</style>
