<script lang="ts">
  import { haptic } from '$lib/utils/haptics';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import { paintSelect } from '$lib/actions/paintSelect';
  import { onMount, onDestroy, getContext } from 'svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { favoriteActions } from '$lib/state/favoriteActions.svelte';
  import { contextMenuState } from '$lib/state/contextMenuState.svelte';
  import { creatorMenuItems } from '$lib/utils/contextMenus';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { contentState, postCacheKey } from '$lib/state/contentState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { i18n } from '$lib/i18n';
  import { gridScaleWheel } from '$lib/state/scaleState.svelte';
  import { formatDate } from '$lib/utils/formatters';
  import { creatorAvatarSrc, creatorPlaceholderUrl, onAvatarError, onAvatarLoad } from '$lib/utils/media';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import type { Creator, Favorite, Post } from '$lib/types/content';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import PostGrid from './PostGrid.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import ChoiceGroup, { type ChoiceOption } from '$lib/components/ui/ChoiceGroup.svelte';
  import SortControl from '$lib/components/layout/SortControl.svelte';
  import { sortOption } from '$lib/utils/sort';
  import type { MenuItem } from '$lib/components/layout/OverflowMenu.svelte';
  import SkeletonGrid from '$lib/components/ui/SkeletonGrid.svelte';
  import { GridHandoff } from '$lib/motion/gridHandoff.svelte';
  import { progressiveReveal } from '$lib/utils/progressiveReveal.svelte';
  import { ripple, tooltip } from '$lib/motion';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { apiSetPostFavorite, apiSetCreatorFavorite } from '$lib/utils/ipc';
  import { notify } from '$lib/utils/toast';
  import { notifyAddedToStash, notifyRemovedFromStash } from '$lib/utils/stashToast';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconWarning from '~icons/fluent/warning-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconHeart from '~icons/fluent/heart-24-regular';
  import IconHeartFilled from '~icons/fluent/heart-24-filled';
  import IconHeartOff from '~icons/fluent/heart-broken-24-regular';
  import IconOptions from '~icons/fluent/options-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconText from '~icons/fluent/document-text-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconDraft from '~icons/fluent/drafts-24-regular';
  import IconPeople from '~icons/fluent/people-24-regular';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-filled';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconBookmarkAdd from '~icons/fluent/bookmark-add-24-regular';
  import IconPersonAdd from '~icons/fluent/person-add-24-regular';
  import IconPersonSubscribed from '~icons/fluent/person-available-24-filled';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  import type { FilterMap } from '$lib/types/filter';
  import { countActiveFilters, matchesTriStateFilter, toggleFilterKey } from '$lib/types/filter';
  import { matchesPostFormatFilter } from '$lib/utils/media';
  import { extraField } from '$lib/utils/fields';
  import { parseTags } from '$lib/utils/formatters';
  import { ctrlWheel } from '$lib/utils/ctrlWheel';

  type FavoritesTab = 'posts' | 'creators';

  const savedState = navigationState.getViewState<{
    activeTab?: FavoritesTab;
    searchQuery?: string;
    searchOpen?: boolean;
    postSort?: string;
    creatorSort?: string;
    postServiceFilters?: FilterMap;
    creatorServiceFilters?: FilterMap;
    formatFilters?: FilterMap;
    onlyWithAttachments?: boolean;
  }>(navigationState.entryKey);

  let activeTab = $state<FavoritesTab>(savedState?.activeTab ?? 'posts');
  let isSelectionActive = $derived(
    selectionState.active &&
    ((activeTab === 'posts' && selectionState.scope === 'posts') ||
     (activeTab === 'creators' && selectionState.scope === 'creators'))
  );
  let selectedPosts = $derived(activeTab === 'posts' && isSelectionActive ? selectionState.getItems<Post>() : []);
  const stashOptions = $derived(libraryState.stashOptions);

  const batchSelectedStashes = $derived(libraryState.stashesForPosts(selectedPosts));

  async function handleBatchToggleStash(collectionId: string) {
    const items = selectionState.getItems<Post>();
    if (items.length === 0 || !collectionId) return;
    const isAllIn = batchSelectedStashes.includes(collectionId);
    try {
      if (isAllIn) {
        for (const p of items) {
          await libraryState.removeFromStash(collectionId, p);
        }
        notifyRemovedFromStash(collectionId, items);
      } else {
        for (const p of items) {
          await libraryState.save(p, collectionId);
        }
        notifyAddedToStash(collectionId);
      }
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
    }
  }

  async function handleBatchCreateAndAddToStash(name: string) {
    const items = selectionState.getItems<Post>();
    if (items.length === 0 || !name.trim()) return;
    try {
      const newStash = await libraryState.createStash(name.trim());
      for (const p of items) {
        await libraryState.save(p, newStash.id);
      }
      notifyAddedToStash(newStash.id, newStash.name);
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
    }
  }

  let loading = $state(false);
  let error = $state<string | null>(null);
  let searchOpen = $state(savedState?.searchOpen ?? Boolean(savedState?.searchQuery));
  let searchQuery = $state(savedState?.searchQuery ?? '');
  let postSort = $state(savedState?.postSort ?? 'favorite_desc');
  let creatorSort = $state(savedState?.creatorSort ?? 'favorite_desc');
  let postServiceFilters = $state<FilterMap>(savedState?.postServiceFilters ?? {});
  let creatorServiceFilters = $state<FilterMap>(savedState?.creatorServiceFilters ?? {});
  let formatFilters = $state<FilterMap>(savedState?.formatFilters ?? {});
  let onlyWithAttachments = $state<boolean>(savedState?.onlyWithAttachments ?? false);
  let filtersOpen = $state(false);
  let initialized = $state(false);
  let visibleCreatorsCount = $state(80);
  const scrollContext = getContext<ScrollableContext | undefined>(SCROLLABLE_CONTEXT);

  $effect(() => {
    navigationState.saveViewState(navigationState.entryKey, {
      activeTab,
      searchQuery,
      searchOpen,
      postSort,
      creatorSort,
      postServiceFilters: $state.snapshot(postServiceFilters),
      creatorServiceFilters: $state.snapshot(creatorServiceFilters),
      formatFilters: $state.snapshot(formatFilters),
      onlyWithAttachments
    });
  });

  let authenticated = $derived(accountState.session.authenticated);
  let posts = $derived((accountState.favoritePosts ?? []).map(mapFavoritePost));
  let creators = $derived((accountState.favoriteCreators ?? []).map(mapFavoriteCreator));
  let normalizedQuery = $derived(searchQuery.trim().toLowerCase());
  let availableServices = $derived([...new Set((activeTab === 'posts' ? posts : creators).map((item) => item.service).filter(Boolean))].sort());
  let currentServices = $derived(activeTab === 'posts' ? postServiceFilters : creatorServiceFilters);
  let activeFilterCount = $derived(
    activeTab === 'posts'
      ? countActiveFilters([postServiceFilters, formatFilters]) + (onlyWithAttachments ? 1 : 0)
      : countActiveFilters([creatorServiceFilters])
  );
  let filteredPosts = $derived.by(() => {
    let filtered = normalizedQuery
      ? posts.filter((post) => `${post.title} ${post.user} ${post.service} ${post.id}`.toLowerCase().includes(normalizedQuery))
      : posts;
    if (configState.settings.pawchive_hide_ai) {
      filtered = filtered.filter((post) => {
        const postTags = parseTags(post.tags);
        const isAi = Boolean(
          postTags.some((t) => {
            const l = t.toLowerCase();
            return l === 'ai' || l.includes('ai generated') || l.includes('artificial intelligence');
          }) ||
          post.title?.toLowerCase().includes('[ai]') ||
          post.title?.toLowerCase().includes('(ai)')
        );
        return !isAi;
      });
    }
    if (Object.keys(postServiceFilters).length > 0) {
      filtered = filtered.filter((post) => matchesTriStateFilter([post.service], postServiceFilters));
    }
    if (Object.keys(formatFilters).length > 0) {
      filtered = filtered.filter((post) => matchesPostFormatFilter(post, formatFilters));
    }
    if (onlyWithAttachments) {
      filtered = filtered.filter((post) => Boolean(post.file || post.attachments?.length));
    }
    return sortPosts(filtered, postSort);
  });

  const enrichingPostKeys = new Set<string>();
  $effect(() => {
    if (activeTab !== 'posts' || loading) return;
    const candidates = filteredPosts.filter((p) => {
      if (!p.id || !p.service || !p.user) return false;
      const key = `${p.service}:${p.user}:${p.id}`;
      if (enrichingPostKeys.has(key)) return false;
      const hasMedia = Boolean(
        p.file ||
        (p.attachments && p.attachments.length > 0) ||
        p.thumbnail_url ||
        p.media_url ||
        p.preview_path ||
        extraField(p, 'local_preview_path')
      );
      return !hasMedia;
    });

    if (candidates.length === 0) return;

    const batch = candidates.slice(0, 6);
    for (const post of batch) {
      const key = `${post.service}:${post.user}:${post.id}`;
      enrichingPostKeys.add(key);
      contentState.enqueueDetailPrefetch(post.service, post.user, post.id);
    }
  });

  const resolvingCreatorKeys = new Set<string>();
  $effect(() => {
    if (activeTab !== 'creators' || loading) return;
    const candidates = filteredCreators.filter((c) => {
      if (!c.id || !c.service) return false;
      const key = `${c.service.toLowerCase()}:${c.id.toLowerCase()}`;
      if (resolvingCreatorKeys.has(key)) return false;
      return c.name === c.id;
    });

    if (candidates.length === 0) return;

    const batch = candidates.slice(0, 10);
    for (const creator of batch) {
      const key = `${creator.service.toLowerCase()}:${creator.id.toLowerCase()}`;
      resolvingCreatorKeys.add(key);
      void creatorsState.resolveName(creator.service, creator.id);
    }
  });
  let filteredCreators = $derived.by(() => {
    let filtered = normalizedQuery
      ? creators.filter((creator) => `${creator.name} ${creator.service} ${creator.id}`.toLowerCase().includes(normalizedQuery))
      : creators;
    if (configState.settings.pawchive_hide_ai) {
      filtered = filtered.filter((creator) => {
        const isAi = Boolean(
          (creator.extra as any)?.tags?.some((t: string) => t.toLowerCase() === 'ai' || t.toLowerCase().includes('ai generated')) ||
          creator.name.toLowerCase().includes('[ai]') ||
          creator.name.toLowerCase().includes('(ai)')
        );
        return !isAi;
      });
    }
    if (Object.keys(creatorServiceFilters).length > 0) {
      filtered = filtered.filter((creator) => matchesTriStateFilter([creator.service], creatorServiceFilters));
    }
    return sortCreators(filtered, creatorSort);
  });

  const creatorsHandoff = new GridHandoff();
  $effect(() => creatorsHandoff.sync(filteredCreators.length > 0, loading));
  onDestroy(() => creatorsHandoff.destroy());

  let visibleCreators = $derived(filteredCreators.slice(0, visibleCreatorsCount));
  let hasMoreCreators = $derived(filteredCreators.length > visibleCreatorsCount);
  const reveal = progressiveReveal(() => visibleCreators.length);

  function loadMoreCreators() {
    if (!hasMoreCreators) return;
    visibleCreatorsCount += 80;
  }

  function creatorSentinel(node: HTMLElement) {
    const root = scrollContext?.viewport ?? null;
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          loadMoreCreators();
        }
      },
      { root, rootMargin: '1000px 0px' }
    );
    io.observe(node);

    return {
      destroy() {
        io.disconnect();
      }
    };
  }

  $effect(() => {
    normalizedQuery;
    creatorSort;
    creatorServiceFilters;
    visibleCreatorsCount = 80;
  });
  let sortOptions = $derived(activeTab === 'posts'
    ? [
        sortOption('favorite_desc', 'favorited', 'desc'),
        sortOption('favorite_asc', 'favorited', 'asc'),
        sortOption('published_desc', 'published', 'desc'),
        sortOption('published_asc', 'published', 'asc'),
        sortOption('title_asc', 'title', 'asc'),
        sortOption('title_desc', 'title', 'desc')
      ]
    : [
        sortOption('favorite_desc', 'favorited', 'desc'),
        sortOption('favorite_asc', 'favorited', 'asc'),
        sortOption('updated_desc', 'updated', 'desc'),
        sortOption('updated_asc', 'updated', 'asc'),
        sortOption('title_asc', 'name', 'asc'),
        sortOption('title_desc', 'name', 'desc')
      ]);
  let currentSort = $derived(activeTab === 'posts' ? postSort : creatorSort);
  let favoriteTabOptions = $derived<ChoiceOption<'posts' | 'creators'>[]>([
    {
      value: 'posts',
      label: i18n.t('favorites.posts'),
      count: posts.length
    },
    {
      value: 'creators',
      label: i18n.t('favorites.creators'),
      count: creators.length
    }
  ]);
  let currentCount = $derived(activeTab === 'posts' ? posts.length : creators.length);
  let baseCardWidth = $derived(layoutState.isMobile ? 155 : 245);
  let scale = $derived(configState.settings.grid_scale / 100);
  let gap = $derived(Math.round((layoutState.isMobile ? 8 : 10) * scale));
  let targetCardWidth = $derived(Math.round(baseCardWidth * scale));
  let ratio = $derived(({ square: '1 / 1', portrait: '4 / 5', landscape: '3 / 2', widescreen: '16 / 9' } as const)[configState.settings.grid_aspect_ratio]);

  function mapFavoritePost(favorite: Favorite, index = 0): Post {
    const rawExtra = (favorite.extra || {}) as any;
    const srv = String(favorite.service ?? rawExtra.service ?? '');
    const user = String(favorite.user ?? favorite.user_id ?? rawExtra.user ?? rawExtra.user_id ?? '');
    const id = String(favorite.id ?? rawExtra.id ?? '');

    const key = postCacheKey(srv, user, id);
    const cached = contentState.posts[key]?.post;

    return {
      ...favorite,
      id,
      user: cached?.user || user,
      service: cached?.service || srv,
      title: cached?.title || String(favorite.title ?? favorite.name ?? rawExtra.title ?? ''),
      content: cached?.content || String(favorite.content ?? rawExtra.content ?? ''),
      file: cached?.file || (favorite.file ?? rawExtra.file) as Post['file'],
      attachments: cached?.attachments || (favorite.attachments ?? rawExtra.attachments) as Post['attachments'],
      thumbnail_url: cached?.thumbnail_url || favorite.thumbnail_url || rawExtra.thumbnail_url,
      media_url: cached?.media_url || favorite.media_url || rawExtra.media_url,
      preview_path: cached?.preview_path || favorite.preview_path || rawExtra.preview_path,
      added: cached?.added || String(favorite.added ?? favorite.indexed ?? rawExtra.added ?? ''),
      published: cached?.published || String(favorite.published ?? favorite.updated ?? rawExtra.published ?? ''),
      favorite_count: cached?.favorite_count ?? Number(favorite.favorite_count ?? rawExtra.favorite_count ?? 0),
      faved_seq: typeof favorite.faved_seq === 'number' ? favorite.faved_seq : undefined,
      faved_at: String(favorite.faved_at ?? rawExtra.faved_at ?? favorite.created_at ?? '') || undefined,
      extra: {
        ...rawExtra,
        ...(cached?.extra || {}),
        ...(rawExtra.local_preview_path ? { local_preview_path: rawExtra.local_preview_path } : {})
      }
    };
  }

  function timestampSeconds(value: unknown) {
    if (!value) return undefined;
    const numeric = Number(value);
    if (Number.isFinite(numeric) && numeric > 0) return numeric > 10_000_000_000 ? Math.round(numeric / 1000) : numeric;
    const milliseconds = new Date(String(value)).getTime();
    return Number.isNaN(milliseconds) ? undefined : Math.round(milliseconds / 1000);
  }

  function mapFavoriteCreator(favorite: Favorite): Creator {
    const extra = (favorite.extra || {}) as Record<string, unknown>;
    const asStr = (val: unknown) => (typeof val === 'string' && val ? val : undefined);

    const favId = String(favorite.id ?? favorite.user ?? favorite.user_id ?? extra.id ?? '');
    const favService = String(favorite.service ?? extra.service ?? '');
    const cachedName = creatorsState.creatorsMap.get(`${favService.toLowerCase()}:${favId.toLowerCase()}`);
    const rawName = String(favorite.name ?? extra.name ?? '');
    const name = rawName && rawName !== favId ? rawName : (cachedName || favId);

    return {
      id: favId,
      name,
      service: favService,
      public_id: favorite.public_id == null ? undefined : String(favorite.public_id),
      relation_id: favorite.relation_id == null ? undefined : String(favorite.relation_id),
      updated: timestampSeconds(favorite.updated ?? extra.updated),
      indexed: timestampSeconds(favorite.indexed ?? extra.indexed),
      favorited: Number(favorite.favorited ?? favorite.favorite_count ?? extra.favorited ?? extra.favorite_count ?? 0) || undefined,
      faved_seq: favorite.faved_seq,
      faved_at: asStr(favorite.faved_at ?? extra.faved_at ?? favorite.created_at),
      avatar_url: asStr(favorite.avatar_url ?? extra.avatar_url),
      avatar_path: asStr(favorite.avatar_path ?? extra.avatar_path),
      banner_url: asStr(favorite.banner_url ?? extra.banner_url),
      banner_path: asStr(favorite.banner_path ?? extra.banner_path),
      avatar_thumbhash: asStr(favorite.avatar_thumbhash ?? extra.avatar_thumbhash),
      banner_thumbhash: asStr(favorite.banner_thumbhash ?? extra.banner_thumbhash),
      extra
    };
  }

  function favoriteOrder(item: Post | Creator) {
    const favedAt = (item as any).faved_at || (item as any).created_at || (item as any).faved_date;
    if (favedAt) {
      const parsed = dateOrder(favedAt);
      if (parsed > 0) return parsed;
    }
    if (typeof item.faved_seq === 'number' && Number.isFinite(item.faved_seq)) {
      return item.faved_seq;
    }
    const fallback = (item as any).published || (item as any).updated || (item as any).added || (item as any).indexed;
    if (fallback) {
      const parsed = dateOrder(fallback);
      if (parsed > 0) return parsed;
    }
    return 0;
  }

  function dateOrder(value: unknown) {
    if (!value) return 0;
    const numeric = Number(value);
    if (Number.isFinite(numeric) && numeric > 0) return numeric;
    const parsed = new Date(String(value)).getTime();
    return Number.isNaN(parsed) ? 0 : parsed;
  }

  function sortPosts(items: Post[], sort: string) {
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

  function handleSort(value: string) {
    if (activeTab === 'posts') postSort = value;
    else creatorSort = value;
  }

  function toggleService(service: string) {
    if (activeTab === 'posts') {
      postServiceFilters = toggleFilterKey(postServiceFilters, service);
    } else {
      creatorServiceFilters = toggleFilterKey(creatorServiceFilters, service);
    }
  }

  function clearServices() {
    if (activeTab === 'posts') postServiceFilters = {};
    else creatorServiceFilters = {};
  }

  function toggleFormat(format: string) {
    formatFilters = toggleFilterKey(formatFilters, format);
  }

  const formatList = [
    { id: 'image', label: () => i18n.t('feed.format_photo'), icon: IconImage },
    { id: 'video', label: () => i18n.t('feed.format_video'), icon: IconVideo },
    { id: 'audio', label: () => i18n.t('feed.format_audio'), icon: IconMusic },
    { id: 'text', label: () => i18n.t('feed.format_text'), icon: IconText },
    { id: 'archive', label: () => i18n.t('feed.format_archive'), icon: IconDocument },
    { id: 'wip', label: () => i18n.t('feed.format_wip'), icon: IconDraft }
  ];

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
    visibleCreatorsCount = 80;
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
    const items = selectionState.getItems<Post>();
    if (items.length === 0) return;
    try {
      for (const post of items) {
        await libraryState.save(post);
      }
      notify.success(
        i18n.t('selection.save_to_library'),
        i18n.t('counts.posts', { count: items.length })
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('library.save_error'), err);
    }
  }

  async function batchDownloadPosts() {
    const items = selectionState.getItems<Post>();
    if (items.length === 0) return;
    try {
      const count = await downloadState.downloadPosts(items);
      notify.success(
        i18n.t('selection.download_all'),
        i18n.t('counts.files', { count: count })
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('downloads.action_error'), err);
    }
  }

  async function batchUnfavoritePosts() {
    const items = selectionState.getItems<Post>();
    if (items.length === 0) return;
    try {
      for (const post of items) {
        await apiSetPostFavorite(post.service, post.user, post.id, false);
      }
      notify.success(
        i18n.t('selection.unfavorite'),
        i18n.t('counts.posts', { count: items.length })
      );
      await accountState.fetchFavorites('post', true);
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('post.favorite_failed'), err);
    }
  }

  function batchSubscribeCreators() {
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

  async function batchUnfavoriteCreators() {
    const items = selectionState.getItems<Creator>();
    if (items.length === 0) return;
    try {
      for (const creator of items) {
        await apiSetCreatorFavorite(creator.service, creator.id, false);
      }
      notify.success(
        i18n.t('selection.unfavorite'),
        i18n.t('counts.creators', { count: items.length })
      );
      await accountState.fetchFavorites('creator', true);
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('post.favorite_failed'), err);
    }
  }

  let creatorKeys = $derived(filteredCreators.map((c) => `${c.service}:${c.id}`));
  let creatorsMap = $derived(new Map(filteredCreators.map((c) => [`${c.service}:${c.id}`, c])));

  $effect(() => {
    if (activeTab === 'creators') {
      selectionState.setContext('creators', creatorKeys, creatorsMap);
    }
  });

  let lastContextTime = 0;

  function handleCreatorClick(event: MouseEvent, creator: Creator) {
    if (Date.now() - lastContextTime < 500) return;

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

  function handleCreatorContextMenu(event: MouseEvent, creator: Creator) {
    event.preventDefault();
    lastContextTime = Date.now();
    haptic();
    const key = `${creator.service}:${creator.id}`;
    const select = () => selectionState.toggle('creators', key, creator, creatorKeys, false, creatorsMap);
    if (isSelectionActive) select();
    else contextMenuState.show(event, creator.name || creator.id, creatorMenuItems(creator, select));
  }

  function handleCreatorCheckbox(event: MouseEvent, creator: Creator) {
    event.stopPropagation();
    const key = `${creator.service}:${creator.id}`;
    selectionState.toggle('creators', key, creator, creatorKeys, event.shiftKey, creatorsMap);
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

  onMount(() => {
    void loadFavorites();
  });

  $effect(() => {
    if (!initialized) void loadFavorites();
  });

  let pageMenu = $derived<MenuItem[]>([
    {
      group: 'page',
      label: i18n.t(isSelectionActive ? 'selection.exit' : 'selection.select_mode'),
      icon: IconCheckboxChecked,
      active: isSelectionActive,
      onclick: () =>
        isSelectionActive ? selectionState.exit() : selectionState.enter(activeTab === 'posts' ? 'posts' : 'creators')
    },
    {
      group: 'page',
      label: i18n.t('common.refresh'),
      icon: IconArrowClockwise,
      disabled: loading,
      onclick: () => void loadFavorites(true)
    }
  ]);
</script>

{#snippet favoriteTabs()}
  <ChoiceGroup
    options={favoriteTabOptions}
    value={activeTab}
    onchange={(val) => selectTab(val as 'posts' | 'creators')}
    align="left"
    class="favorites-mode-choice"
  >
    {#snippet addon()}
      <SortControl options={sortOptions} value={currentSort} onchange={handleSort} />
    {/snippet}
  </ChoiceGroup>
{/snippet}

{#snippet filterContent()}
  <div class="filter-section-title">
    <div class="filter-section-icon"><IconGlobe class="w-3.5 h-3.5" /></div>
    <span>{i18n.t('feed.platform')}</span>
  </div>
  <div class="service-options">
    <Button
      variant="ghost"
      size="sm"
      onclick={clearServices}
      class="filter-chip chip-all {Object.keys(currentServices).length === 0 ? 'state-include' : ''}"
    >
      <IconGlobe class="w-5 h-5" />
      <span>{i18n.t('feed.all_platforms')}</span>
    </Button>
    {#each availableServices as service}
      {@const state = currentServices[service] ?? 'neutral'}
      <Button
        variant="ghost"
        size="sm"
        onclick={() => toggleService(service)}
        class="filter-chip {state === 'include' ? 'state-include' : state === 'exclude' ? 'state-exclude' : ''}"
      >
        <ServiceIcon {service} class="w-5 h-5" />
        <span>{service}</span>
        {#if state === 'include'}
          <IconSearch class="w-3.5 h-3.5 ml-auto text-[var(--status-success)] shrink-0" />
        {:else if state === 'exclude'}
          <IconDismiss class="w-3.5 h-3.5 ml-auto text-[var(--status-error)] shrink-0" />
        {/if}
      </Button>
    {/each}
  </div>
  {#if activeTab === 'posts'}
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
          {#if state === 'include'}
            <IconSearch class="w-3.5 h-3.5 ml-auto text-[var(--status-success)] shrink-0" />
          {:else if state === 'exclude'}
            <IconDismiss class="w-3.5 h-3.5 ml-auto text-[var(--status-error)] shrink-0" />
          {/if}
        </Button>
      {/each}
    </div>
    <div class="filter-section-title">
      <div class="filter-section-icon"><IconOptions class="w-3.5 h-3.5" /></div>
      <span>{i18n.t('feed.filters')}</span>
    </div>
    <div class="settings-list">
      <div
        role="switch"
        aria-checked={onlyWithAttachments}
        tabindex="0"
        class="view-option"
        class:active={onlyWithAttachments}
        use:ripple
        onclick={() => (onlyWithAttachments = !onlyWithAttachments)}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onlyWithAttachments = !onlyWithAttachments;
          }
        }}
      >
        <span>
          <strong>{i18n.t('feed.with_attachments')}</strong>
          <small>{i18n.t('feed.with_attachments_desc')}</small>
        </span>
        <Toggle
          checked={onlyWithAttachments}
          tabindex={-1}
          class="pointer-events-none"
        />
      </div>
    </div>
  {/if}
{/snippet}

{#snippet favoritesFilter()}
  <PopoverMenu
    bind:open={filtersOpen}
    title={i18n.t('feed.filters')}
    badge={activeFilterCount}
    active={activeFilterCount > 0}
    icon={IconOptions}
  >
    {@render filterContent()}
  </PopoverMenu>
{/snippet}

{#snippet search()}
  <HeaderActions
    pinOpen
    bind:searchOpen
    bind:searchQuery
    searchPlaceholder={activeTab === 'posts' ? i18n.t('feed.search_placeholder') : i18n.t('creators.search_placeholder')}
  />
{/snippet}

<PageShell title={i18n.t('nav.favorites')} menu={pageMenu} scrollKey={navigationState.entryKey} onrefresh={() => accountState.refresh()}
  selectionTotal={activeTab === 'posts' ? filteredPosts.length : filteredCreators.length}
  onSelectAll={handleSelectAll}
  {selectionActions}
>
  {#snippet views()}{@render favoriteTabs()}{/snippet}
  {#snippet filter()}{@render favoritesFilter()}{/snippet}
  {#snippet actions()}{@render search()}{/snippet}

  {#if error && currentCount === 0}
    <EmptyState icon={IconWarning} tone="error" title={i18n.t('favorites.load_error')} description={error}>
      <Button variant="tonal" onclick={() => void loadFavorites(true)}>
        <IconArrowClockwise />
        <span>{i18n.t('common.retry')}</span>
      </Button>
    </EmptyState>
  {:else if activeTab === 'posts'}
    <PostGrid
      posts={filteredPosts}
      loading={loading}
      stateKey={`favorites:posts:${searchQuery}:services=${JSON.stringify(postServiceFilters)}:formats=${JSON.stringify(formatFilters)}:attachments=${onlyWithAttachments}:sort=${postSort}`}
      ariaLabel={i18n.t('favorites.posts')}
      emptyTitle={searchQuery || activeFilterCount > 0 ? i18n.t('favorites.no_results') : i18n.t('favorites.empty_posts')}
      emptyDescription={searchQuery || activeFilterCount > 0 ? i18n.t('favorites.no_results_desc') : (!authenticated ? i18n.t('favorites.empty_posts_guest') : i18n.t('favorites.empty_posts_desc'))}
    />
  {:else if filteredCreators.length === 0 && !creatorsHandoff.skeletonMounted}
    <EmptyState
      icon={IconPeople}
      title={searchQuery || activeFilterCount > 0 ? i18n.t('favorites.no_results') : i18n.t('favorites.empty_creators')}
      description={searchQuery || activeFilterCount > 0 ? i18n.t('favorites.no_results_desc') : (!authenticated ? i18n.t('favorites.empty_creators_guest') : i18n.t('favorites.empty_creators_desc'))}
    />
  {:else}
    <div class="grid-stack">
      <div
        class="creators-grid"
        use:paintSelect={{ scope: 'creators', keys: creatorKeys, items: creatorsMap }}
        {@attach ctrlWheel(gridScaleWheel)}
        style={`--favorites-card-width: ${targetCardWidth}px; --favorites-gap: ${gap}px;`}
      >
      {#each visibleCreators as creator, index (creator.service + ':' + creator.id)}
        {@const creatorKey = `${creator.service}:${creator.id}`}
        {@const isSelected = isSelectionActive && selectionState.isSelected(creatorKey)}
        {@const placeholder = creatorPlaceholderUrl(creator)}
        {@const avatarSrc = creatorAvatarSrc(creator)}
        {@const isCreatorFav = accountState.isCreatorFavorite(creator.service, creator.id)}
        {@const creatorSub = subscriptionState.forCreator(creator.service, creator.id)}
        {@const favPending = favoriteActions.isCreatorPending(creator)}
        {@const enterDelay = creatorsHandoff.delayFor(index)}
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

          <div class="grid-tile-placeholder"><span class="fallback-initials">{creator.name.slice(0, 2).toUpperCase()}</span></div>
          {#if placeholder}
            <img class="grid-tile-media placeholder-blur" src={placeholder} alt="" aria-hidden="true" />
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
              <span class="grid-tile-logo inline-logo"><ServiceIcon service={creator.service} /></span>
              <span class="grid-tile-author-name is-service">{creator.service}</span>
            </div>
            <div class="grid-tile-meta">
              <span>{creator.id}</span>
              <div class="grid-tile-meta-stats">
                <span class="grid-tile-meta-row"><IconHeartFilled />{i18n.t('favorites.saved')}</span>
                <span>{formatDate(creator.updated ?? creator.indexed)}</span>
              </div>
            </div>
          </div>
        </article>
      {/each}
      </div>
      {#if creatorsHandoff.skeletonMounted}
        <SkeletonGrid
          cardWidth={targetCardWidth}
          {gap}
          fading={creatorsHandoff.skeletonFading}
        />
      {/if}
    </div>

    {#if hasMoreCreators}
      <div use:creatorSentinel class="sentinel">
        <IconLoading />
      </div>
    {/if}
  {/if}

  {#if error && currentCount > 0}<p class="inline-error">{error}</p>{/if}
</PageShell>

{#snippet selectionActions()}
  {#if activeTab === 'posts'}
    <Select
      options={stashOptions}
      selectedValues={batchSelectedStashes}
      placeholder={i18n.t('library.add_to_stash')}
      onchange={handleBatchToggleStash}
      createLabel={i18n.t('library.new_stash')}
      onCreate={handleBatchCreateAndAddToStash}
      variant="ghost"
      multi={true}
      closeOnChange={false}
      icon={IconFolder}
      iconOnly={true}
      ariaLabel={i18n.t('library.add_to_stash')}
      align="right"
    />

    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchSaveToLibrary}
      title={i18n.t('selection.save_to_library')}
    >
      <IconBookmarkAdd />
    </Button>

    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchDownloadPosts}
      title={i18n.t('selection.download_all')}
    >
      <IconArrowDownload />
    </Button>

    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchUnfavoritePosts}
      title={i18n.t('selection.unfavorite')}
    >
      <IconHeartOff />
    </Button>
  {:else}
    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchSubscribeCreators}
      title={i18n.t('selection.subscribe')}
    >
      <IconPersonAdd />
    </Button>

    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchUnfavoriteCreators}
      title={i18n.t('selection.unfavorite')}
    >
      <IconHeartOff />
    </Button>
  {/if}
{/snippet}

<style>
  :global(.favorites-mode-choice) {
    flex-shrink: 0 !important;
  }

  .creators-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--favorites-card-width)), 1fr));
    gap: var(--favorites-gap);
    align-items: start;
  }

  .fallback-initials { font-size: 26px; font-weight: 700; color: var(--text-muted); }
  .inline-error { margin-top: 20px; text-align: center; font-size: 12px; }
  .sentinel {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 32px 0;
    color: var(--text-muted);
  }
</style>
