<script lang="ts">
  import { onMount, getContext, onDestroy, tick } from 'svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import type { Creator } from '$lib/types/content';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import PageHeader from '$lib/components/layout/PageHeader.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import CountBadge from '$lib/components/ui/CountBadge.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import { apiSaveSettings, apiSetCreatorFavorite } from '$lib/utils/ipc';
  import { creatorAvatarSrc, creatorPlaceholderUrl, formatProviderName } from '$lib/utils/media';
  import { notify } from '$lib/utils/toast';
  import { tooltip, ripple } from '$lib/motion';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { providerState } from '$lib/state/providerState.svelte';
  import SelectionActionBar from '$lib/components/ui/SelectionActionBar.svelte';
  import ChoiceGroup, { type ChoiceOption } from '$lib/components/ui/ChoiceGroup.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';

  import IconRefresh from '~icons/fluent/arrow-sync-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconPause from '~icons/fluent/pause-24-regular';
  import IconPlay from '~icons/fluent/play-24-regular';

  import IconOptions from '~icons/fluent/options-24-regular';
  import IconArrowSort from '~icons/fluent/arrow-sort-24-regular';
  import IconHeartFilled from '~icons/fluent/heart-24-filled';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconPersonAdd from '~icons/fluent/person-add-24-regular';
  import IconPersonDelete from '~icons/fluent/person-delete-24-regular';
  import IconHeart from '~icons/fluent/heart-24-regular';
  import IconMoreVertical from '~icons/fluent/more-vertical-24-regular';

  import type { FilterMap } from '$lib/types/filter';
  import { countActiveFilters, toggleFilterKey } from '$lib/types/filter';

  let filtersOpen = $state(false);
  let stickyFiltersOpen = $state(false);

  const savedState = navigationState.getViewState<{
    searchQuery?: string;
    searchOpen?: boolean;
    serviceFilters?: FilterMap;
    providerFilters?: FilterMap;
    sortBy?: 'name' | 'updated' | 'indexed' | 'favorited';
    sortOrder?: 'asc' | 'desc';
    activeTab?: 'all' | 'subscribed';
  }>(navigationState.entryKey);

  if (savedState) {
    if (savedState.searchQuery !== undefined) creatorsState.searchQuery = savedState.searchQuery;
    if (savedState.serviceFilters !== undefined) creatorsState.serviceFilters = savedState.serviceFilters;
    if (savedState.providerFilters !== undefined) creatorsState.providerFilters = savedState.providerFilters;
    if (savedState.sortBy !== undefined) creatorsState.sortBy = savedState.sortBy;
    if (savedState.sortOrder !== undefined) creatorsState.sortOrder = savedState.sortOrder;
    if (savedState.activeTab !== undefined) creatorsState.activeTab = savedState.activeTab;
  }

  let searchOpen = $state(savedState?.searchOpen ?? Boolean(creatorsState.searchQuery));

  $effect(() => {
    navigationState.saveViewState(navigationState.entryKey, {
      searchQuery: creatorsState.searchQuery,
      searchOpen,
      serviceFilters: $state.snapshot(creatorsState.serviceFilters),
      providerFilters: $state.snapshot(creatorsState.providerFilters),
      sortBy: creatorsState.sortBy,
      sortOrder: creatorsState.sortOrder,
      activeTab: creatorsState.activeTab
    });
  });

  let loadSentinel = $state<HTMLDivElement>();
  let observer: IntersectionObserver | undefined;

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let scaleVisible = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;

  const ratios = {
    square: '1 / 1',
    portrait: '4 / 5',
    landscape: '3 / 2',
    widescreen: '16 / 9'
  } as const;

  let baseCardWidth = $derived(layoutState.isMobile ? 155 : 245);
  let scale = $derived(configState.settings.grid_scale / 100);
  let gap = $derived((layoutState.isMobile ? 8 : 10) * scale);
  let targetCardWidth = $derived(baseCardWidth * scale);
  let ratio = $derived(ratios[configState.settings.grid_aspect_ratio]);

  const scrollContext = getContext<ScrollableContext | undefined>(SCROLLABLE_CONTEXT);

  onMount(async () => {
    if (providerState.providers.length === 0) {
      await providerState.loadProviders();
    }
    if (creatorsState.services.length === 0) {
      await creatorsState.loadServices();
    }
    void creatorsState.loadPage(false);
  });

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
    if (hideTimer) clearTimeout(hideTimer);
  });

  function loadMore() {
    if (!creatorsState.hasMore || creatorsState.loadingMore) return;
    void creatorsState.loadMore();
  }

  function handleScroll(top: number) {
    if (!creatorsState.hasMore || creatorsState.loadingMore || !scrollContext?.viewport) return;
    const vp = scrollContext.viewport;
    const distanceToBottom = vp.scrollHeight - top - vp.clientHeight;
    if (distanceToBottom < 1400) {
      loadMore();
    }
  }

  function sentinel(node: HTMLElement) {
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          loadMore();
        }
      },
      { rootMargin: '1000px 0px' }
    );
    io.observe(node);

    return {
      destroy() {
        io.disconnect();
      }
    };
  }

  function formatTimestamp(ts?: number) {
    if (!ts) return '';
    const date = new Date(ts * 1000);
    return date.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }

  function toggleService(service: string) {
    creatorsState.serviceFilters = toggleFilterKey(creatorsState.serviceFilters, service);
    void creatorsState.loadPage(true);
  }

  function resetFilters() {
    creatorsState.serviceFilters = {};
    creatorsState.providerFilters = {};
    creatorsState.aiFilter = 'neutral';
    void creatorsState.loadPage(true);
  }


  function handleWheel(event: WheelEvent) {
    if (!event.ctrlKey) return;
    event.preventDefault();

    const direction = event.deltaY < 0 ? 1 : -1;
    const step = 5;
    const nextScale = Math.min(200, Math.max(50, configState.settings.grid_scale + direction * step));
    configState.settings.grid_scale = nextScale;

    scaleVisible = true;
    if (hideTimer) clearTimeout(hideTimer);
    hideTimer = setTimeout(() => {
      scaleVisible = false;
    }, 1000);

    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      void apiSaveSettings(configState.settings);
    }, 500);
  }

  let activeCreatorSorts = $derived(providerState.activeCapabilities?.creator_sorts);

  let sortOptions = $derived.by(() => {
    const sorts = activeCreatorSorts;
    const allOptions = [
      { id: 'favorited', desc: { value: 'favorited_desc', label: i18n.t('creators.sort_favorited_desc') || 'Popularity (Desc)' }, asc: { value: 'favorited_asc', label: i18n.t('creators.sort_favorited_asc') || 'Popularity (Asc)' } },
      { id: 'updated', desc: { value: 'updated_desc', label: i18n.t('creators.sort_updated_desc') || 'Updated (Newest first)' }, asc: { value: 'updated_asc', label: i18n.t('creators.sort_updated_asc') || 'Updated (Oldest first)' } },
      { id: 'indexed', desc: { value: 'indexed_desc', label: i18n.t('creators.sort_indexed_desc') || 'Indexed (Newest first)' }, asc: { value: 'indexed_asc', label: i18n.t('creators.sort_indexed_asc') || 'Indexed (Oldest first)' } },
      { id: 'name', asc: { value: 'name_asc', label: i18n.t('creators.sort_name_asc') || 'Name (A-Z)' }, desc: { value: 'name_desc', label: i18n.t('creators.sort_name_desc') || 'Name (Z-A)' } }
    ];

    if (!sorts || sorts.length === 0) {
      return allOptions.flatMap(o => o.id === 'name' ? [o.asc, o.desc] : [o.desc, o.asc]);
    }

    const result: { value: string; label: string }[] = [];
    for (const s of sorts) {
      const matched = allOptions.find(o => o.id === s.id);
      if (matched) {
        if (s.id === 'name') {
          result.push(matched.asc, matched.desc);
        } else {
          result.push(matched.desc, matched.asc);
        }
      }
    }
    return result.length > 0 ? result : allOptions.flatMap(o => o.id === 'name' ? [o.asc, o.desc] : [o.desc, o.asc]);
  });

  let currentSortValue = $derived(`${creatorsState.sortBy}_${creatorsState.sortOrder}`);

  let currentSortLabel = $derived.by(() => {
    const opt = sortOptions.find((o) => o.value === currentSortValue);
    return opt?.label ?? (i18n.t('favorites.sort_by') || 'Sort');
  });

  function handleSortChange(val: string) {
    const parts = val.split('_');
    creatorsState.sortBy = parts[0] as any;
    creatorsState.sortOrder = parts[1] as any;
    scrollContext?.viewport?.scrollTo({ top: 0 });
    void creatorsState.loadPage(true);
  }

  let subscribedKeys = $derived(
    new Set(subscriptionState.items.map(item => `${item.service.toLowerCase()}:${item.creator_id.toLowerCase()}`))
  );

  let activeFilterCount = $derived(
    countActiveFilters([
      creatorsState.serviceFilters,
      creatorsState.providerFilters,
      creatorsState.aiFilter !== 'neutral' ? { ai: creatorsState.aiFilter } : {}
    ])
  );
  let activeTab = $derived(creatorsState.activeTab);

  let creatorsList = $derived(creatorsState.creators);
  let hasMore = $derived(creatorsState.hasMore);

  let mobileMoreOpen = $state(false);

  let creatorTabOptions = $derived<ChoiceOption<'all' | 'subscribed'>[]>([
    { value: 'all', label: i18n.t('creators.tab_all') || 'All' },
    {
      value: 'subscribed',
      label: i18n.t('creators.tab_subscribed') || 'Subscriptions',
      count: subscriptionState.items.length
    }
  ]);

  function selectTab(tab: 'all' | 'subscribed') {
    creatorsState.activeTab = tab;
    if (isSelectionActive) selectionState.clear();
    scrollContext?.viewport?.scrollTo({ top: 0 });
    void creatorsState.loadPage(true);
  }

  async function handleRefresh() {
    await creatorsState.refresh();
  }

  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'creators');

  let creatorsKeys = $derived(creatorsList.map((c) => `${c.service}:${c.id}`));
  let creatorsMap = $derived(new Map(creatorsList.map((c) => [`${c.service}:${c.id}`, c])));

  $effect(() => {
    selectionState.setContext('creators', creatorsKeys, creatorsMap);
  });

  let lastContextTime = 0;

  function handleCreatorClick(event: MouseEvent, creator: any) {
    if (Date.now() - lastContextTime < 500) return;

    const key = `${creator.service}:${creator.id}`;
    if (event.ctrlKey || event.metaKey) {
      event.preventDefault();
      event.stopPropagation();
      selectionState.toggle('creators', key, creator, creatorsKeys, false, creatorsMap);
      return;
    }

    if (isSelectionActive) {
      event.preventDefault();
      event.stopPropagation();
      selectionState.toggle('creators', key, creator, creatorsKeys, event.shiftKey, creatorsMap);
      return;
    }

    navigationState.openCreator(creator.service, creator.id);
  }

  function handleCreatorContextMenu(event: MouseEvent, creator: any) {
    event.preventDefault();
    lastContextTime = Date.now();
    try { navigator.vibrate?.(35); } catch {}
    const key = `${creator.service}:${creator.id}`;
    selectionState.toggle('creators', key, creator, creatorsKeys, false, creatorsMap);
  }

  function handleCreatorCheckbox(event: MouseEvent, creator: any) {
    event.stopPropagation();
    const key = `${creator.service}:${creator.id}`;
    selectionState.toggle('creators', key, creator, creatorsKeys, event.shiftKey, creatorsMap);
  }

  function handleSelectAllCreators() {
    selectionState.selectAll(creatorsList.map((c) => ({
      key: `${c.service}:${c.id}`,
      item: c
    })));
  }

  async function batchSubscribe() {
    const items = selectionState.getItems<any>();
    if (items.length === 0) return;
    try {
      for (const creator of items) {
        await subscriptionState.save({
          service: creator.service,
          creator_id: creator.id,
          creator_name: creator.name || creator.id,
          initial_import: 'none',
          auto_download: false,
          download_scope: 'primary',
          poll_interval_minutes: 30
        });
      }
      notify.success(
        i18n.t('selection.subscribe') || 'Subscribed',
        `${items.length} ${items.length === 1 ? 'creator' : 'creators'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('subscriptions.action_error') || 'Failed to subscribe', err);
    }
  }

  async function batchUnsubscribe() {
    const items = selectionState.getItems<any>();
    if (items.length === 0) return;
    try {
      for (const creator of items) {
        const sub = subscriptionState.forCreator(creator.service, creator.id);
        if (sub) {
          await subscriptionState.remove(sub.id);
        }
      }
      notify.success(
        i18n.t('selection.unsubscribe') || 'Unsubscribed',
        `${items.length} ${items.length === 1 ? 'creator' : 'creators'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('subscriptions.action_error') || 'Failed to unsubscribe', err);
    }
  }

  async function batchFavoriteCreators(isFav: boolean) {
    const items = selectionState.getItems<any>();
    if (items.length === 0) return;
    try {
      for (const creator of items) {
        await apiSetCreatorFavorite(creator.service, creator.id, isFav);
      }
      notify.success(
        i18n.t(isFav ? 'selection.favorite' : 'selection.unfavorite') || 'Updated favorites',
        `${items.length} ${items.length === 1 ? 'creator' : 'creators'}`
      );
      await accountState.fetchFavorites('creator');
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('post.favorite_failed') || 'Failed to update favorites', err);
    }
  }

  function toggleProvider(providerId: string) {
    creatorsState.providerFilters = toggleFilterKey(creatorsState.providerFilters, providerId);
    void creatorsState.loadPage(true);
  }

  let enabledProviders = $derived(providerState.providers.filter((p) => p.enabled));
