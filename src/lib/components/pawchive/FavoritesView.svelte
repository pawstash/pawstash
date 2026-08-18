<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { i18n } from '$lib/i18n';
  import { apiFetchCreatorArtworkDataUrl, apiSaveSettings } from '$lib/utils/ipc';
  import { creatorAvatarUrl } from '$lib/utils/media';
  import type { Creator, Favorite, PawchivePost } from '$lib/types/pawchive';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import PageHeader from '$lib/components/layout/PageHeader.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import PostGrid from '$lib/components/pawchive/PostGrid.svelte';
  import ServiceIcon from '$lib/components/pawchive/ServiceIcon.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { getPostDownloadTargets } from '$lib/utils/media';
  import { apiSetPostFavorite, apiSetCreatorFavorite } from '$lib/utils/ipc';
  import { notify } from '$lib/utils/toast';
  import SelectionActionBar from '$lib/components/ui/SelectionActionBar.svelte';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconHeart from '~icons/fluent/heart-24-regular';
  import IconHeartFilled from '~icons/fluent/heart-24-filled';
  import IconHeartOff from '~icons/fluent/heart-broken-24-regular';
  import IconOptions from '~icons/fluent/options-24-regular';
  import IconArrowSort from '~icons/fluent/arrow-sort-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconText from '~icons/fluent/document-text-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconPeople from '~icons/fluent/people-24-regular';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconBookmarkAdd from '~icons/fluent/bookmark-add-24-regular';
  import IconPersonAdd from '~icons/fluent/person-add-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  type FavoritesTab = 'posts' | 'creators';

  const savedState = navigationState.getViewState<{
    activeTab?: FavoritesTab;
    searchQuery?: string;
    searchOpen?: boolean;
    postSort?: string;
    creatorSort?: string;
    selectedPostServices?: string[];
    selectedCreatorServices?: string[];
    selectedFormats?: string[];
    onlyWithAttachments?: boolean;
  }>(navigationState.entryKey);

  let activeTab = $state<FavoritesTab>(savedState?.activeTab ?? 'posts');
  let isSelectionActive = $derived(
    selectionState.active &&
    ((activeTab === 'posts' && selectionState.scope === 'posts') ||
     (activeTab === 'creators' && selectionState.scope === 'creators'))
  );
  let selectedPosts = $derived(activeTab === 'posts' && isSelectionActive ? selectionState.getItems<PawchivePost>() : []);
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

  let loading = $state(false);
  let error = $state<string | null>(null);
  let searchOpen = $state(savedState?.searchOpen ?? Boolean(savedState?.searchQuery));
  let searchQuery = $state(savedState?.searchQuery ?? '');
  let postSort = $state(savedState?.postSort ?? 'favorite_desc');
  let creatorSort = $state(savedState?.creatorSort ?? 'favorite_desc');
  let selectedPostServices = $state<string[]>(savedState?.selectedPostServices ?? []);
  let selectedCreatorServices = $state<string[]>(savedState?.selectedCreatorServices ?? []);
  let selectedFormats = $state<string[]>(savedState?.selectedFormats ?? []);
  let onlyWithAttachments = $state<boolean>(savedState?.onlyWithAttachments ?? false);
  let filtersOpen = $state(false);
  let stickyFiltersOpen = $state(false);
  let initialized = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let avatarUrls = $state<Record<string, string>>({});

  $effect(() => {
    navigationState.saveViewState(navigationState.entryKey, {
      activeTab,
      searchQuery,
      searchOpen,
      postSort,
      creatorSort,
      selectedPostServices,
      selectedCreatorServices,
      selectedFormats,
      onlyWithAttachments
    });
  });

  let authenticated = $derived(accountState.session.authenticated);
  let posts = $derived((accountState.favoritePosts ?? []).map(mapFavoritePost));
  let creators = $derived((accountState.favoriteCreators ?? []).map(mapFavoriteCreator));
  let normalizedQuery = $derived(searchQuery.trim().toLowerCase());
  let availableServices = $derived([...new Set((activeTab === 'posts' ? posts : creators).map((item) => item.service).filter(Boolean))].sort());
  let currentServices = $derived(activeTab === 'posts' ? selectedPostServices : selectedCreatorServices);
  let activeFilterCount = $derived(currentServices.length + (activeTab === 'posts' ? selectedFormats.length + (onlyWithAttachments ? 1 : 0) : 0));
  let filteredPosts = $derived.by(() => {
    let filtered = normalizedQuery
      ? posts.filter((post) => `${post.title} ${post.user} ${post.service} ${post.id}`.toLowerCase().includes(normalizedQuery))
      : posts;
    if (selectedPostServices.length > 0) filtered = filtered.filter((post) => selectedPostServices.includes(post.service));
    if (selectedFormats.length > 0) filtered = filtered.filter((post) => selectedFormats.some((format) => matchesFormat(post, format)));
    if (onlyWithAttachments) filtered = filtered.filter((post) => Boolean(post.file || post.attachments?.length));
    return sortPosts(filtered, postSort);
  });
  let filteredCreators = $derived.by(() => {
    let filtered = normalizedQuery
      ? creators.filter((creator) => `${creator.name} ${creator.service} ${creator.id}`.toLowerCase().includes(normalizedQuery))
      : creators;
    if (selectedCreatorServices.length > 0) filtered = filtered.filter((creator) => selectedCreatorServices.includes(creator.service));
    return sortCreators(filtered, creatorSort);
  });
  let sortOptions = $derived(activeTab === 'posts'
    ? [
        { value: 'favorite_desc', label: i18n.t('favorites.sort_favorite_desc') },
        { value: 'favorite_asc', label: i18n.t('favorites.sort_favorite_asc') },
        { value: 'published_desc', label: i18n.t('favorites.sort_published_desc') },
        { value: 'published_asc', label: i18n.t('favorites.sort_published_asc') },
        { value: 'title_asc', label: i18n.t('favorites.sort_title_asc') },
        { value: 'title_desc', label: i18n.t('favorites.sort_title_desc') }
      ]
    : [
        { value: 'favorite_desc', label: i18n.t('favorites.sort_favorite_desc') },
        { value: 'favorite_asc', label: i18n.t('favorites.sort_favorite_asc') },
        { value: 'updated_desc', label: i18n.t('favorites.sort_updated_desc') },
        { value: 'updated_asc', label: i18n.t('favorites.sort_updated_asc') },
        { value: 'title_asc', label: i18n.t('favorites.sort_name_asc') },
        { value: 'title_desc', label: i18n.t('favorites.sort_name_desc') }
      ]);
  let currentSort = $derived(activeTab === 'posts' ? postSort : creatorSort);
  let currentCount = $derived(activeTab === 'posts' ? posts.length : creators.length);
  let baseCardWidth = $derived(layoutState.isMobile ? 155 : 245);
  let scale = $derived(configState.settings.grid_scale / 100);
  let gap = $derived(Math.round((layoutState.isMobile ? 8 : 10) * scale));
  let targetCardWidth = $derived(Math.round(baseCardWidth * scale));
  let ratio = $derived(({ square: '1 / 1', portrait: '4 / 5', landscape: '3 / 2', widescreen: '16 / 9' } as const)[configState.settings.grid_aspect_ratio]);

  function mapFavoritePost(favorite: Favorite): PawchivePost {
    return {
      ...favorite,
      id: String(favorite.id ?? ''),
      user: String(favorite.user ?? favorite.user_id ?? ''),
      service: String(favorite.service ?? ''),
      title: String(favorite.title ?? favorite.name ?? ''),
      content: String(favorite.content ?? ''),
      file: favorite.file as PawchivePost['file'],
      attachments: favorite.attachments as PawchivePost['attachments'],
      added: String(favorite.added ?? favorite.indexed ?? ''),
      published: String(favorite.published ?? favorite.updated ?? ''),
      favorite_count: Number(favorite.favorite_count ?? 0)
    };
  }

  function mapFavoriteCreator(favorite: Favorite): Creator {
    return {
      id: String(favorite.id ?? favorite.user ?? favorite.user_id ?? ''),
      name: String(favorite.name ?? favorite.id ?? ''),
      service: String(favorite.service ?? ''),
      public_id: favorite.public_id == null ? undefined : String(favorite.public_id),
      relation_id: favorite.relation_id == null ? undefined : String(favorite.relation_id),
      updated: timestampSeconds(favorite.updated),
      indexed: timestampSeconds(favorite.indexed),
      favorited: Number(favorite.favorited ?? favorite.favorite_count ?? 0) || undefined,
      faved_seq: favorite.faved_seq
    };
  }

  $effect(() => {
    for (const creator of creators) {
      const key = `${creator.service}:${creator.id}`;
      if (avatarUrls[key]) continue;
      void apiFetchCreatorArtworkDataUrl(creator.service, creator.id, 'avatar')
        .then((url) => avatarUrls = { ...avatarUrls, [key]: url })
        .catch(() => {});
    }
  });

  function favoriteOrder(item: PawchivePost | Creator) {
    return Number(item.faved_seq ?? 0);
  }

  function dateOrder(value: unknown) {
    if (!value) return 0;
    const numeric = Number(value);
    if (Number.isFinite(numeric) && numeric > 0) return numeric;
    const parsed = new Date(String(value)).getTime();
    return Number.isNaN(parsed) ? 0 : parsed;
  }

  function sortPosts(items: PawchivePost[], sort: string) {
    return [...items].sort((left, right) => {
      if (sort === 'favorite_desc') return favoriteOrder(right) - favoriteOrder(left);
      if (sort === 'favorite_asc') return favoriteOrder(left) - favoriteOrder(right);
      if (sort === 'published_desc') return dateOrder(right.published ?? right.added) - dateOrder(left.published ?? left.added);
      if (sort === 'published_asc') return dateOrder(left.published ?? left.added) - dateOrder(right.published ?? right.added);
      const comparison = left.title.localeCompare(right.title, undefined, { sensitivity: 'base', numeric: true });
      return sort === 'title_desc' ? -comparison : comparison;
    });
  }

  function sortCreators(items: Creator[], sort: string) {
    return [...items].sort((left, right) => {
      if (sort === 'favorite_desc') return favoriteOrder(right) - favoriteOrder(left);
      if (sort === 'favorite_asc') return favoriteOrder(left) - favoriteOrder(right);
      if (sort === 'updated_desc') return dateOrder(right.updated ?? right.indexed) - dateOrder(left.updated ?? left.indexed);
      if (sort === 'updated_asc') return dateOrder(left.updated ?? left.indexed) - dateOrder(right.updated ?? right.indexed);
      const comparison = left.name.localeCompare(right.name, undefined, { sensitivity: 'base', numeric: true });
      return sort === 'title_desc' ? -comparison : comparison;
    });
  }

  function matchesFormat(post: PawchivePost, format: string) {
    const files = [post.file, ...(post.attachments ?? [])].filter(Boolean);
    const names = files.map((file) => String(file?.name ?? file?.path ?? ''));
    const types = files.map((file) => String(file?.type ?? ''));
    const matches = (mime: string, extensions: RegExp) => types.some((type) => type.startsWith(`${mime}/`)) || names.some((name) => extensions.test(name));
    if (format === 'image') return matches('image', /\.(jpe?g|png|gif|webp|bmp|avif)$/i);
    if (format === 'video') return matches('video', /\.(mp4|mkv|webm|mov|avi|m4v)$/i) || Boolean(post.content?.includes('<video') || post.content?.includes('iframe'));
    if (format === 'audio') return matches('audio', /\.(mp3|wav|ogg|flac|m4a|aac)$/i);
    if (format === 'archive') return names.some((name) => /\.(zip|rar|7z|tar|gz)$/i.test(name));
    if (format === 'text') return names.some((name) => /\.(txt|md|pdf|doc|docx|epub)$/i.test(name));
    return false;
  }

  function handleSort(value: string) {
    if (activeTab === 'posts') postSort = value;
    else creatorSort = value;
  }

  function toggleService(service: string) {
    if (activeTab === 'posts') {
      selectedPostServices = selectedPostServices.includes(service)
        ? selectedPostServices.filter((item) => item !== service)
        : [...selectedPostServices, service];
    } else {
      selectedCreatorServices = selectedCreatorServices.includes(service)
        ? selectedCreatorServices.filter((item) => item !== service)
        : [...selectedCreatorServices, service];
    }
  }

  function clearServices() {
    if (activeTab === 'posts') selectedPostServices = [];
    else selectedCreatorServices = [];
  }

  function toggleFormat(format: string) {
    selectedFormats = selectedFormats.includes(format)
      ? selectedFormats.filter((item) => item !== format)
      : [...selectedFormats, format];
  }

  function resetFilters() {
    clearServices();
    if (activeTab === 'posts') {
      selectedFormats = [];
      onlyWithAttachments = false;
    }
  }



  function timestampSeconds(value: unknown) {
    if (!value) return undefined;
    const numeric = Number(value);
    if (Number.isFinite(numeric) && numeric > 0) return numeric > 10_000_000_000 ? Math.round(numeric / 1000) : numeric;
    const milliseconds = new Date(String(value)).getTime();
    return Number.isNaN(milliseconds) ? undefined : Math.round(milliseconds / 1000);
  }

  async function loadFavorites(force = false) {
    if (loading) return;
    loading = true;
    error = null;
    try {
      await Promise.all([
        accountState.fetchFavorites('post', force),
        accountState.fetchFavorites('creator', force)
      ]);
      initialized = true;
    } catch (reason) {
      error = reason instanceof Error ? reason.message : String(reason);
    } finally {
      loading = false;
    }
  }

  function selectTab(tab: FavoritesTab) {
    activeTab = tab;
    searchQuery = '';
    filtersOpen = false;
    stickyFiltersOpen = false;
    if (selectionState.active) selectionState.clear();
  }

  function handleSelectAll() {
    if (activeTab === 'posts') {
      selectionState.selectAll(filteredPosts.map((p) => ({
        key: `${p.service}:${p.user}:${p.id}`,
        item: p
      })));
    } else {
      selectionState.selectAll(filteredCreators.map((c) => ({
        key: `${c.service}:${c.id}`,
        item: c
      })));
    }
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

  async function batchUnfavoritePosts() {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0) return;
    try {
      for (const post of items) {
        await apiSetPostFavorite(post.service, post.user, post.id, false);
      }
      notify.success(
        i18n.t('selection.unfavorite') || 'Removed from favorites',
        `${items.length} ${items.length === 1 ? 'post' : 'posts'}`
      );
      await accountState.fetchFavorites('post', true);
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('post.favorite_failed') || 'Failed to update favorites', err);
    }
  }

  async function batchSubscribeCreators() {
    const items = selectionState.getItems<Creator>();
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

  async function batchUnfavoriteCreators() {
    const items = selectionState.getItems<Creator>();
    if (items.length === 0) return;
    try {
      for (const creator of items) {
        await apiSetCreatorFavorite(creator.service, creator.id, false);
      }
      notify.success(
        i18n.t('selection.unfavorite') || 'Removed from favorites',
        `${items.length} ${items.length === 1 ? 'creator' : 'creators'}`
      );
      await accountState.fetchFavorites('creator', true);
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('post.favorite_failed') || 'Failed to remove from favorites', err);
    }
  }

  let creatorKeys = $derived(filteredCreators.map((c) => `${c.service}:${c.id}`));
  let creatorsMap = $derived(new Map(filteredCreators.map((c) => [`${c.service}:${c.id}`, c])));

  $effect(() => {
    if (activeTab === 'creators') {
      selectionState.setContext('creators', creatorKeys, creatorsMap);
    }
  });

  function handleCreatorClick(event: MouseEvent, creator: Creator) {
    const key = `${creator.service}:${creator.id}`;
    if (event.ctrlKey || event.metaKey) {
      event.preventDefault();
      event.stopPropagation();
      selectionState.toggle('creators', key, creator, creatorKeys, false, creatorsMap);
      return;
    }

    if (isSelectionActive && selectionState.scope === 'creators') {
      event.preventDefault();
      event.stopPropagation();
      selectionState.toggle('creators', key, creator, creatorKeys, event.shiftKey, creatorsMap);
      return;
    }

    navigationState.openCreator(creator.service, creator.id);
  }

  function handleCreatorCheckbox(event: MouseEvent, creator: Creator) {
    event.stopPropagation();
    const key = `${creator.service}:${creator.id}`;
    selectionState.toggle('creators', key, creator, creatorKeys, event.shiftKey, creatorsMap);
  }

  function closeSearch() {
    searchOpen = false;
    searchQuery = '';
  }

  function getAvatarUrl(creator: Creator) {
    return avatarUrls[`${creator.service}:${creator.id}`] || creatorAvatarUrl(creator.service, creator.id);
  }

  function formatDate(value: unknown) {
    if (!value) return '';
    const numeric = Number(value);
    const date = Number.isFinite(numeric) && numeric > 0
      ? new Date(numeric < 10_000_000_000 ? numeric * 1000 : numeric)
      : new Date(String(value));
    return Number.isNaN(date.getTime()) ? '' : date.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }

  function handleCreatorWheel(event: WheelEvent) {
    if (!event.ctrlKey) return;
    event.preventDefault();
    const next = configState.settings.grid_scale + (event.deltaY < 0 ? 5 : -5);
    configState.settings.grid_scale = Math.max(60, Math.min(160, Math.round(next / 5) * 5));
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => void apiSaveSettings(configState.settings), 300);
  }

  onMount(() => {
    void loadFavorites();
  });

  $effect(() => {
    if (!initialized) void loadFavorites();
  });

  $effect(() => {
    if (!searchOpen) return;
    const input = document.querySelector('.favorites-search-input') as HTMLInputElement | null;
    input?.focus();
  });
