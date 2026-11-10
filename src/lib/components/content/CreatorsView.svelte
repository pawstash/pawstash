<script lang="ts">
  import { haptic } from '$lib/utils/haptics';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import { paintSelect } from '$lib/actions/paintSelect';
  import { onMount, getContext, onDestroy } from 'svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatDate } from '$lib/utils/formatters';
  import type { Creator } from '$lib/types/content';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import { apiSetCreatorFavorite } from '$lib/utils/ipc';
  import { gridScaleWheel } from '$lib/state/scaleState.svelte';
  import { creatorAvatarSrc, creatorPlaceholderUrl, formatProviderName, onAvatarError, onAvatarLoad } from '$lib/utils/media';
  import { notify } from '$lib/utils/toast';
  import { tooltip, ripple } from '$lib/motion';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { favoriteActions } from '$lib/state/favoriteActions.svelte';
  import { contextMenuState } from '$lib/state/contextMenuState.svelte';
  import { creatorMenuItems } from '$lib/utils/contextMenus';
  import { providerState } from '$lib/state/providerState.svelte';
  import ChoiceGroup, { type ChoiceOption } from '$lib/components/ui/ChoiceGroup.svelte';
  import SortControl from '$lib/components/layout/SortControl.svelte';
  import { sortOption, type SortOption } from '$lib/utils/sort';
  import type { MenuItem } from '$lib/components/layout/OverflowMenu.svelte';
  import SkeletonGrid from '$lib/components/ui/SkeletonGrid.svelte';
  import { GridHandoff } from '$lib/motion/gridHandoff.svelte';
  import { progressiveReveal } from '$lib/utils/progressiveReveal.svelte';

  import IconOptions from '~icons/fluent/options-24-regular';
  import IconHeartFilled from '~icons/fluent/heart-24-filled';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconWarning from '~icons/fluent/warning-24-regular';
  import IconPeople from '~icons/fluent/people-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconCheckmark from '~icons/fluent/checkmark-20-filled';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconPersonAdd from '~icons/fluent/person-add-24-regular';
  import IconPersonDelete from '~icons/fluent/person-delete-24-regular';
  import IconPersonSubscribed from '~icons/fluent/person-available-24-filled';
  import IconHeart from '~icons/fluent/heart-24-regular';

  import type { FilterMap } from '$lib/types/filter';
  import { countActiveFilters, toggleFilterKey } from '$lib/types/filter';
  import { ctrlWheel } from '$lib/utils/ctrlWheel';

  let filtersOpen = $state(false);

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

  function toggleService(service: string) {
    creatorsState.serviceFilters = toggleFilterKey(creatorsState.serviceFilters, service);
    void creatorsState.loadPage(true);
  }

  let activeCreatorSorts = $derived(providerState.activeCapabilities?.creator_sorts);

  let sortOptions = $derived.by(() => {
    const sorts = activeCreatorSorts;
    const allOptions = [
      { id: 'favorited', desc: sortOption('favorited_desc', 'popularity', 'desc'), asc: sortOption('favorited_asc', 'popularity', 'asc') },
      { id: 'updated', desc: sortOption('updated_desc', 'updated', 'desc'), asc: sortOption('updated_asc', 'updated', 'asc') },
      { id: 'indexed', desc: sortOption('indexed_desc', 'indexed', 'desc'), asc: sortOption('indexed_asc', 'indexed', 'asc') },
      { id: 'name', asc: sortOption('name_asc', 'name', 'asc'), desc: sortOption('name_desc', 'name', 'desc') }
    ];

    if (!sorts || sorts.length === 0) {
      return allOptions.flatMap(o => o.id === 'name' ? [o.asc, o.desc] : [o.desc, o.asc]);
    }

    const result: SortOption<string>[] = [];
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

  function handleSortChange(val: string) {
    const parts = val.split('_');
    creatorsState.sortBy = parts[0] as any;
    creatorsState.sortOrder = parts[1] as any;
    scrollContext?.viewport?.scrollTo({ top: 0 });
    void creatorsState.loadPage(true);
  }

  let activeFilterCount = $derived(
    countActiveFilters([
      creatorsState.serviceFilters,
      creatorsState.providerFilters,
      creatorsState.aiFilter !== 'neutral' ? { ai: creatorsState.aiFilter } : {}
    ])
  );
  let activeTab = $derived(creatorsState.activeTab);

  let creatorsList = $derived(creatorsState.creators);
  const reveal = progressiveReveal(() => creatorsList.length);

  const handoff = new GridHandoff();
  $effect(() => handoff.sync(creatorsList.length > 0, creatorsState.loading));
  onDestroy(() => handoff.destroy());
  let hasMore = $derived(creatorsState.hasMore);

  let creatorTabOptions = $derived<ChoiceOption<'all' | 'subscribed'>[]>([
    { value: 'all', label: i18n.t('creators.tab_all') },
    {
      value: 'subscribed',
      label: i18n.t('creators.tab_subscribed'),
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

  let pageMenu = $derived<MenuItem[]>([
    {
      group: 'page',
      label: i18n.t(isSelectionActive ? 'selection.exit' : 'selection.select_mode'),
      icon: IconCheckboxChecked,
      active: isSelectionActive,
      onclick: () => (isSelectionActive ? selectionState.exit() : selectionState.enter('creators'))
    },
    {
      group: 'page',
      label: i18n.t('common.refresh'),
      icon: IconArrowClockwise,
      disabled: creatorsState.loading || creatorsState.syncing,
      onclick: () => void handleRefresh()
    }
  ]);

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
    haptic();
    const key = `${creator.service}:${creator.id}`;
    const select = () => selectionState.toggle('creators', key, creator, creatorsKeys, false, creatorsMap);
    if (isSelectionActive) select();
    else contextMenuState.show(event, creator.name || creator.id, creatorMenuItems(creator, select));
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

  function batchSubscribe() {
    const items = selectionState.getItems<Creator>();
    if (items.length === 0) return;
    subscriptionState.openDialog({
      service: items[0].service,
      creatorId: items[0].id,
      creatorName: items[0].name || items[0].id,
      batch: items.map((c) => ({ service: c.service, id: c.id, name: c.name || c.id }))
    });
    selectionState.exit();
  }

  function batchUnsubscribe() {
    const items = selectionState.getItems<Creator>();
    if (items.length === 0) return;
    const subs = items
      .map((c) => subscriptionState.forCreator(c.service, c.id))
      .filter((s): s is NonNullable<typeof s> => Boolean(s));
    if (subs.length === 0) return;
    subscriptionState.promptUnsubscribeBatch(subs);
    selectionState.exit();
  }

  async function batchFavoriteCreators(isFav: boolean) {
    const items = selectionState.getItems<any>();
    if (items.length === 0) return;
    try {
      for (const creator of items) {
        await apiSetCreatorFavorite(creator.service, creator.id, isFav);
      }
      notify.success(
        i18n.t(isFav ? 'selection.favorite' : 'selection.unfavorite'),
        i18n.t('counts.creators', { count: items.length })
      );
      await accountState.fetchFavorites('creator');
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('post.favorite_failed'), err);
    }
  }

  function toggleCreatorFavorite(event: MouseEvent, creator: Creator) {
    event.stopPropagation();
    event.preventDefault();
    void favoriteActions.toggleCreator(creator);
  }

  function handleSubscriptionClick(event: MouseEvent, creator: Creator) {
    event.stopPropagation();
    event.preventDefault();
    subscriptionState.openDialog({
      service: creator.service,
      creatorId: creator.id,
      creatorName: creator.name || creator.id,
      avatarUrl: creator.avatar_url
    });
  }

  function toggleProvider(providerId: string) {
    creatorsState.providerFilters = toggleFilterKey(creatorsState.providerFilters, providerId);
    void creatorsState.loadPage(true);
  }

  let enabledProviders = $derived(providerState.providers.filter((p) => p.enabled));
</script>

{#snippet filterInnerContent()}
  {#if enabledProviders.length > 1}
    <div class="filter-section-title">
      <div class="filter-section-icon"><IconGlobe class="w-3.5 h-3.5" /></div>
      <span>{i18n.t('providers.title')}</span>
    </div>
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
        </Button>
      {/each}
    </div>
  {/if}

  <div class="filter-section-title">
    <div class="filter-section-icon"><IconGlobe class="w-3.5 h-3.5" /></div>
    <span>{i18n.t('feed.platform')}</span>
  </div>
  <div class="service-options">
    <Button
      variant="ghost"
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
      </Button>
    {/each}
  </div>

  {#if !configState.settings.pawchive_hide_ai}
    {@const state = creatorsState.aiFilter}
    <div class="filter-section-title">
      <div class="filter-section-icon"><IconSparkle class="w-3.5 h-3.5" /></div>
      <span>{i18n.t('feed.format')}</span>
    </div>
    <div class="service-options">
      <Button
        variant="ghost"
        size="sm"
        onclick={() => {
          creatorsState.aiFilter = creatorsState.aiFilter === 'exclude' ? 'neutral' : 'exclude';
          void creatorsState.loadPage(true);
        }}
        class="filter-chip {state === 'exclude' ? 'state-include' : ''}"
        aria-pressed={state === 'exclude'}
      >
        <IconSparkle class="w-5 h-5" />
        <span>{i18n.t('feed.hide_ai')}</span>
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
    {#snippet addon()}
      <SortControl options={sortOptions} value={currentSortValue} onchange={handleSortChange} />
    {/snippet}
  </ChoiceGroup>
{/snippet}

{#snippet creatorsFilter()}
  <PopoverMenu
    bind:open={filtersOpen}
    title={i18n.t('feed.filters')}
    badge={activeFilterCount}
    active={activeFilterCount > 0}
    icon={IconOptions}
  >
    {@render filterInnerContent()}
  </PopoverMenu>
{/snippet}

{#snippet search()}
  <HeaderActions
    pinOpen
    bind:searchOpen
    bind:searchQuery={creatorsState.searchQuery}
    searchPlaceholder={i18n.t('creators.search_placeholder')}
  />
{/snippet}

<PageShell
  title={i18n.t('nav.creators')}
  menu={pageMenu}
  scrollKey={navigationState.entryKey}
  onrefresh={handleRefresh}
  onscroll={handleScroll}
  selectionTotal={creatorsList.length}
  onSelectAll={handleSelectAllCreators}
  {selectionActions}
>
  {#snippet views()}{@render creatorTabs()}{/snippet}
  {#snippet filter()}{@render creatorsFilter()}{/snippet}
  {#snippet actions()}{@render search()}{/snippet}

  {#if creatorsState.error && creatorsState.creators.length === 0}
    <EmptyState icon={IconWarning} tone="error" title={i18n.t('creators.load_error')} description={creatorsState.error}>
      <Button variant="tonal" onclick={() => void creatorsState.refresh()}>
        <IconArrowClockwise />
        <span>{i18n.t('common.retry')}</span>
      </Button>
    </EmptyState>
  {:else if creatorsList.length === 0 && !handoff.skeletonMounted}
    {#if activeTab === 'subscribed'}
      <EmptyState icon={IconPeople} title={i18n.t('subscriptions.empty')} description={i18n.t('subscriptions.empty_desc')} />
    {:else}
      <EmptyState icon={IconPeople} title={i18n.t('creators.empty')} description={i18n.t('creators.empty_desc')} />
    {/if}
  {:else}

    <div class="grid-stack">
      <div
        class="creators-grid"
        use:paintSelect={{ scope: 'creators', keys: creatorsKeys, items: creatorsMap }}
        {@attach ctrlWheel(gridScaleWheel)}
        style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-gap: ${gap}px;`}
      >
      {#each creatorsList as creator, index (creator.service + ':' + creator.id)}
        {@const creatorKey = `${creator.service}:${creator.id}`}
        {@const isSelected = isSelectionActive && selectionState.isSelected(creatorKey)}
        {@const placeholder = creatorPlaceholderUrl(creator)}
        {@const avatarSrc = creatorAvatarSrc(creator)}
        {@const isCreatorFav = accountState.isCreatorFavorite(creator.service, creator.id)}
        {@const creatorSub = subscriptionState.forCreator(creator.service, creator.id)}
        {@const favPending = favoriteActions.isCreatorPending(creator)}
        {@const enterDelay = handoff.delayFor(index)}
        <article
          class="grid-tile"
          class:selected={isSelected}
          class:is-entering={enterDelay !== null}
          style:aspect-ratio={ratio}
          style:content-visibility={index >= reveal.revealed ? 'hidden' : null}
          style:--tile-enter-delay={enterDelay !== null ? `${enterDelay}ms` : null}
          data-creator-key={creatorKey}
          data-select-key={creatorKey}
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
              aria-label={i18n.t('selection.select_creator')}
            >
              {#if isSelected}
                <IconCheckmark />
              {/if}
            </button>
          {:else}
            <div class="grid-tile-top-actions tile-toolbar">
              <div class="grid-tile-action-item" class:is-active={isCreatorFav}>
                <button
                  type="button"
                  class="grid-tile-action grid-tile-action-fav"
                  class:favorited={isCreatorFav}
                  disabled={favPending}
                  onclick={(e) => toggleCreatorFavorite(e, creator)}
                  use:ripple
                  use:tooltip={i18n.t(isCreatorFav ? 'post.unfavorite' : 'post.favorite')}
                  aria-label={i18n.t(isCreatorFav ? 'post.unfavorite' : 'post.favorite')}
                >
                  {#if favPending}
                    <IconLoading />
                  {:else if isCreatorFav}
                    <IconHeartFilled />
                  {:else}
                    <IconHeart />
                  {/if}
                </button>
              </div>

              <div class="grid-tile-action-item" class:is-active={Boolean(creatorSub)}>
                <button
                  type="button"
                  class="grid-tile-action"
                  class:is-selected={Boolean(creatorSub)}
                  onclick={(e) => handleSubscriptionClick(e, creator)}
                  use:ripple
                  use:tooltip={i18n.t(creatorSub ? 'subscriptions.subscription' : 'subscriptions.subscribe')}
                  aria-label={i18n.t(creatorSub ? 'subscriptions.subscription' : 'subscriptions.subscribe')}
                >
                  {#if creatorSub}
                    <IconPersonSubscribed />
                  {:else}
                    <IconPersonAdd />
                  {/if}
                </button>
              </div>
            </div>
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
              onload={onAvatarLoad}
              onerror={onAvatarError}
            />
          {/if}

          <div class="grid-tile-shade"></div>

          <div class="grid-tile-footer">
            <h2 class="grid-tile-title">{creator.name}</h2>

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

              <span class="grid-tile-author-name is-service">{creator.service}</span>
            </div>

            <div class="grid-tile-meta">
              <span>{creator.id}</span>
              <div class="grid-tile-meta-stats">
                {#if Number(creator.favorited ?? 0) > 0}
                  <span class="grid-tile-meta-row">
                    <IconHeartFilled />
                    {creator.favorited}
                  </span>
                {/if}
                <span>{formatDate(creator.updated)}</span>
              </div>
            </div>
          </div>
        </article>
      {/each}
      </div>
      {#if handoff.skeletonMounted}
        <SkeletonGrid
          cardWidth={targetCardWidth}
          {gap}
          fading={handoff.skeletonFading}
        />
      {/if}
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

{#snippet selectionActions()}
  <Button
    variant="ghost"
    class="btn-icon"
    onclick={batchSubscribe}
    title={i18n.t('selection.subscribe')}
  >
    <IconPersonAdd />
  </Button>

  {#if activeTab === 'subscribed'}
    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchUnsubscribe}
      title={i18n.t('selection.unsubscribe')}
    >
      <IconPersonDelete />
    </Button>
  {/if}

  <Button
    variant="ghost"
    class="btn-icon"
    onclick={() => void batchFavoriteCreators(true)}
    title={i18n.t('selection.favorite')}
  >
    <IconHeartFilled class="text-accent" />
  </Button>
{/snippet}

<style>
  :global(.creators-mode-choice) {
    flex-shrink: 0 !important;
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
    color: var(--text-muted);
  }

  .sentinel {
    display: grid;
    place-items: center;
    height: 60px;
    margin-top: 20px;
    color: var(--text-muted);
  }
</style>