</script>

{#snippet filterInnerContent()}
  {#if enabledProviders.length > 1}
    <span class="filter-label">{i18n.t('providers.title') || 'Sources'}</span>
    <div class="service-options">
      {#each enabledProviders as provider}
        {@const state = creatorsState.providerFilters[provider.id] ?? 'neutral'}
        {@const cleanName = formatProviderName(provider.name)}
        <Button
          variant="ghost"
          size="sm"
          onclick={() => toggleProvider(provider.id)}
          class="filter-chip {state === 'include' ? 'state-include' : state === 'exclude' ? 'state-exclude' : ''}"
        >
          <span>{cleanName}</span>
          {#if state === 'include'}
            <IconSearch class="w-3.5 h-3.5 ml-auto text-[#4ade80] shrink-0" />
          {:else if state === 'exclude'}
            <IconDismiss class="w-3.5 h-3.5 ml-auto text-[#f87171] shrink-0" />
          {/if}
        </Button>
      {/each}
    </div>
    <div class="floating-divider"></div>
  {/if}

  <span class="filter-label">{i18n.t('feed.platform')}</span>
  <div class="service-options">
    <Button
      variant={Object.keys(creatorsState.serviceFilters).length === 0 ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => {
        creatorsState.serviceFilters = {};
        void creatorsState.loadPage(true);
      }}
      class="filter-chip chip-all {Object.keys(creatorsState.serviceFilters).length === 0 ? 'state-include' : ''}"
    >
      <IconGlobe class="w-5 h-5" />
      <span>{i18n.t('feed.all_platforms')}</span>
    </Button>
    {#each creatorsState.enabledServices as service}
      {@const state = creatorsState.serviceFilters[service] ?? 'neutral'}
      <Button
        variant="ghost"
        size="sm"
        onclick={() => toggleService(service)}
        class="filter-chip {state === 'include' ? 'state-include' : state === 'exclude' ? 'state-exclude' : ''}"
      >
        <ServiceIcon service={service} class="w-5 h-5" />
        <span>{service}</span>
        {#if state === 'include'}
          <IconSearch class="w-3.5 h-3.5 ml-auto text-[#4ade80] shrink-0" />
        {:else if state === 'exclude'}
          <IconDismiss class="w-3.5 h-3.5 ml-auto text-[#f87171] shrink-0" />
        {/if}
      </Button>
    {/each}
  </div>

  {#if !configState.settings.pawchive_hide_ai}
    {@const state = creatorsState.aiFilter}
    <div class="floating-divider"></div>
    <span class="filter-label">{i18n.t('feed.format') || 'Filters'}</span>
    <div class="service-options">
      <Button
        variant="ghost"
        size="sm"
        onclick={() => {
          const cur = creatorsState.aiFilter;
          creatorsState.aiFilter = cur === 'neutral' ? 'include' : cur === 'include' ? 'exclude' : 'neutral';
          void creatorsState.loadPage(true);
        }}
        class="filter-chip {state === 'include' ? 'state-include' : state === 'exclude' ? 'state-exclude' : ''}"
      >
        <IconSparkle class="w-5 h-5" />
        <span>{i18n.t('feed.format_ai') || 'AI Generated'}</span>
        {#if state === 'include'}
          <IconSearch class="w-3.5 h-3.5 ml-auto text-[#4ade80] shrink-0" />
        {:else if state === 'exclude'}
          <IconDismiss class="w-3.5 h-3.5 ml-auto text-[#f87171] shrink-0" />
        {/if}
      </Button>
    </div>
  {/if}
{/snippet}

{#snippet creatorTabs()}
  <ChoiceGroup
    options={creatorTabOptions}
    value={activeTab}
    onchange={(val) => selectTab(val as 'all' | 'subscribed')}
    align="left"
    class="creators-mode-choice"
  >
    {#snippet activeAddon()}
      <Select
        options={sortOptions}
        value={currentSortValue}
        onchange={handleSortChange}
        class="creators-sort-select"
        icon={IconArrowSort}
        iconOnly={true}
        ariaLabel={`${i18n.t('favorites.sort_by') || 'Sort'}: ${currentSortLabel}`}
      />
    {/snippet}
  </ChoiceGroup>
{/snippet}

{#snippet creatorsFilter(sticky = false)}
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

{#snippet actionsCluster(sticky = false)}
  <HeaderActions
    bind:searchOpen
    bind:searchQuery={creatorsState.searchQuery}
    searchPlaceholder={i18n.t('creators.search_placeholder') || 'Search creators...'}
  >
    {#if !layoutState.isMobile}
      <Button
        variant={isSelectionActive ? 'accent' : 'ghost'}
        class="btn-icon"
        onclick={() => (isSelectionActive ? selectionState.exit() : selectionState.enter('creators'))}
        title={i18n.t('selection.select_mode') || 'Select mode'}
        aria-label="Select mode"
      >
        <IconCheckboxChecked class="w-5 h-5" />
      </Button>

      <Button
        variant="ghost"
        class="btn-icon"
        disabled={creatorsState.loading || creatorsState.syncing}
        aria-label={i18n.t('feed.refresh')}
        title={i18n.t('feed.refresh')}
        onclick={handleRefresh}
      >
        {#if creatorsState.loading || creatorsState.syncing}<IconLoading class="w-5 h-5" />{:else}<IconArrowClockwise class="w-5 h-5" />{/if}
      </Button>

      {@render creatorsFilter(sticky)}
    {:else}
      {@render creatorsFilter(sticky)}

      <Button
        variant="ghost"
        class="btn-icon"
        onclick={() => (mobileMoreOpen = true)}
        title={i18n.t('common.more') || 'More'}
        aria-label="More actions"
      >
        <IconMoreVertical class="w-5 h-5" />
      </Button>
    {/if}
  </HeaderActions>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey} onrefresh={handleRefresh} onscroll={handleScroll}>
  {#snippet overlay()}
    <StickyHeader threshold={120}>
      {#snippet center()}
        {@render creatorTabs()}
      {/snippet}
      {#snippet trailing()}
        {@render actionsCluster(true)}
      {/snippet}
    </StickyHeader>
  {/snippet}

  <PageHeader>
    {#snippet tabs()}
      {@render creatorTabs()}
    {/snippet}
    {#snippet actions()}
      {@render actionsCluster(false)}
    {/snippet}
  </PageHeader>

  {#if creatorsState.loading && creatorsState.creators.length === 0}
    <div class="status-container">
      <IconLoading class="spinner" />
      <span>{i18n.t('feed.loading')}</span>
    </div>
  {:else if creatorsState.error && creatorsState.creators.length === 0}
    <div class="status-container error">
      <strong>{creatorsState.error}</strong>
      <span class="error-msg">{creatorsState.error}</span>
      <Button variant="accent" size="sm" onclick={() => void creatorsState.refresh()}>
        <IconArrowClockwise /> {i18n.t('feed.retry')}
      </Button>
    </div>
  {:else if creatorsList.length === 0}
    {#if activeTab === 'subscribed'}
      <div class="status-container empty">
        <strong>{i18n.t('subscriptions.empty')}</strong>
        <span>{i18n.t('subscriptions.empty_desc')}</span>
      </div>
    {:else}
      <div class="status-container empty">
        <strong>{i18n.t('creators.empty')}</strong>
        <span>{i18n.t('creators.empty_desc')}</span>
      </div>
    {/if}
  {:else}
    {#if scaleVisible}
      <div class="scale-indicator">{configState.settings.grid_scale}%</div>
    {/if}

    <div
      class="creators-grid"
      onwheel={handleWheel}
      style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-gap: ${gap}px;`}
    >
      {#each creatorsList as creator (creator.service + ':' + creator.id)}
        {@const creatorKey = `${creator.service}:${creator.id}`}
        {@const isSelected = isSelectionActive && selectionState.isSelected(creatorKey)}
        {@const placeholder = creatorPlaceholderUrl(creator)}
        {@const avatarSrc = creatorAvatarSrc(creator)}
        <article
          class="grid-tile"
          class:selected={isSelected}
          style:aspect-ratio={ratio}
          data-creator-key={creatorKey}
        >
          <button
            class="grid-tile-open"
            type="button"
            onclick={(e) => handleCreatorClick(e, creator)}
            oncontextmenu={(e) => handleCreatorContextMenu(e, creator)}
            aria-label={creator.name}
          ></button>

          {#if isSelectionActive}
            <button
              type="button"
              class="grid-tile-select-checkbox"
              class:checked={isSelected}
              onclick={(e) => handleCreatorCheckbox(e, creator)}
              aria-label="Select creator"
            >
              {#if isSelected}
                <IconCheckmark class="w-[14px] h-[14px]" />
              {/if}
            </button>
          {/if}

          <div class="grid-tile-placeholder">
            <span class="fallback-initials">{creator.name.slice(0, 2).toUpperCase()}</span>
          </div>

          {#if placeholder}
            <img
              class="grid-tile-media placeholder-blur"
              src={placeholder}
              alt=""
              aria-hidden="true"
            />
          {/if}

          {#if avatarSrc}
            <img
              class="grid-tile-media"
              src={avatarSrc}
              alt=""
              loading="lazy"
              decoding="async"
              onerror={(e) => {
                (e.currentTarget as HTMLImageElement).style.display = 'none';
              }}
            />
          {/if}

          <div class="grid-tile-shade"></div>

          <div class="grid-tile-footer">
            <div class="grid-tile-author">
              <button
                type="button"
                class="grid-tile-logo inline-logo"
                onclick={(e) => handleCreatorClick(e, creator)}
                use:tooltip={creator.service}
                aria-label={`${i18n.t('feed.open_creator')}: ${creator.service}`}
              >
                <ServiceIcon service={creator.service} />
              </button>

              <span
                role="link"
                tabindex="0"
                class="grid-tile-author-name"
                onclick={(e) => handleCreatorClick(e, creator)}
                onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleCreatorClick(e as any, creator)}
              >
                {creator.name}
              </span>
            </div>

            <div class="grid-tile-meta">
              <span>{creator.service} · {creator.id}</span>
              <div class="grid-tile-meta-stats">
                {#if Number(creator.favorited ?? 0) > 0}
                  <span class="grid-tile-meta-row">
                    <IconHeartFilled />
                    {creator.favorited}
                  </span>
                {/if}
                <span>{formatTimestamp(creator.updated)}</span>
              </div>
            </div>
          </div>
        </article>
      {/each}
    </div>

    {#if hasMore}
      <div use:sentinel class="sentinel">
        {#if creatorsState.loadingMore}
          <IconLoading />
        {/if}
      </div>
    {/if}
  {/if}
</PageShell>

<SelectionActionBar
  totalCount={creatorsList.length}
  onSelectAll={handleSelectAllCreators}
>
  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={batchSubscribe}
    title={i18n.t('selection.subscribe')}
  >
    <IconPersonAdd class="w-[16px] h-[16px]" />
    <span>{i18n.t('selection.subscribe')}</span>
  </Button>

  {#if activeTab === 'subscribed'}
    <Button
      variant="danger"
      size="sm"
      class="selection-btn"
      onclick={batchUnsubscribe}
      title={i18n.t('selection.unsubscribe')}
    >
      <IconPersonDelete class="w-[16px] h-[16px]" />
      <span>{i18n.t('selection.unsubscribe')}</span>
    </Button>
  {/if}

  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={() => void batchFavoriteCreators(true)}
    title={i18n.t('selection.favorite')}
  >
    <IconHeartFilled class="w-[16px] h-[16px] text-accent" />
    <span>{i18n.t('selection.favorite')}</span>
  </Button>
</SelectionActionBar>

{#if layoutState.isMobile}
  <BottomSheet
    open={mobileMoreOpen}
    title={i18n.t('common.more') || 'More'}
    onclose={() => (mobileMoreOpen = false)}
  >
    <div class="flex flex-col gap-1 py-1">
      <button
        type="button"
        class="sheet-action-item"
        use:ripple
        onclick={() => {
          mobileMoreOpen = false;
          if (isSelectionActive) {
            selectionState.exit();
          } else {
            selectionState.enter('creators');
          }
        }}
      >
        <IconCheckboxChecked class={isSelectionActive ? 'text-accent' : 'text-secondary'} />
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{isSelectionActive ? (i18n.t('selection.exit') || 'Exit selection mode') : (i18n.t('selection.select_mode') || 'Select creators')}</span>
        </div>
      </button>

      <button
        type="button"
        class="sheet-action-item"
        disabled={creatorsState.loading || creatorsState.syncing}
        use:ripple
        onclick={() => {
          mobileMoreOpen = false;
          void handleRefresh();
        }}
      >
        {#if creatorsState.loading || creatorsState.syncing}
          <IconLoading class="text-accent" />
        {:else}
          <IconArrowClockwise class="text-secondary" />
        {/if}
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{i18n.t('feed.refresh') || 'Refresh'}</span>
        </div>
      </button>
    </div>
  </BottomSheet>
{/if}

<style>

  :global(.creators-mode-choice) {
    flex-shrink: 0 !important;
  }

  :global(.creators-sort-select) {
    width: auto !important;
    max-width: none !important;
    flex-shrink: 0 !important;
  }

  :global(.creators-sort-select .select-trigger),
  :global(.creators-sort-select .select-trigger.icon-only) {
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 calc(3px * var(--ui-scale, 1)) 0 0 !important;
    background: var(--accent-container) !important;
    color: var(--choice-active-bg, var(--accent-on-container)) !important;
    border-radius: calc(var(--radius-sm, 6px) * var(--ui-scale, 1))
                   min(calc(var(--radius-full) * var(--ui-scale, 1)), calc(var(--control-height, 46px) / 2))
                   min(calc(var(--radius-full) * var(--ui-scale, 1)), calc(var(--control-height, 46px) / 2))
                   calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) !important;
    border-top-left-radius: calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) !important;
    border-bottom-left-radius: calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) !important;
    border-top-right-radius: min(calc(var(--radius-full) * var(--ui-scale, 1)), calc(var(--control-height, 46px) / 2)) !important;
    border-bottom-right-radius: min(calc(var(--radius-full) * var(--ui-scale, 1)), calc(var(--control-height, 46px) / 2)) !important;
    border: none !important;
    box-shadow: none !important;
    transition:
      background var(--duration-fast) var(--ease-expo),
      color var(--duration-fast) var(--ease-expo),
      opacity var(--duration-fast) var(--ease-expo) !important;
  }

  :global(.creators-sort-select .select-trigger:hover),
  :global(.creators-sort-select .select-trigger.icon-only:hover) {
    background: color-mix(in srgb, var(--accent-container) 70%, var(--accent-primary)) !important;
    color: var(--choice-active-bg, var(--accent-on-container)) !important;
  }

  :global(.creators-sort-select .select-trigger:active),
  :global(.creators-sort-select .select-trigger.icon-only:active) {
    opacity: 0.85 !important;
  }

  :global(.creators-sort-select .select-trigger svg),
  :global(.creators-sort-select .select-trigger.icon-only svg) {
    width: 20px !important;
    height: 20px !important;
    color: var(--choice-active-bg, var(--accent-on-container)) !important;
    opacity: 1 !important;
  }

  .status-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 6px;
    min-height: 310px;
    color: rgba(255, 255, 255, 0.42);
  }

  .status-container.error {
    color: #fca5a5;
  }

  .error-msg {
    max-width: 400px;
    font-size: 12px;
    opacity: 0.8;
    margin-bottom: 8px;
    overflow-wrap: anywhere;
  }

  .status-container.empty strong {
    font-size: 14px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.76);
  }

  .status-container.empty span {
    max-width: 360px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.42);
    line-height: 1.5;
  }

  .status-container :global(svg) {
    width: 34px;
    height: 34px;
    color: rgba(255, 255, 255, 0.42);
    margin-bottom: 5px;
  }

  .creators-grid {
    position: relative;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--grid-card-width)), 1fr));
    gap: var(--grid-gap);
    width: 100%;
  }

  .fallback-initials {
    font-family: var(--font-sans);
    font-size: calc(30px * var(--grid-scale, 1));
    font-weight: 700;
    letter-spacing: 0.05em;
    color: rgba(255, 255, 255, 0.22);
  }

  .sentinel {
    display: grid;
    place-items: center;
    height: 60px;
    margin-top: 20px;
    color: var(--text-muted);
  }

  .scale-indicator {
    position: fixed;
    z-index: 80;
    left: 50%;
    bottom: 34px;
    transform: translateX(-50%);
    padding: 7px 12px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 999px;
    background: rgba(10, 10, 14, 0.82);
    color: white;
    font-size: 12px;
    font-weight: 650;
    backdrop-filter: blur(14px);
    pointer-events: none;
  }
</style>