</script>

{#snippet favoriteTabs()}
  <nav class="favorites-tabs" aria-label={i18n.t('favorites.title')}>
    <Button variant={activeTab === 'posts' ? 'accent' : 'ghost'} onclick={() => selectTab('posts')}>
      <IconHeart class="tab-icon" />
      <span>{i18n.t('favorites.posts')}</span>
      {#if posts.length > 0}<span class="tab-count">{posts.length}</span>{/if}
    </Button>
    <Button variant={activeTab === 'creators' ? 'accent' : 'ghost'} onclick={() => selectTab('creators')}>
      <IconPeople class="tab-icon" />
      <span>{i18n.t('favorites.creators')}</span>
      {#if creators.length > 0}<span class="tab-count">{creators.length}</span>{/if}
    </Button>
  </nav>
{/snippet}

{#snippet filterContent()}
  <div class="filter-heading">
    <strong>{i18n.t('feed.filters')}</strong>
    {#if activeFilterCount > 0}<button type="button" onclick={resetFilters}>{i18n.t('feed.reset_filters')}</button>{/if}
  </div>
  <span class="filter-label">{i18n.t('feed.platform')}</span>
  <div class="filter-options">
    <Button variant={currentServices.length === 0 ? 'accent' : 'ghost'} size="sm" onclick={clearServices} class="filter-chip">
      <IconGlobe /><span>{i18n.t('feed.all_platforms')}</span>
    </Button>
    {#each availableServices as service}
      <Button variant={currentServices.includes(service) ? 'accent' : 'ghost'} size="sm" onclick={() => toggleService(service)} class="filter-chip">
        <ServiceIcon {service} /><span>{service}</span>
      </Button>
    {/each}
  </div>
  {#if activeTab === 'posts'}
    <span class="filter-label">{i18n.t('feed.format')}</span>
    <div class="filter-options">
      <Button variant={selectedFormats.includes('image') ? 'accent' : 'ghost'} size="sm" onclick={() => toggleFormat('image')} class="filter-chip"><IconImage /><span>{i18n.t('feed.format_photo')}</span></Button>
      <Button variant={selectedFormats.includes('video') ? 'accent' : 'ghost'} size="sm" onclick={() => toggleFormat('video')} class="filter-chip"><IconVideo /><span>{i18n.t('feed.format_video')}</span></Button>
      <Button variant={selectedFormats.includes('audio') ? 'accent' : 'ghost'} size="sm" onclick={() => toggleFormat('audio')} class="filter-chip"><IconMusic /><span>{i18n.t('feed.format_audio')}</span></Button>
      <Button variant={selectedFormats.includes('text') ? 'accent' : 'ghost'} size="sm" onclick={() => toggleFormat('text')} class="filter-chip"><IconText /><span>{i18n.t('feed.format_text')}</span></Button>
      <Button variant={selectedFormats.includes('archive') ? 'accent' : 'ghost'} size="sm" onclick={() => toggleFormat('archive')} class="filter-chip"><IconDocument /><span>{i18n.t('feed.format_archive')}</span></Button>
    </div>
    <span class="filter-label section-label">{i18n.t('feed.filters')}</span>
    <div class="view-option" class:active={onlyWithAttachments}>
      <Checkbox checked={onlyWithAttachments} onchange={(value) => onlyWithAttachments = value} />
      <button onclick={() => onlyWithAttachments = !onlyWithAttachments}>
        <strong>{i18n.t('feed.with_attachments')}</strong>
        <small>{i18n.t('feed.with_attachments_desc')}</small>
      </button>
      <IconDocument class="view-option-icon w-[20px] h-[20px]" />
    </div>
  {/if}
{/snippet}

{#snippet actionsCluster(sticky = false)}
  <HeaderActions
    bind:searchOpen
    bind:searchQuery
    searchPlaceholder={activeTab === 'posts' ? i18n.t('feed.search_placeholder') : i18n.t('creators.search_placeholder')}
  >
    <Button
      variant={isSelectionActive ? 'accent' : 'ghost'}
      class="btn-icon"
      onclick={() => (isSelectionActive ? selectionState.exit() : selectionState.enter(activeTab === 'posts' ? 'posts' : 'creators'))}
      title={i18n.t('selection.select_mode') || 'Select mode'}
      aria-label="Select mode"
    >
      <IconCheckboxChecked class="w-5 h-5" />
    </Button>
    <Button variant="ghost" class="btn-icon" disabled={loading} onclick={() => void loadFavorites(true)} title={i18n.t('feed.refresh')} aria-label={i18n.t('feed.refresh')}>
      {#if loading}<IconLoading class="w-5 h-5" />{:else}<IconArrowClockwise class="w-5 h-5" />{/if}
    </Button>
    {#if sticky}
      <PopoverMenu
        bind:open={stickyFiltersOpen}
        title={i18n.t('feed.filters')}
        badge={activeFilterCount}
        active={activeFilterCount > 0}
        icon={IconOptions}
      >
        {@render filterContent()}
      </PopoverMenu>
    {:else}
      <PopoverMenu
        bind:open={filtersOpen}
        title={i18n.t('feed.filters')}
        badge={activeFilterCount}
        active={activeFilterCount > 0}
        icon={IconOptions}
      >
        {@render filterContent()}
      </PopoverMenu>
    {/if}
  </HeaderActions>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey} onrefresh={() => accountState.refresh()}>
  {#snippet overlay()}
    <StickyHeader threshold={120} title={i18n.t('favorites.title') || 'Favorites'}>
      {#snippet center()}
        <div class="flex items-center gap-2">
          {@render favoriteTabs()}
          <Select
            variant="ghost"
            options={sortOptions}
            value={currentSort}
            onchange={handleSort}
            class="favorites-sort"
            icon={IconArrowSort}
            iconOnly={layoutState.isMobile}
            ariaLabel={i18n.t('favorites.sort_by') || 'Sort'}
          />
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
        {@render favoriteTabs()}
        <Select
          variant="ghost"
          options={sortOptions}
          value={currentSort}
          onchange={handleSort}
          class="favorites-sort"
          icon={IconArrowSort}
          iconOnly={layoutState.isMobile}
          ariaLabel={i18n.t('favorites.sort_by') || 'Sort'}
        />
      </div>
    {/snippet}
    {#snippet actions()}
      {@render actionsCluster(false)}
    {/snippet}
  </PageHeader>

  {#if loading && currentCount === 0}
    <div class="status-container"><IconLoading class="spinner" /><span>{i18n.t('feed.loading')}</span></div>
  {:else if error && currentCount === 0}
    <div class="status-container error">
      <strong>{i18n.t('favorites.load_error')}</strong>
      <span>{error}</span>
      <Button variant="accent" size="sm" onclick={() => void loadFavorites(true)}><IconArrowClockwise /> {i18n.t('feed.retry')}</Button>
    </div>
  {:else if activeTab === 'posts'}
    <PostGrid
      posts={filteredPosts}
      loading={loading}
      stateKey={`favorites:posts:${searchQuery}:services=${selectedPostServices.join(',')}:formats=${selectedFormats.join(',')}:attachments=${onlyWithAttachments}:sort=${postSort}`}
      ariaLabel={i18n.t('favorites.posts')}
      emptyTitle={searchQuery || activeFilterCount > 0 ? i18n.t('favorites.no_results') : i18n.t('favorites.empty_posts')}
      emptyDescription={searchQuery || activeFilterCount > 0 ? i18n.t('favorites.no_results_desc') : (!authenticated ? i18n.t('favorites.empty_posts_guest') : i18n.t('favorites.empty_posts_desc'))}
    />
  {:else if filteredCreators.length === 0}
    <div class="status-container empty">
      <IconPeople class="status-icon" />
      <strong>{searchQuery || activeFilterCount > 0 ? i18n.t('favorites.no_results') : i18n.t('favorites.empty_creators')}</strong>
      <span>{searchQuery || activeFilterCount > 0 ? i18n.t('favorites.no_results_desc') : (!authenticated ? i18n.t('favorites.empty_creators_guest') : i18n.t('favorites.empty_creators_desc'))}</span>
    </div>
  {:else}
    <div
      class="creators-grid"
      onwheel={handleCreatorWheel}
      style={`--favorites-card-width: ${targetCardWidth}px; --favorites-gap: ${gap}px;`}
    >
      {#each filteredCreators as creator (creator.service + ':' + creator.id)}
        {@const creatorKey = `${creator.service}:${creator.id}`}
        {@const isSelected = isSelectionActive && selectionState.isSelected(creatorKey)}
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

          <div class="grid-tile-placeholder"><span class="fallback-initials">{creator.name.slice(0, 2).toUpperCase()}</span></div>
          <img class="grid-tile-media" src={getAvatarUrl(creator)} alt="" loading="lazy" decoding="async" onerror={(event) => ((event.currentTarget as HTMLImageElement).style.display = 'none')} />
          <div class="grid-tile-shade"></div>
          <div class="grid-tile-footer">
            <div class="grid-tile-author">
              <span class="grid-tile-logo inline-logo"><ServiceIcon service={creator.service} /></span>
              <span
                role="link"
                tabindex="0"
                class="grid-tile-author-name"
                onclick={(e) => handleCreatorClick(e, creator)}
                onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleCreatorClick(e as any, creator)}
              >{creator.name}</span>
            </div>
            <div class="grid-tile-meta">
              <span>{creator.service} · {creator.id}</span>
              <div class="grid-tile-meta-stats">
                <span class="grid-tile-meta-row"><IconHeartFilled />{i18n.t('favorites.saved')}</span>
                <span>{formatDate(creator.updated ?? creator.indexed)}</span>
              </div>
            </div>
          </div>
        </article>
      {/each}
    </div>
  {/if}

  {#if error && currentCount > 0}<p class="inline-error">{error}</p>{/if}
</PageShell>

<SelectionActionBar
  totalCount={activeTab === 'posts' ? filteredPosts.length : filteredCreators.length}
  onSelectAll={handleSelectAll}
>
  {#if activeTab === 'posts'}
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
      variant="danger"
      size="sm"
      class="selection-btn"
      onclick={batchUnfavoritePosts}
      title={i18n.t('selection.unfavorite')}
    >
      <IconHeartOff class="w-[16px] h-[16px]" />
      <span>{i18n.t('selection.unfavorite')}</span>
    </Button>
  {:else}
    <Button
      variant="ghost"
      size="sm"
      class="selection-btn"
      onclick={batchSubscribeCreators}
      title={i18n.t('selection.subscribe')}
    >
      <IconPersonAdd class="w-[16px] h-[16px]" />
      <span>{i18n.t('selection.subscribe')}</span>
    </Button>

    <Button
      variant="danger"
      size="sm"
      class="selection-btn"
      onclick={batchUnfavoriteCreators}
      title={i18n.t('selection.unfavorite')}
    >
      <IconHeartOff class="w-[16px] h-[16px]" />
      <span>{i18n.t('selection.unfavorite')}</span>
    </Button>
  {/if}
</SelectionActionBar>

<style>
  .favorites-tabs {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  :global(.favorites-tabs .btn) {
    height: 44px !important;
    padding: 0 18px !important;
    border-radius: var(--radius-full) !important;
    font-size: 13.5px !important;
    gap: 8px !important;
  }

  :global(.tab-icon) { width: 18px; height: 18px; }

  :global(.btn-icon) {
    width: 44px !important;
    height: 44px !important;
    padding: 0 !important;
    border-radius: var(--radius-full) !important;
    flex-shrink: 0;
  }

  :global(.btn-icon svg) { width: 20px !important; height: 20px !important; }

  :global(.favorites-sort) { height: 44px !important; width: auto !important; }
  :global(.favorites-sort:not(:has(.icon-only))) { min-width: 150px; }
  :global(.favorites-sort .select-trigger) { height: 44px !important; border-radius: var(--radius-full) !important; font-size: 13px !important; }

  .status-container {
    min-height: 310px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    text-align: center;
    color: rgba(255, 255, 255, 0.42);
  }

  .status-container strong {
    color: rgba(255, 255, 255, 0.76);
    font-size: 14px;
    font-weight: 600;
  }

  .status-container span {
    max-width: 360px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.42);
    line-height: 1.5;
  }

  .status-container.error span, .inline-error { color: #fca5a5; }
  :global(.status-icon) { width: 34px; height: 34px; color: rgba(255, 255, 255, 0.42); margin-bottom: 5px; }
  :global(.spinner) { width: 28px; height: 28px; }

  .creators-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--favorites-card-width)), 1fr));
    gap: var(--favorites-gap);
    align-items: start;
  }

  .fallback-initials { font-size: 26px; font-weight: 700; color: rgba(255,255,255,.35); }
  .inline-error { margin-top: 20px; text-align: center; font-size: 12px; }
</style>
