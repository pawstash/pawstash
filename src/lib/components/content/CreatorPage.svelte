<script lang="ts">
  import { onMount, onDestroy, tick, untrack } from 'svelte';
  import { contentState, creatorCacheKey, type CachedCreator } from '$lib/state/contentState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import { accountState } from '$lib/state/accountState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { providerState } from '$lib/state/providerState.svelte';
  import { themeState, getContrastColor } from '$lib/theme/themeState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import {
    apiFetchCreatorArtworkDataUrl,
    apiFetchCreatorProfile,
    apiFetchCreatorPosts,
    apiFetchCreatorLinks,
    apiFetchSimilarCreators,
    apiFetchCreatorTags,
    apiFetchAnnouncements,
    apiFetchFancards,
    apiOpenInBrowser,
    apiSetCreatorFavorite,
    apiSetPostFavorite
  } from '$lib/utils/ipc';
  import {
    creatorAvatarUrl,
    creatorBannerUrl,
    creatorPageUrl,
    fancardMediaUrl,
    fancardThumbnailUrl,
    formatProviderName,
    getPlatformProfileUrl,
    getPostDownloadTargets,
    getPostFormats
  } from '$lib/utils/media';
  import { thumbHashToAverageColor } from '$lib/utils/thumbhash';
  import { parseTags, getPostTags, formatDate, formatBytes, parseDateTimestamp, cleanPostTitle } from '$lib/utils/formatters';
  import { logger } from '$lib/utils/logger';
  import type { DownloadScope, InitialImport } from '$lib/types/subscription';
  import type { Post, CreatorProfile, Announcement, Fancard } from '$lib/types/content';
  import type { FilterMap } from '$lib/types/filter';
  import { countActiveFilters, matchesTriStateFilter, toggleFilterKey } from '$lib/types/filter';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import HeroBackdrop from '$lib/components/ui/HeroBackdrop.svelte';
  import TagList from '$lib/components/ui/TagList.svelte';
  import CountBadge from '$lib/components/ui/CountBadge.svelte';
  import PostGrid from './PostGrid.svelte';
  import RichContent from './RichContent.svelte';
  import MediaViewer, { type MediaViewerItem } from './MediaViewer.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import ChoiceGroup, { type ChoiceOption } from '$lib/components/ui/ChoiceGroup.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import { ripple } from '$lib/motion';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import SelectionActionBar from '$lib/components/ui/SelectionActionBar.svelte';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
  import IconArrowSort from '~icons/fluent/arrow-sort-24-regular';
  import IconAdd from '~icons/fluent/add-24-regular';
  import IconSettings from '~icons/fluent/settings-24-regular';
  import IconGrid from '~icons/fluent/grid-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconHeart from '~icons/fluent/heart-24-regular';
  import IconHeartFilled from '~icons/fluent/heart-24-filled';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconBookmarkAdd from '~icons/fluent/bookmark-add-24-regular';
  import IconOptions from '~icons/fluent/options-24-regular';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconDraft from '~icons/fluent/drafts-24-regular';
  import IconText from '~icons/fluent/document-text-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconMoreVertical from '~icons/fluent/more-vertical-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconLink from '~icons/fluent/link-24-regular';
  import IconNews from '~icons/fluent/news-24-regular';
  import IconCard from '~icons/fluent/payment-24-regular';
  import IconClock from '~icons/fluent/clock-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconImageOff from '~icons/fluent/image-off-24-regular';
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import { notify } from '$lib/utils/toast';
  import { tooltip } from '$lib/motion';

  interface Props {
    service: string;
    creatorId: string;
    initialTag?: string;
  }

  let { service, creatorId, initialTag }: Props = $props();

  const ratios = {
    square: '1 / 1',
    portrait: '4 / 5',
    landscape: '3 / 2',
    widescreen: '16 / 9'
  } as const;
  let ratio = $derived(ratios[configState.settings.grid_aspect_ratio]);

  const savedState = navigationState.getViewState<{
    postSearchQuery?: string;
    postSearchOpen?: boolean;
    postSearchResults?: Post[];
    formatFilters?: FilterMap;
    onlyWithAttachments?: boolean;
    activeTab?: 'posts' | 'similar' | 'links' | 'announcements' | 'fancards';
    sortOrder?: 'default' | 'newest' | 'oldest' | 'popular';
    selectedTag?: string | null;
  }>(navigationState.entryKey);

  const emptyEntry: CachedCreator = {
    profile: null,
    posts: [],
    loading: false,
    loadingMore: false,
    loaded: false,
    error: null,
    offset: 0,
    hasMore: true
  };

  let entry = $derived.by(() => contentState.creators[creatorCacheKey(service, creatorId)] ?? emptyEntry);
  let subscription = $derived(subscriptionState.forCreator(service, creatorId));
  let subscriptionMenuOpen = $state(false);
  let saving = $state(false);
  let destination = $state('00000000-0000-0000-0000-000000000001');
  let initialImport = $state<InitialImport>('none');
  let autoDownload = $state(false);
  let downloadScope = $state<DownloadScope>('primary');
  let interval = $state(30);
  let authenticated = $derived(accountState.session.authenticated);
  let isFavorited = $state(false);
  let favoritingPending = $state(false);

  let activeTab = $state<'posts' | 'similar' | 'links' | 'announcements' | 'fancards'>(savedState?.activeTab ?? 'posts');
  let sortOrder = $state<'default' | 'newest' | 'oldest' | 'popular'>(savedState?.sortOrder ?? 'default');
  let selectedTag = $state<string | null>(savedState?.selectedTag ?? null);

  let lastHandledInitialTag: string | undefined = undefined;
  $effect(() => {
    if (initialTag && initialTag !== lastHandledInitialTag) {
      lastHandledInitialTag = initialTag;
      selectedTag = initialTag;
      activeTab = 'posts';
    }
  });

  let similarCreators = $state<CreatorProfile[]>([]);
  let creatorLinks = $state<CreatorProfile[]>([]);
  let announcements = $state<Announcement[]>([]);
  let fancards = $state<Fancard[]>([]);
  let apiCreatorTags = $state<string[]>([]);

  let postSearchOpen = $state(savedState?.postSearchOpen ?? Boolean(savedState?.postSearchQuery));
  let postSearchQuery = $state(savedState?.postSearchQuery ?? '');
  let postSearchResults = $state<Post[]>(savedState?.postSearchResults ?? []);
  let formatFilters = $state<FilterMap>(savedState?.formatFilters ?? {});
  let onlyWithAttachments = $state<boolean>(savedState?.onlyWithAttachments ?? false);
  let filtersOpen = $state(false);
  let stickyFiltersOpen = $state(false);
  let mobileMoreOpen = $state(false);
  let mobileSubSettingsOpen = $state(false);
  let activeFilterCount = $derived(countActiveFilters([formatFilters]) + (onlyWithAttachments ? 1 : 0));

  let postSearchLoading = $state(false);
  let postSearchError = $state<string | null>(null);
  let postSearchOffset = $state(0);
  let postSearchHasMore = $state(false);
  let postSearchRequest = 0;
  const CREATOR_POST_PAGE_SIZE = 50;

  let baseCardWidth = $derived(layoutState.isMobile ? 155 : 245);
  let scale = $derived(configState.settings.grid_scale / 100);
  let gap = $derived(Math.round((layoutState.isMobile ? 8 : 10) * scale));
  let targetCardWidth = $derived(baseCardWidth * scale);

  function toggleFormat(fmt: string) {
    formatFilters = toggleFilterKey(formatFilters, fmt);
  }

  let formatList = $derived([
    { id: 'image', label: () => i18n.t('feed.format_photo') || 'Photo', icon: IconImage },
    { id: 'video', label: () => i18n.t('feed.format_video') || 'Video', icon: IconVideo },
    { id: 'audio', label: () => i18n.t('feed.format_audio') || 'Audio', icon: IconMusic },
    { id: 'text', label: () => i18n.t('feed.format_text') || 'Text', icon: IconText },
    { id: 'archive', label: () => i18n.t('feed.format_archive') || 'Files', icon: IconDocument },
    { id: 'wip', label: () => i18n.t('feed.format_wip') || 'WIP / Sketch', icon: IconDraft },
    ...(!configState.settings.pawchive_hide_ai ? [{ id: 'ai', label: () => i18n.t('feed.format_ai') || 'AI Generated', icon: IconSparkle }] : [])
  ]);

  function clearAllFilters() {
    formatFilters = {};
    onlyWithAttachments = false;
  }

  $effect(() => {
    navigationState.saveViewState(navigationState.entryKey, {
      postSearchQuery,
      postSearchOpen,
      postSearchResults,
      formatFilters: $state.snapshot(formatFilters),
      onlyWithAttachments,
      activeTab,
      sortOrder,
      selectedTag
    });
  });

  let creatorName = $derived.by<string>(() => {
    if (typeof entry.profile?.name === 'string' && entry.profile.name !== creatorId) {
      return entry.profile.name;
    }
    const fromMap = creatorsState.creatorsMap.get(`${service.toLowerCase()}:${creatorId.toLowerCase()}`);
    if (fromMap && fromMap !== creatorId) {
      return fromMap;
    }
    return (typeof entry.profile?.name === 'string' && entry.profile.name) || String(creatorId);
  });
  let cachedAvatarUrl = $state<string | null>(null);
  let cachedBannerUrl = $state<string | null>(null);
  let avatarThumbhash = $derived((entry.profile?.extra as any)?.avatar_thumbhash);
  let headerThumbhash = $derived((entry.profile?.extra as any)?.header_thumbhash);
  let avatarUrl = $derived(cachedAvatarUrl || creatorAvatarUrl(service, creatorId, avatarThumbhash));
  let bannerUrl = $derived(cachedBannerUrl || creatorBannerUrl(service, creatorId, headerThumbhash));
  let avatarFailed = $state(false);
  let bannerFailed = $state(false);
  let effectiveAvatar = $derived(avatarFailed ? null : avatarUrl);
  let effectiveBanner = $derived(bannerFailed ? null : bannerUrl);

  $effect(() => {
    if (bannerUrl) {
      bannerFailed = false;
      const img = new Image();
      img.src = bannerUrl;
      img.onerror = () => {
        bannerFailed = true;
      };
    }
  });

  let initialLetter = $derived(creatorName ? creatorName.charAt(0).toUpperCase() : '?');

  let candidateProviders = $derived(providerState.getProvidersForService(service));
  let providerSelectOptions = $derived.by(() => {
    if (candidateProviders.length <= 1) {
      return candidateProviders.map((p) => ({
        value: p.id,
        label: formatProviderName(p.name)
      }));
    }
    return [
      { value: 'auto', label: i18n.t('post.source_auto') || 'Merged' },
      ...candidateProviders.map((p) => ({
        value: p.id,
        label: formatProviderName(p.name)
      }))
    ];
  });
  let activeProviderId = $derived(
    candidateProviders.length === 1
      ? candidateProviders[0].id
      : providerState.getSelectedProvider(service, creatorId, '*')
  );

  let creatorTags = $derived.by<Array<{ name: string; count: number }>>(() => {
    const tagCountMap = new Map<string, number>();
    const tagDisplayNameMap = new Map<string, string>();

    for (const post of entry.posts) {
      const pTags = getPostTags(post);
      for (const rawTag of pTags) {
        const clean = rawTag.replace(/^#+/, '').trim();
        if (!clean) continue;
        const lower = clean.toLowerCase();
        tagCountMap.set(lower, (tagCountMap.get(lower) || 0) + 1);
        if (!tagDisplayNameMap.has(lower)) {
          tagDisplayNameMap.set(lower, clean);
        }
      }
    }

    if (entry.posts.length === 0) {
      const direct = parseTags(entry.profile?.tags || (entry.profile?.extra as any)?.tags || (entry.profile?.extra as any)?.categories);
      for (const t of [...direct, ...apiCreatorTags]) {
        const clean = t.replace(/^#+/, '').trim();
        if (!clean) continue;
        const lower = clean.toLowerCase();
        if (!tagDisplayNameMap.has(lower)) {
          tagDisplayNameMap.set(lower, clean);
          tagCountMap.set(lower, 0);
        }
      }
    }

    if (tagDisplayNameMap.size === 0) return [];

    return Array.from(tagDisplayNameMap.entries())
      .map(([lower, name]) => ({
        name,
        count: tagCountMap.get(lower) || 0
      }))
      .filter((item) => (entry.posts.length > 0 ? item.count > 0 : true))
      .sort((a, b) => b.count - a.count);
  });

  let normalizedPostSearch = $derived(postSearchQuery.trim().toLocaleLowerCase());
  let visibleCreatorPosts = $derived.by(() => {
    let posts = entry.posts;

    if (configState.settings.pawchive_hide_ai) {
      posts = posts.filter((post) => {
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

    if (normalizedPostSearch) {
      const localMatches = entry.posts.filter((post) =>
        [post.title, post.id, post.content, post.substring]
          .some((value) => String(value ?? '').toLocaleLowerCase().includes(normalizedPostSearch))
      );

      if (normalizedPostSearch.length >= 2 && postSearchResults.length > 0) {
        const map = new Map<string, Post>();
        for (const p of localMatches) map.set(`${p.service}:${p.user}:${p.id}`, p);
        for (const p of postSearchResults) map.set(`${p.service}:${p.user}:${p.id}`, p);
        posts = Array.from(map.values());
      } else {
        posts = localMatches;
      }
    }

    if (selectedTag) {
      const normTag = selectedTag.trim().toLowerCase().replace(/^#+/, '');
      const normSpaceTag = normTag.replace(/_/g, ' ');
      posts = posts.filter((post) => {
        const pTags = getPostTags(post).map((t) => t.trim().toLowerCase().replace(/^#+/, ''));
        return pTags.some((pt) => pt === normTag || pt.replace(/_/g, ' ') === normSpaceTag);
      });
    }

    if (onlyWithAttachments) {
      posts = posts.filter((post) => (post.attachment_count ?? post.attachments?.length ?? 0) > 0 || Boolean(post.file?.path));
    }

    if (Object.keys(formatFilters).length > 0) {
      posts = posts.filter((post) => matchesTriStateFilter(getPostFormats(post), formatFilters));
    }

    if (sortOrder === 'newest') {
      posts = [...posts].sort((a, b) => {
        const da = parseDateTimestamp(a.published || a.added);
        const db = parseDateTimestamp(b.published || b.added);
        return db - da;
      });
    } else if (sortOrder === 'oldest') {
      posts = [...posts].sort((a, b) => {
        const da = parseDateTimestamp(a.published || a.added);
        const db = parseDateTimestamp(b.published || b.added);
        return da - db;
      });
    } else if (sortOrder === 'popular') {
      posts = [...posts].sort((a, b) => {
        const getFav = (p: any) => Number(
          p.favorite_count ??
          p.extra?.favorite_count ??
          p.extra?.favorites ??
          p.extra?.favs ??
          p.extra?.likes ??
          p.extra?.likeCount ??
          p.extra?.like_count ??
          p.extra?.bookmarked ??
          p.extra?.bookmarks ??
          p.extra?.fav_count ??
          p.extra?.score ??
          0
        );
        const diff = getFav(b) - getFav(a);
        if (diff !== 0) return diff;
        const da = parseDateTimestamp(a.published || a.added);
        const db = parseDateTimestamp(b.published || b.added);
        return db - da;
      });
    }

    return posts;
  });

  let sortOptions = $derived([
    { value: 'default', label: i18n.t('post.media_sort_default') || 'Default Order' },
    { value: 'newest', label: i18n.t('favorites.sort_published_desc') || 'Newest First' },
    { value: 'oldest', label: i18n.t('favorites.sort_published_asc') || 'Oldest First' },
    { value: 'popular', label: i18n.t('creators.sort_favorited_desc') || 'Most Favorited' }
  ]);

  let currentSortLabel = $derived.by(() => {
    const opt = sortOptions.find((o) => o.value === sortOrder);
    return opt?.label ?? (i18n.t('favorites.sort_by') || 'Sort');
  });

  let isPostsFiltered = $derived(
    Boolean(selectedTag || onlyWithAttachments || Object.keys(formatFilters).length > 0 || normalizedPostSearch)
  );

  let creatorTabOptions = $derived.by(() => {
    let postsCount: string | number | undefined;
    if (isPostsFiltered) {
      postsCount = visibleCreatorPosts.length;
    } else if (entry.posts.length > 0) {
      postsCount = `${entry.posts.length}${entry.hasMore ? '+' : ''}`;
    }

    const list: Array<ChoiceOption<'posts' | 'similar' | 'links' | 'announcements' | 'fancards'>> = [
      {
        value: 'posts',
        label: i18n.t('creator.posts') || 'Posts',
        count: postsCount
      }
    ];

    if (similarCreators.length > 0) {
      list.push({
        value: 'similar',
        label: i18n.t('creator.similar_artists') || 'Similar Artists',
        count: similarCreators.length
      });
    }

    if (creatorLinks.length > 0) {
      list.push({
        value: 'links',
        label: i18n.t('creator.linked_accounts') || 'Linked Accounts',
        count: creatorLinks.length
      });
    }

    if (announcements.length > 0) {
      list.push({
        value: 'announcements',
        label: i18n.t('creator.announcements') || 'Announcements',
        count: announcements.length
      });
    }

    if (fancards.length > 0) {
      list.push({
        value: 'fancards',
        label: i18n.t('creator.fancards') || 'Fancards',
        count: fancards.length
      });
    }

    return list;
  });

  $effect(() => {
    if (!postSearchOpen) return;
    void tick().then(() => {
      const isStickyVisible = Boolean(document.querySelector('.sticky-header-bar.visible'));
      const selector = isStickyVisible
        ? '.sticky-header-bar.visible .search-input-field'
        : '.media-controls-right .search-input-field, .post-content-wrapper .search-input-field';
      const input = document.querySelector<HTMLInputElement>(selector);
      if (input) {
        input.focus();
        input.select?.();
      }
    });
  });

  function closePostSearch() {
    postSearchRequest += 1;
    postSearchOpen = false;
    postSearchQuery = '';
    postSearchResults = [];
    postSearchLoading = false;
    postSearchError = null;
    postSearchOffset = 0;
    postSearchHasMore = false;
  }

  async function searchCreatorPosts(reset = false) {
    const query = postSearchQuery.trim();
    if (query.length < 2 || (!reset && postSearchLoading)) return;

    const request = ++postSearchRequest;
    const offset = reset ? 0 : postSearchOffset;
    postSearchLoading = true;
    postSearchError = null;

    try {
      const posts = await apiFetchCreatorPosts(service, creatorId, query, offset);
      if (request !== postSearchRequest || query !== postSearchQuery.trim()) return;

      postSearchResults = reset ? posts : [...postSearchResults, ...posts];
      postSearchOffset = offset + posts.length;
      postSearchHasMore = posts.length === CREATOR_POST_PAGE_SIZE;

      if (postSearchHasMore) {
        void (async () => {
          while (postSearchHasMore && request === postSearchRequest && query === postSearchQuery.trim()) {
            await new Promise((resolve) => setTimeout(resolve, 180));
            if (request !== postSearchRequest || query !== postSearchQuery.trim()) break;

            try {
              const nextBatch = await apiFetchCreatorPosts(service, creatorId, query, postSearchOffset);
              if (request !== postSearchRequest || query !== postSearchQuery.trim()) break;

              const existingIds = new Set(postSearchResults.map((p) => p.id));
              const newItems = nextBatch.filter((p) => !existingIds.has(p.id));
              postSearchResults = [...postSearchResults, ...newItems];
              postSearchOffset += nextBatch.length;
              postSearchHasMore = nextBatch.length === CREATOR_POST_PAGE_SIZE;
              if (!postSearchHasMore || nextBatch.length === 0) break;
            } catch (err) {
              logger.warn('Search auto-fetch error:', err);
              break;
            }
          }
        })();
      }
    } catch (error) {
      if (request === postSearchRequest) {
        postSearchError = error instanceof Error ? error.message : String(error);
      }
    } finally {
      if (request === postSearchRequest) postSearchLoading = false;
    }
  }

  $effect(() => {
    const query = normalizedPostSearch;
    postSearchRequest += 1;
    postSearchError = null;

    if (query.length < 2) {
      postSearchResults = [];
      postSearchOffset = 0;
      postSearchHasMore = false;
      postSearchLoading = false;
      return;
    }

    const timeout = window.setTimeout(() => void searchCreatorPosts(true), 250);
    return () => window.clearTimeout(timeout);
  });

  async function loadExtraData() {
    apiFetchSimilarCreators(service, creatorId).then((res) => {
      const map = new Map<string, CreatorProfile>();
      for (const item of (res || [])) {
        const key = `${item.service || service}:${item.id}`;
        if (!map.has(key) && !(item.id === creatorId && (item.service || service) === service)) {
          map.set(key, item);
        }
      }
      similarCreators = Array.from(map.values());
    }).catch(() => {});

    apiFetchCreatorLinks(service, creatorId).then((res) => {
      creatorLinks = res || [];
    }).catch(() => {});

    apiFetchAnnouncements(service, creatorId).then((res) => {
      announcements = res || [];
    }).catch(() => {});

    apiFetchFancards(service, creatorId).then((res) => {
      fancards = res || [];
    }).catch(() => {});

    apiFetchCreatorTags(service, creatorId).then((res) => {
      if (res && res.length > 0) {
        apiCreatorTags = res;
      }
    }).catch(() => {});
  }

  let lastLoadedCreatorKey = '';
  $effect(() => {
    const currentService = service;
    const currentCreatorId = creatorId;
    const currentKey = `${currentService}:${currentCreatorId}`;
    if (currentService && currentCreatorId) {
      if (lastLoadedCreatorKey !== currentKey) {
        lastLoadedCreatorKey = currentKey;
        cachedAvatarUrl = null;
        cachedBannerUrl = null;
        avatarFailed = false;
        bannerFailed = false;
        similarCreators = [];
        creatorLinks = [];
        announcements = [];
        fancards = [];
        apiCreatorTags = [];
      }
      untrack(() => {
        void contentState.loadCreator(currentService, currentCreatorId, true);
        void apiFetchCreatorArtworkDataUrl(currentService, currentCreatorId, 'avatar').then((url) => cachedAvatarUrl = url).catch(() => {});
        void apiFetchCreatorArtworkDataUrl(currentService, currentCreatorId, 'banner').then((url) => cachedBannerUrl = url).catch(() => {});
        void loadExtraData();
        void checkFavoriteStatus();
      });
    }
  });

  $effect(() => {
    const s = service;
    const c = creatorId;
    return () => {
      contentState.stopAutoFetchCreatorPosts(s, c);
    };
  });

  $effect(() => {
    if (creatorName === creatorId) {
      void apiFetchCreatorProfile(service, creatorId).then((p) => {
        if (p && p.name && p.name !== creatorId) {
          const key = creatorCacheKey(service, creatorId);
          const cur = contentState.creators[key];
          if (cur) {
            contentState.creators[key] = { ...cur, profile: p };
          }
        }
      }).catch(() => {});
      void creatorsState.load();
    }
  });

  async function refreshCreator() {
    try {
      await Promise.all([
        contentState.refreshCreator(service, creatorId),
        apiFetchCreatorArtworkDataUrl(service, creatorId, 'avatar').then((url) => cachedAvatarUrl = url).catch(() => {}),
        apiFetchCreatorArtworkDataUrl(service, creatorId, 'banner').then((url) => cachedBannerUrl = url).catch(() => {}),
        loadExtraData(),
        checkFavoriteStatus()
      ]);
    } catch (error) {
      notify.error(i18n.t('feed.refresh_failed') || 'Failed to refresh', error);
    }
  }

  function getAverageColor(url: string): Promise<string> {
    return new Promise((resolve) => {
      const img = new Image();
      img.crossOrigin = 'Anonymous';
      img.onload = () => {
        try {
          const canvas = document.createElement('canvas');
          canvas.width = 1;
          canvas.height = 1;
          const ctx = canvas.getContext('2d');
          if (!ctx) return resolve('');
          ctx.drawImage(img, 0, 0, 1, 1);
          const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
          resolve(`rgb(${r}, ${g}, ${b})`);
        } catch (error) {
          logger.warn('Creator artwork color extraction failed', error);
          resolve('');
        }
      };
      img.onerror = () => resolve('');
      img.src = url;
    });
  }

  async function getCreatorAccentColor(hasBanner: boolean, hasAvatar: boolean) {
    const artworkKinds: Array<'banner' | 'avatar'> = [];
    if (hasBanner) artworkKinds.push('banner');
    if (hasAvatar) artworkKinds.push('avatar');

    for (const artworkKind of artworkKinds) {
      try {
        const dataUrl = await apiFetchCreatorArtworkDataUrl(service, creatorId, artworkKind);
        const color = await getAverageColor(dataUrl);
        if (color) return color;
      } catch (error) {
        logger.warn(`Failed to extract creator ${artworkKind} accent for ${service}:${creatorId}`, error);
      }
    }
    return '';
  }

  $effect(() => {
    const dynamicAccent = configState.settings.dynamic_accent;
    if (!dynamicAccent) return;

    let cancelled = false;
    const cachedAccent = contentState.getCreatorAccent(service, creatorId);
    const thumbColor = cachedAccent || thumbHashToAverageColor(headerThumbhash) || thumbHashToAverageColor(avatarThumbhash);

    if (thumbColor) {
      themeState.setOverrideAccent(thumbColor);
    }

    const hasBanner = Boolean(effectiveBanner);
    const hasAvatar = Boolean(effectiveAvatar);

    if (!cachedAccent && !thumbColor && (hasBanner || hasAvatar)) {
      void getCreatorAccentColor(hasBanner, hasAvatar).then((color) => {
        if (!color || cancelled) return;
        contentState.setCreatorAccent(service, creatorId, color);
        themeState.setOverrideAccent(color);
      });
    }

    return () => {
      cancelled = true;
      themeState.clearOverrideAccent();
    };
  });

  async function checkFavoriteStatus() {
    try {
      await accountState.fetchFavorites('creator');
      isFavorited = accountState.isCreatorFavorite(service, creatorId);
    } catch (error) {
      logger.error(`Failed to check creator favorite status for ${service}:${creatorId}`, error);
    }
  }

  async function toggleFavorite() {
    if (favoritingPending) return;

    favoritingPending = true;
    const targetState = !isFavorited;
    try {
      await apiSetCreatorFavorite(service, creatorId, targetState);
      isFavorited = targetState;

      if (targetState) {
        accountState.addCreatorFavoriteOptimistic({ id: creatorId, service, name: creatorName });
      } else {
        accountState.removeCreatorFavoriteOptimistic(service, creatorId);
      }

      if (!authenticated) {
        notify.success(i18n.t(targetState ? 'favorites.saved_locally' : 'favorites.removed_locally'));
      } else {
        notify.success(i18n.t(targetState ? 'post.added_to_favorites' : 'post.removed_from_favorites'), creatorName);
      }
    } catch (error) {
      notify.error(i18n.t('post.favorite_failed'), error);
    } finally {
      favoritingPending = false;
    }
  }

  function initEditorFields() {
    if (subscription) {
      destination = subscription.destination_collection_id;
      initialImport = subscription.initial_import;
      autoDownload = subscription.auto_download;
      downloadScope = subscription.download_scope;
      interval = subscription.poll_interval_minutes;
    }
  }

  async function subscribeDefault() {
    saving = true;
    try {
      await subscriptionState.save({
        service,
        creator_id: creatorId,
        creator_name: creatorName,
        destination_collection_id: '00000000-0000-0000-0000-000000000001',
        initial_import: 'none',
        auto_download: false,
        download_scope: 'primary',
        poll_interval_minutes: 30
      });
      notify.success(i18n.t('subscriptions.saved'), creatorName);
    } catch (error) {
      notify.error(i18n.t('subscriptions.action_error'), error);
    } finally {
      saving = false;
    }
  }

  async function saveSubscription() {
    saving = true;
    try {
      await subscriptionState.save({
        service,
        creator_id: creatorId,
        creator_name: creatorName,
        destination_collection_id: destination,
        initial_import: initialImport,
        auto_download: autoDownload,
        download_scope: downloadScope,
        poll_interval_minutes: interval
      });
      subscriptionMenuOpen = false;
      mobileSubSettingsOpen = false;
      notify.success(i18n.t('subscriptions.saved'), creatorName);
    } catch (error) {
      notify.error(i18n.t('subscriptions.action_error'), error);
    } finally {
      saving = false;
    }
  }

  async function unsubscribe() {
    if (!subscription) return;
    saving = true;
    try {
      await subscriptionState.remove(subscription.id);
      subscriptionMenuOpen = false;
      mobileSubSettingsOpen = false;
      notify.success(i18n.t('subscriptions.removed'), creatorName);
    } catch (error) {
      notify.error(i18n.t('subscriptions.action_error'), error);
    } finally {
      saving = false;
    }
  }

  async function handleLoadMore() {
    if (normalizedPostSearch.length >= 2) {
      if (!postSearchLoading && postSearchHasMore) {
        await searchCreatorPosts();
      }
      return;
    }
    if (!entry.loadingMore && entry.hasMore) {
      await contentState.loadMoreCreatorPosts(service, creatorId);
    }
  }

  let currentProviderName = $derived.by(() => {
    if (activeProviderId && activeProviderId !== 'auto') {
      const p = providerState.getProviderById(activeProviderId);
      if (p) return formatProviderName(p.name);
    }
    const defaultProv = providerState.getDriverForService(service)?.config;
    if (defaultProv) return formatProviderName(defaultProv.name || defaultProv.id);
    return 'Provider';
  });

  function openInProvider() {
    const url = creatorPageUrl(
      service,
      creatorId,
      activeProviderId && activeProviderId !== 'auto' ? activeProviderId : undefined
    );
    if (url) void apiOpenInBrowser(url).catch((err) => logger.warn('Failed to open creator in provider', err));
  }

  function openOriginalProfile() {
    const url = getPlatformProfileUrl(service, creatorId, entry.profile?.public_id);
    if (url) void apiOpenInBrowser(url).catch((err) => logger.warn('Failed to open creator original profile', err));
  }

  function openInBrowser() {
    openInProvider();
  }

  onDestroy(() => {
    contentState.stopAutoFetchCreatorPosts(service, creatorId);
  });

  let copiedId = $state(false);
  async function copyCreatorId() {
    try {
      await navigator.clipboard.writeText(creatorId);
      copiedId = true;
      notify.success(i18n.t('common.copied') || 'Copied ID');
      setTimeout(() => { copiedId = false; }, 2000);
    } catch (e) {
      logger.error(`Failed to copy creator ID: ${creatorId}`, e);
    }
  }

  // Media Viewer state for fancards
  let isViewerOpen = $state(false);
  let viewerActiveIndex = $state(0);
  let viewerItems = $derived.by((): MediaViewerItem[] =>
    fancards.map((card, idx) => {
      const ext = (card.ext || card.mime?.split('/').pop() || 'jpg').replace(/^\.+/, '');
      return {
        id: String(card.id || card.hash || idx),
        name: `fancard_${card.id || idx + 1}.${ext}`,
        kind: 'image',
        url: fancardMediaUrl(card, service),
        poster: fancardThumbnailUrl(card, service),
        size: card.size
      };
    })
  );

  function openFancardViewer(index: number) {
    viewerActiveIndex = index;
    isViewerOpen = true;
  }

  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'posts');
  let selectedCount = $derived(selectionState.count);

  let selectedPosts = $derived(isSelectionActive ? selectionState.getItems<Post>() : []);
  let stashes = $derived(libraryState.allStashes);
  let stashOptions = $derived(stashes.map((s) => ({ value: s.id, label: libraryState.getStashDisplayName(s) })));

  let batchSelectedStashes = $derived.by(() => {
    if (selectedPosts.length === 0) return [];
    const stashCounts = new Map<string, number>();
    for (const post of selectedPosts) {
      const ids = libraryState.getPostStashes(post);
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

  function handleSelectAllPosts() {
    selectionState.selectAll(visibleCreatorPosts.map((p) => ({
      key: `${p.service}:${p.user}:${p.id}`,
      item: p
    })));
  }

  async function handleBatchToggleStash(collectionId: string) {
    const items = selectionState.getItems<Post>();
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
    const items = selectionState.getItems<Post>();
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

  async function batchSaveToLibrary() {
    const items = selectionState.getItems<Post>();
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
    const items = selectionState.getItems<Post>();
    if (items.length === 0) return;
    try {
      const count = await downloadState.downloadPosts(items);
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
    const items = selectionState.getItems<Post>();
    if (items.length === 0) return;
    try {
      for (const post of items) {
        await apiSetPostFavorite(post.service, post.user, post.id, isFav);
        if (isFav) {
          accountState.addPostFavoriteOptimistic({ id: post.id, service: post.service, user: post.user, title: post.title });
        } else {
          accountState.removePostFavoriteOptimistic(post.service, post.user, post.id);
        }
      }
      notify.success(
        isFav
          ? (i18n.t('selection.favorite') || 'Favorited')
          : (i18n.t('selection.unfavorite') || 'Unfavorited'),
        `${items.length} ${items.length === 1 ? 'post' : 'posts'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('post.favorite_failed') || 'Failed to update favorites', err);
    }
  }
</script>

{#snippet subscriptionEditorFields()}
  <div class="sub-form flex flex-col gap-4">
    <div class="field-group">
      <div class="field-label text-xs font-semibold text-[var(--fg-muted)] mb-1">
        {i18n.t('subscriptions.destination')}
      </div>
      <Select
        options={stashOptions}
        value={destination}
        onchange={(v) => destination = v}
      />
    </div>

    <div class="field-group">
      <div class="field-label text-xs font-semibold text-[var(--fg-muted)] mb-1">
        {i18n.t('subscriptions.initial_import')}
      </div>
      <Select
        options={[
          { value: 'none', label: i18n.t('subscriptions.import_none') },
          { value: 'latest50', label: i18n.t('subscriptions.import_latest') },
          { value: 'all', label: i18n.t('subscriptions.import_all') }
        ]}
        value={initialImport}
        onchange={(v) => initialImport = v as InitialImport}
      />
    </div>

    <div class="field-group flex items-center justify-between">
      <span class="text-xs font-semibold text-[var(--fg-default)]">{i18n.t('subscriptions.auto_download')}</span>
      <Checkbox checked={autoDownload} onchange={(v) => autoDownload = v} />
    </div>

    {#if autoDownload}
      <div class="field-group">
        <div class="field-label text-xs font-semibold text-[var(--fg-muted)] mb-1">
          {i18n.t('subscriptions.download_scope')}
        </div>
        <Select
          options={[
            { value: 'primary', label: i18n.t('subscriptions.primary_file') },
            { value: 'all', label: i18n.t('subscriptions.all_files') }
          ]}
          value={downloadScope}
          onchange={(v) => downloadScope = v as DownloadScope}
        />
      </div>
    {/if}

    <div class="field-group">
      <div class="field-label text-xs font-semibold text-[var(--fg-muted)] mb-1">
        {i18n.t('subscriptions.interval')}
      </div>
      <Input
        type="text"
        value={String(interval)}
        oninput={(e) => {
          const val = Number((e.target as HTMLInputElement).value);
          if (!isNaN(val) && val > 0) interval = val;
        }}
      >
        {#snippet right()}
          <span class="text-xs font-medium text-[var(--fg-muted)] select-none pr-1.5 pointer-events-none">
            {i18n.t('subscriptions.minutes_unit')}
          </span>
        {/snippet}
      </Input>
    </div>

    <div class="flex items-center gap-2.5 mt-3 pt-3 border-t border-[var(--border-color)]">
      {#if subscription}
        <Button variant="danger" size="base" class="flex-1" onclick={unsubscribe} disabled={saving}>
          {i18n.t('subscriptions.unsubscribe') || i18n.t('selection.unsubscribe')}
        </Button>
      {/if}
      <Button variant="accent" size="base" class="flex-1" onclick={saveSubscription} disabled={saving}>
        {i18n.t('subscriptions.save')}
      </Button>
    </div>
  </div>
{/snippet}

{#snippet subscriptionControl()}
  {#if !subscription}
    <Button
      variant="ghost"
      onclick={subscribeDefault}
      disabled={saving}
      class="action-btn"
      title={i18n.t('subscriptions.subscribe')}
      aria-label={i18n.t('subscriptions.subscribe')}
    >
      <IconAdd class="w-[18px] h-[18px]" />
      <span class="btn-text">{i18n.t('subscriptions.subscribe')}</span>
    </Button>
  {:else}
    <PopoverMenu
      bind:open={subscriptionMenuOpen}
      title={i18n.t('subscriptions.subscription')}
      width="340px"
    >
      {#snippet trigger()}
        <Button
          variant="accent"
          onclick={() => { initEditorFields(); subscriptionMenuOpen = !subscriptionMenuOpen; }}
          class="action-btn"
          title={i18n.t('subscriptions.subscription')}
          aria-label={i18n.t('subscriptions.subscription')}
        >
          <IconSettings class="w-[18px] h-[18px]" />
          <span class="btn-text">{i18n.t('subscriptions.subscription')}</span>
        </Button>
      {/snippet}

      <div class="subscription-popover-form">
        {@render subscriptionEditorFields()}
      </div>
    </PopoverMenu>
  {/if}
{/snippet}

{#snippet filterInnerContent()}
  <span class="filter-label">{i18n.t('feed.format') || 'Format'}</span>
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

  <div class="floating-divider"></div>

  <span class="filter-label section-label">{i18n.t('feed.filters')}</span>
  <button
    type="button"
    class="view-option"
    class:active={onlyWithAttachments}
    use:ripple
    onclick={() => onlyWithAttachments = !onlyWithAttachments}
  >
    <Checkbox
      checked={onlyWithAttachments}
      onchange={(v) => onlyWithAttachments = v}
    />
    <span>
      <strong>{i18n.t('feed.with_attachments')}</strong>
      <small>{i18n.t('feed.with_attachments_desc')}</small>
    </span>
    <IconDocument class="view-option-icon" />
  </button>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey} onrefresh={refreshCreator}>
  {#snippet overlay()}
    <StickyHeader threshold={120}>
      {#snippet leading()}
        <Button variant="ghost" onclick={() => navigationState.back()} class="sticky-back-btn btn-icon" title={i18n.t('nav.back')}>
          <IconArrowLeft class="w-[20px] h-[20px]" />
        </Button>
        <div class="creator-header-avatar sticky-avatar">
          {#if effectiveAvatar}
            <img src={effectiveAvatar} alt={creatorName} onerror={() => avatarFailed = true} />
          {:else}
            <span class="sticky-initial">{initialLetter}</span>
          {/if}
        </div>
        <span class="sticky-post-title">{creatorName}</span>

        <Button
          variant={isFavorited ? 'accent' : 'ghost'}
          disabled={favoritingPending}
          onclick={toggleFavorite}
          class="btn-icon action-btn sticky-fav-btn"
          title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
          aria-label={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
        >
          {#if isFavorited}
            <IconHeartFilled class="w-[18px] h-[18px] fav-active-heart" />
          {:else}
            <IconHeart class="w-[18px] h-[18px]" />
          {/if}
        </Button>
      {/snippet}

      {#snippet trailing()}
        <HeaderActions
          bind:searchOpen={postSearchOpen}
          bind:searchQuery={postSearchQuery}
          searchPlaceholder={i18n.t('feed.search_placeholder')}
          onsearchtoggle={(open) => {
            if (!open) closePostSearch();
          }}
        >
          {#if !layoutState.isMobile}
            <PopoverMenu
              bind:open={stickyFiltersOpen}
              title={i18n.t('feed.filters')}
              icon={IconOptions}
              badge={activeFilterCount}
              active={activeFilterCount > 0}
            >
              {@render filterInnerContent()}
            </PopoverMenu>

            <Button
              variant="ghost"
              class="btn-icon action-btn"
              onclick={refreshCreator}
              disabled={entry.loading}
              title={i18n.t('feed.refresh') || 'Refresh'}
              aria-label="Refresh"
            >
              {#if entry.loading}
                <IconLoading class="w-[18px] h-[18px]" />
              {:else}
                <IconArrowClockwise class="w-[18px] h-[18px]" />
              {/if}
            </Button>

            <Button
              variant={isSelectionActive ? 'accent' : 'ghost'}
              class="btn-icon action-btn"
              onclick={() => (isSelectionActive ? selectionState.exit() : selectionState.enter('posts'))}
              title={i18n.t('selection.select_mode') || 'Select mode'}
              aria-label="Select mode"
            >
              <IconCheckboxChecked class="w-[18px] h-[18px]" />
            </Button>
          {:else}
            <PopoverMenu
              bind:open={stickyFiltersOpen}
              title={i18n.t('feed.filters')}
              icon={IconOptions}
              badge={activeFilterCount}
              active={activeFilterCount > 0}
            >
              {@render filterInnerContent()}
            </PopoverMenu>

            {#if isSelectionActive}
              <Button
                variant="accent"
                size="sm"
                class="px-2.5 h-[38px] text-xs font-semibold gap-1 rounded-full"
                onclick={() => selectionState.exit()}
                title={i18n.t('common.done') || 'Done'}
                aria-label="Exit selection mode"
              >
                <IconCheck class="w-4 h-4" />
                <span>{i18n.t('common.done') || 'Done'}</span>
              </Button>
            {:else}
              <Button
                variant="ghost"
                class="btn-icon action-btn"
                onclick={() => (mobileMoreOpen = true)}
                title={i18n.t('common.more') || 'More'}
                aria-label="More actions"
              >
                <IconMoreVertical class="w-5 h-5" />
              </Button>
            {/if}
          {/if}
        </HeaderActions>
      {/snippet}
    </StickyHeader>
  {/snippet}

  {#if effectiveBanner}
    <HeroBackdrop src={effectiveBanner} />
  {/if}

  <div class="post-content-wrapper">
    <!-- Top Action Bar -->
    <div class="post-actions-bar">
      {#if !layoutState.isMobile}
        <div class="left-actions flex items-center flex-wrap gap-2 min-w-0">
          <Button variant="ghost" onclick={() => navigationState.back()} class="action-btn">
            <IconArrowLeft class="w-[18px] h-[18px]" />
            <span>{i18n.t('nav.back')}</span>
          </Button>

          <Button
            variant={isFavorited ? 'accent' : 'ghost'}
            disabled={favoritingPending}
            onclick={toggleFavorite}
            class="action-btn"
            title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
          >
            {#if isFavorited}
              <IconHeartFilled class="w-[18px] h-[18px] fav-active-heart" />
            {:else}
              <IconHeart class="w-[18px] h-[18px]" />
            {/if}
            <span>{i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}</span>
          </Button>

          {@render subscriptionControl()}
        </div>
      {:else}
        <div class="left-actions flex items-center gap-2 min-w-0">
          <Button
            variant="ghost"
            onclick={() => navigationState.back()}
            class="action-btn btn-icon"
            title={i18n.t('nav.back')}
            aria-label={i18n.t('nav.back')}
          >
            <IconArrowLeft class="w-5 h-5" />
          </Button>

          <Button
            variant={isFavorited ? 'accent' : 'ghost'}
            disabled={favoritingPending}
            onclick={toggleFavorite}
            class="action-btn btn-icon"
            title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
            aria-label={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
          >
            {#if isFavorited}
              <IconHeartFilled class="w-5 h-5 fav-active-heart" />
            {:else}
              <IconHeart class="w-5 h-5" />
            {/if}
          </Button>

          {#if !subscription}
            <Button
              variant="ghost"
              onclick={subscribeDefault}
              disabled={saving}
              class="action-btn btn-icon"
              title={i18n.t('subscriptions.subscribe')}
              aria-label={i18n.t('subscriptions.subscribe')}
            >
              <IconAdd class="w-5 h-5" />
            </Button>
          {:else}
            <Button
              variant="accent"
              onclick={() => { initEditorFields(); mobileSubSettingsOpen = true; }}
              class="action-btn btn-icon"
              title={i18n.t('subscriptions.subscription')}
              aria-label={i18n.t('subscriptions.subscription')}
            >
              <IconSettings class="w-5 h-5" />
            </Button>
          {/if}
        </div>

        <div class="right-actions flex items-center gap-2 ml-auto">
          <Button
            variant="ghost"
            class="action-btn btn-icon"
            onclick={() => (mobileMoreOpen = true)}
            title={i18n.t('common.more') || 'More'}
            aria-label="More options"
          >
            <IconMoreVertical class="w-5 h-5" />
          </Button>
        </div>
      {/if}
    </div>

    <!-- Creator Header Info -->
    <header class="detail-header">
      <div class="creator-title-row">
        <div class="creator-header-avatar">
          {#if effectiveAvatar}
            <img
              src={effectiveAvatar}
              alt={creatorName}
              class="creator-avatar-img"
              onerror={() => avatarFailed = true}
            />
          {:else}
            <div class="creator-avatar-initial">
              {initialLetter}
            </div>
          {/if}
        </div>

        <h1>{creatorName}</h1>
      </div>

      <div class="post-date post-meta-row flex items-center flex-wrap gap-2 mt-2 min-h-[38px] text-sm text-[var(--fg-muted)]">
        <Button
          variant="ghost"
          onclick={openOriginalProfile}
          tooltip={`${i18n.t('creator.open_original_profile') || 'Open original profile'}: ${service}`}
          aria-label={`Open on ${service}`}
        >
          <ServiceIcon {service} class="w-4 h-4" />
          <span class="capitalize">{service}</span>
        </Button>

        <span class="text-[var(--fg-subtle)]">·</span>
        <Button
          variant="ghost"
          onclick={openInProvider}
          tooltip={`${i18n.t('creator.open_in_provider') || 'Open in provider'}: ${currentProviderName}`}
          aria-label={`Open in ${currentProviderName}`}
        >
          <IconOpen class="w-4 h-4" />
          <span>{currentProviderName}</span>
        </Button>

        <span class="text-[var(--fg-subtle)]">·</span>
        <Button
          variant="ghost"
          onclick={copyCreatorId}
          tooltip={i18n.t('common.copy') || 'Copy ID'}
        >
          <span class="font-mono text-[var(--fg-subtle)]">{creatorId}</span>
          {#if copiedId}
            <IconCheck class="w-[16px] h-[16px] text-accent" />
          {:else}
            <IconCopy class="w-[16px] h-[16px] opacity-60" />
          {/if}
        </Button>

        {#if candidateProviders.length > 1}
          <span class="text-[var(--fg-subtle)]">·</span>
          <div class="inline-flex items-center gap-1 shrink-0">
            <span class="text-xs text-[var(--fg-subtle)]">{i18n.t('post.source') || 'Source'}:</span>
            <Select
              variant="ghost"
              options={providerSelectOptions}
              value={activeProviderId}
              onchange={(val) => providerState.setSelectedProvider(service, creatorId, '*', val)}
            />
          </div>
        {/if}
      </div>
    </header>

    <div class="media-controls-row mt-2">
      <div class="creator-tabs-scroll">
        <ChoiceGroup
          options={creatorTabOptions}
          value={activeTab}
          onchange={(val) => activeTab = val as typeof activeTab}
          hasActiveAddon={(opt) => opt.value === 'posts'}
          align="left"
          class="creator-sections-choice"
        >
          {#snippet activeAddon()}
            <Select
              options={sortOptions}
              value={sortOrder}
              onchange={(v) => sortOrder = v as typeof sortOrder}
              class="creator-sort-select"
              icon={IconArrowSort}
              iconOnly={true}
              ariaLabel={`${i18n.t('favorites.sort_by') || 'Sort'}: ${currentSortLabel}`}
            />
          {/snippet}
        </ChoiceGroup>
      </div>

      <div class="media-controls-right">
        {#if activeTab === 'posts'}
          <HeaderActions
            bind:searchOpen={postSearchOpen}
            bind:searchQuery={postSearchQuery}
            searchPlaceholder={i18n.t('feed.search_placeholder')}
            onsearchtoggle={(open) => {
              if (!open) closePostSearch();
            }}
          >
            {#if !layoutState.isMobile}
              <PopoverMenu
                bind:open={filtersOpen}
                title={i18n.t('feed.filters')}
                icon={IconOptions}
                badge={activeFilterCount}
                active={activeFilterCount > 0}
              >
                {@render filterInnerContent()}
              </PopoverMenu>

              <Button
                variant={isSelectionActive ? 'accent' : 'ghost'}
                class="btn-icon"
                onclick={() => (isSelectionActive ? selectionState.exit() : selectionState.enter('posts'))}
                title={i18n.t('selection.select_mode') || 'Select mode'}
                aria-label="Select mode"
              >
                <IconCheckboxChecked class="w-5 h-5" />
              </Button>
            {:else}
              <PopoverMenu
                bind:open={filtersOpen}
                title={i18n.t('feed.filters')}
                icon={IconOptions}
                badge={activeFilterCount}
                active={activeFilterCount > 0}
              >
                {@render filterInnerContent()}
              </PopoverMenu>

              {#if isSelectionActive}
                <Button
                  variant="accent"
                  size="sm"
                  class="px-2.5 h-[38px] text-xs font-semibold gap-1 rounded-full"
                  onclick={() => selectionState.exit()}
                  title={i18n.t('common.done') || 'Done'}
                  aria-label="Exit selection mode"
                >
                  <IconCheck class="w-4 h-4" />
                  <span>{i18n.t('common.done') || 'Done'}</span>
                </Button>
              {/if}
            {/if}
          </HeaderActions>
        {/if}
      </div>
    </div>

    <!-- Active Tab Content -->
    {#if activeTab === 'posts'}
      <div class="creator-posts-section">
        {#if creatorTags.length > 0}
          <div class="creator-tags-bar mb-3">
            <TagList
              tags={creatorTags}
              activeTag={selectedTag}
              showAll={true}
              allLabel={i18n.t('common.all') || 'All'}
              allCount={entry.posts.length}
              size="sm"
              onclick={(tag) => {
                const norm = tag.trim().toLowerCase().replace(/^#+/, '');
                const currentNorm = selectedTag?.trim().toLowerCase().replace(/^#+/, '');
                if (currentNorm === norm || currentNorm?.replace(/_/g, ' ') === norm.replace(/_/g, ' ')) {
                  selectedTag = null;
                } else {
                  selectedTag = tag;
                }
              }}
              onclear={() => {
                selectedTag = null;
              }}
            />
          </div>
        {/if}

        {#if selectedTag && visibleCreatorPosts.length === 0}
          <div class="status-container empty py-12 text-center">
            <p class="text-sm font-medium text-[var(--text-secondary)]">
              {i18n.t('creator.no_posts_with_tag') || 'No posts found matching tag'} <span class="text-[var(--accent-primary)] font-semibold">#{selectedTag}</span>
            </p>
            <Button variant="tonal" size="sm" class="mt-3" onclick={() => selectedTag = null}>
              {i18n.t('creator.clear_tag_filter') || 'Clear tag filter'}
            </Button>
          </div>
        {:else if postSearchError && visibleCreatorPosts.length === 0}
          <div class="creator-error">{postSearchError}</div>
        {:else if entry.error && entry.posts.length === 0}
          <div class="creator-error">{entry.error}</div>
        {:else}
          <PostGrid
            posts={visibleCreatorPosts}
            loading={entry.loading || entry.loadingMore || (normalizedPostSearch.length >= 2 && postSearchLoading)}
            hasMore={normalizedPostSearch.length >= 2 ? postSearchHasMore : entry.hasMore}
            emptyTitle={postSearchQuery ? (i18n.t('feed.no_results') || 'No posts found') : (i18n.t('feed.empty') || 'No posts available')}
            ariaLabel={creatorName}
            onLoadMore={handleLoadMore}
          />
        {/if}
      </div>
    {:else if activeTab === 'similar'}
      <div class="creator-section-view">
        {#if similarCreators.length === 0}
          <div class="status-container empty">
            <IconSparkle />
            <strong>{i18n.t('creator.no_similar_artists')}</strong>
            <span>{i18n.t('creator.no_similar_artists_desc')}</span>
          </div>
        {:else}
          <div class="creator-cards-grid" style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-gap: ${gap}px;`}>
            {#each similarCreators as sim}
              {@const sService = String(sim.service ?? service)}
              {@const sId = String(sim.id ?? '')}
              {@const sName = String(sim.name ?? sId)}
              {@const simAvatar = creatorAvatarUrl(sService, sId, (sim.extra as any)?.avatar_thumbhash)}
              <article
                class="grid-tile"
                style:aspect-ratio={ratio}
              >
                <button
                  class="grid-tile-open"
                  type="button"
                  onclick={() => navigationState.openCreator(sService, sId)}
                  aria-label={sName}
                ></button>

                <div class="grid-tile-placeholder">
                  <span class="fallback-initials">{sName.slice(0, 2).toUpperCase()}</span>
                </div>

                {#if simAvatar}
                  <img
                    class="grid-tile-media"
                    src={simAvatar}
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
                      onclick={() => navigationState.openCreator(sService, sId)}
                      use:tooltip={sService}
                      aria-label={`${i18n.t('feed.open_creator')}: ${sService}`}
                    >
                      <ServiceIcon service={sService} />
                    </button>

                    <span
                      role="link"
                      tabindex="0"
                      class="grid-tile-author-name"
                      onclick={() => navigationState.openCreator(sService, sId)}
                      onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && navigationState.openCreator(sService, sId)}
                    >
                      {sName}
                    </span>
                  </div>

                  <div class="grid-tile-meta">
                    <span>{sService} · {sId}</span>
                  </div>
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </div>
    {:else if activeTab === 'links'}
      <div class="creator-section-view">
        {#if creatorLinks.length === 0}
          <div class="status-container empty">
            <IconGlobe />
            <strong>{i18n.t('creator.no_links') || 'No linked accounts found'}</strong>
            <span>{i18n.t('creator.no_links_desc') || 'This creator does not have other platform accounts linked yet.'}</span>
          </div>
        {:else}
          <div class="creator-cards-grid" style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-gap: ${gap}px;`}>
            {#each creatorLinks as link}
              {@const lService = String(link.service ?? '')}
              {@const lId = String(link.id ?? '')}
              {@const lName = String(link.name ?? lId)}
              {@const linkAvatar = creatorAvatarUrl(lService, lId, (link.extra as any)?.avatar_thumbhash)}
              <article
                class="grid-tile"
                style:aspect-ratio={ratio}
              >
                <button
                  class="grid-tile-open"
                  type="button"
                  onclick={() => navigationState.openCreator(lService, lId)}
                  aria-label={lName}
                ></button>

                <div class="grid-tile-placeholder">
                  <span class="fallback-initials">{lName.slice(0, 2).toUpperCase()}</span>
                </div>

                {#if linkAvatar}
                  <img
                    class="grid-tile-media"
                    src={linkAvatar}
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
                      onclick={() => navigationState.openCreator(lService, lId)}
                      use:tooltip={lService}
                      aria-label={`${i18n.t('feed.open_creator')}: ${lService}`}
                    >
                      <ServiceIcon service={lService} />
                    </button>

                    <span
                      role="link"
                      tabindex="0"
                      class="grid-tile-author-name"
                      onclick={() => navigationState.openCreator(lService, lId)}
                      onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && navigationState.openCreator(lService, lId)}
                    >
                      {lName}
                    </span>
                  </div>

                  <div class="grid-tile-meta">
                    <span>{lService} · {lId}</span>
                  </div>
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </div>
    {:else if activeTab === 'announcements'}
      <div class="creator-section-view">
        {#if announcements.length === 0}
          <div class="status-container empty">
            <IconNews />
            <strong>{i18n.t('creator.no_announcements') || 'No community posts found'}</strong>
            <span>{i18n.t('creator.no_announcements_desc') || 'There are no community posts or announcements from this creator yet.'}</span>
          </div>
        {:else}
          <div class="announcements-list">
            {#each announcements as ann}
              <article class="announcement-item">
                <div class="announcement-meta">
                  <span class="announcement-meta-label">{i18n.t('post.published_at')}:</span>
                  <strong class="announcement-meta-value">{formatDate(ann.added || (ann as any).published)}</strong>
                </div>

                <div class="announcement-body">
                  <RichContent html={ann.content} currentService={service} currentCreatorId={creatorId} />
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </div>
    {:else if activeTab === 'fancards'}
      <div class="creator-section-view">
        {#if fancards.length === 0}
          <div class="status-container empty">
            <IconImageOff />
            <strong>{i18n.t('creator.no_fancards') || 'No fancards found'}</strong>
            <span>{i18n.t('creator.no_fancards_desc') || 'This creator does not have any fancards available.'}</span>
          </div>
        {:else}
          <div class="creator-cards-grid" style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-gap: ${gap}px;`}>
            {#each fancards as card, index}
              {@const cardThumb = fancardThumbnailUrl(card, service)}
              {@const cardFull = fancardMediaUrl(card, service)}
              {@const ext = (card.ext || card.mime?.split('/').pop() || 'IMG').replace(/^\.+/, '').toUpperCase()}
              <article
                class="grid-tile"
                style:aspect-ratio={ratio}
              >
                <button
                  class="grid-tile-open"
                  type="button"
                  onclick={() => openFancardViewer(index)}
                  aria-label={`Fancard ${card.id}`}
                ></button>

                <div class="grid-tile-placeholder">
                  <span class="fallback-initials">{ext}</span>
                </div>

                {#if cardThumb}
                  <img
                    class="grid-tile-media"
                    src={cardThumb}
                    alt=""
                    loading="lazy"
                    decoding="async"
                    onerror={(e) => {
                      const target = e.currentTarget as HTMLImageElement;
                      if (cardFull && target.src !== cardFull) {
                        target.src = cardFull;
                      } else {
                        target.style.display = 'none';
                      }
                    }}
                  />
                {:else if cardFull}
                  <img class="grid-tile-media" src={cardFull} alt="" loading="lazy" decoding="async" />
                {/if}

                <div class="grid-tile-shade"></div>

                <div class="grid-tile-footer">
                  <div class="grid-tile-author">
                    <button
                      type="button"
                      class="grid-tile-logo inline-logo"
                      onclick={() => openFancardViewer(index)}
                      use:tooltip={service}
                      aria-label={`${i18n.t('feed.open_creator')}: ${service}`}
                    >
                      <ServiceIcon {service} />
                    </button>

                    <span class="grid-tile-author-name">
                      {ext} {card.size ? `· ${formatBytes(card.size)}` : ''}
                    </span>
                  </div>

                  <div class="grid-tile-meta">
                    <span>{formatDate(card.added)}</span>
                  </div>
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</PageShell>

{#if isViewerOpen && viewerItems.length > 0}
  <MediaViewer
    items={viewerItems}
    initialIndex={viewerActiveIndex}
    onclose={() => isViewerOpen = false}
  />
{/if}

<SelectionActionBar
  totalCount={visibleCreatorPosts.length}
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

{#if layoutState.isMobile}
  <BottomSheet
    open={mobileMoreOpen}
    title={creatorName}
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
            selectionState.enter('posts');
          }
        }}
      >
        <IconCheckboxChecked class={isSelectionActive ? 'text-accent' : 'text-secondary'} />
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{isSelectionActive ? (i18n.t('selection.exit') || 'Exit selection mode') : (i18n.t('selection.select_mode') || 'Select posts')}</span>
        </div>
      </button>

      <button
        type="button"
        class="sheet-action-item"
        disabled={entry.loading}
        use:ripple
        onclick={() => {
          mobileMoreOpen = false;
          void refreshCreator();
        }}
      >
        {#if entry.loading}
          <IconLoading class="text-accent" />
        {:else}
          <IconArrowClockwise class="text-secondary" />
        {/if}
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{i18n.t('feed.refresh') || 'Refresh'}</span>
        </div>
      </button>

      {#if subscription}
        <button
          type="button"
          class="sheet-action-item"
          use:ripple
          onclick={() => {
            mobileMoreOpen = false;
            initEditorFields();
            mobileSubSettingsOpen = true;
          }}
        >
          <IconSettings class="text-secondary" />
          <div class="flex flex-col min-w-0">
            <span class="text-sm font-semibold text-primary">{i18n.t('subscriptions.subscription')}</span>
            <span class="text-xs text-[var(--fg-muted)]">{i18n.t('subscriptions.destination')}: {stashes.find(s => s.id === subscription?.destination_collection_id)?.name || 'Default'}</span>
          </div>
        </button>
      {:else}
        <button
          type="button"
          class="sheet-action-item"
          disabled={saving}
          use:ripple
          onclick={() => {
            mobileMoreOpen = false;
            void subscribeDefault();
          }}
        >
          <IconAdd class="text-accent" />
          <div class="flex flex-col min-w-0">
            <span class="text-sm font-semibold text-primary">{i18n.t('subscriptions.subscribe')}</span>
          </div>
        </button>
      {/if}

      <button
        type="button"
        class="sheet-action-item"
        use:ripple
        onclick={() => {
          mobileMoreOpen = false;
          openOriginalProfile();
        }}
      >
        <ServiceIcon {service} />
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{i18n.t('creator.open_original_profile') || 'Open original profile'}</span>
          <span class="text-xs text-muted capitalize">{service}</span>
        </div>
      </button>

      <button
        type="button"
        class="sheet-action-item"
        use:ripple
        onclick={() => {
          mobileMoreOpen = false;
          openInProvider();
        }}
      >
        <IconOpen class="text-secondary" />
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{i18n.t('creator.open_in_provider') || 'Open in provider'}</span>
          <span class="text-xs text-muted">{currentProviderName}</span>
        </div>
      </button>

      <button
        type="button"
        class="sheet-action-item"
        use:ripple
        onclick={() => {
          void copyCreatorId();
        }}
      >
        {#if copiedId}
          <IconCheck class="text-accent" />
        {:else}
          <IconCopy class="text-secondary" />
        {/if}
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{i18n.t('common.copy') || 'Copy ID'}</span>
          <span class="text-xs text-[var(--fg-muted)] font-mono">{creatorId}</span>
        </div>
      </button>

      {#if candidateProviders.length > 1}
        <div class="px-4 py-3 border-t border-[var(--border-color)]">
          <div class="text-xs font-semibold text-[var(--fg-muted)] mb-1.5">{i18n.t('post.source') || 'Provider'}</div>
          <Select
            variant="ghost"
            options={providerSelectOptions}
            value={activeProviderId}
            onchange={(val) => providerState.setSelectedProvider(service, creatorId, '*', val)}
          />
        </div>
      {/if}
    </div>
  </BottomSheet>

  <BottomSheet
    open={mobileSubSettingsOpen}
    title={i18n.t('subscriptions.subscription')}
    onclose={() => (mobileSubSettingsOpen = false)}
  >
    {@render subscriptionEditorFields()}
  </BottomSheet>
{/if}

<style>
  .post-content-wrapper {
    position: relative;
    z-index: 2;
    min-width: 0;
    max-width: 100%;
    overflow-x: clip;
  }

  .post-actions-bar {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 20px;
    padding-bottom: 14px;
    z-index: 10;
    min-width: 0;
    max-width: 100%;
  }

  .post-actions-bar :global(.btn),
  .media-controls-row :global(.btn),
  :global(.sticky-leading-zone .btn) {
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 18px !important;
    font-size: calc(var(--control-font-size, 14px) * var(--ui-scale, 1)) !important;
    border-radius: var(--radius-full) !important;
    gap: 8px !important;
  }

  .post-actions-bar :global(.btn.btn-icon),
  .media-controls-row :global(.btn.btn-icon),
  :global(.sticky-leading-zone .btn.btn-icon),
  :global(.sticky-trailing-zone .btn.btn-icon) {
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 !important;
    border-radius: 50% !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  :global(.page-shell.mobile) .post-actions-bar,
  :global(.page-shell.is-mobile) .post-actions-bar {
    margin-bottom: 8px;
    padding-bottom: 6px;
    gap: 6px;
  }

  @media (max-width: 640px) {
    .post-actions-bar {
      margin-bottom: 8px;
      padding-bottom: 6px;
      gap: 6px;
    }
  }

  .post-actions-bar :global(.btn svg),
  .media-controls-row :global(.btn svg) {
    width: 20px;
    height: 20px;
  }

  .post-actions-bar :global(.fav-active-heart) {
    color: var(--text-on-accent, var(--text-primary));
  }

  .detail-header {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding-bottom: 14px;
    z-index: 10;
    min-width: 0;
    max-width: 100%;
  }

  .creator-title-row {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
    max-width: 100%;
  }

  .creator-header-avatar {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    overflow: hidden;
    flex-shrink: 0;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    display: grid;
    place-items: center;
  }

  .creator-avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .creator-avatar-initial {
    font-size: 20px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .creator-title-row h1 {
    margin: 0;
    color: white;
    font-family: var(--font-sans);
    font-size: clamp(28px, 4.5vw, 42px);
    font-weight: var(--font-weight-normal);
    line-height: 1.12;
    word-break: break-word;
    overflow-wrap: break-word;
    min-width: 0;
  }

  :global(.page-shell.mobile) .detail-header,
  :global(.page-shell.is-mobile) .detail-header {
    padding-bottom: 4px;
  }

  :global(.page-shell.mobile) .creator-title-row,
  :global(.page-shell.is-mobile) .creator-title-row {
    gap: 10px;
  }

  :global(.page-shell.mobile) .creator-title-row h1,
  :global(.page-shell.is-mobile) .creator-title-row h1 {
    font-size: clamp(22px, 6vw, 30px);
  }

  :global(.page-shell.mobile) .creator-tags-bar,
  :global(.page-shell.is-mobile) .creator-tags-bar {
    margin-bottom: 10px;
  }

  @media (max-width: 640px) {
    .detail-header {
      padding-bottom: 4px;
    }
    .creator-title-row {
      gap: 10px;
    }
    .creator-title-row h1 {
      font-size: clamp(22px, 6vw, 30px);
    }
    .creator-tags-bar {
      margin-bottom: 10px;
    }
  }

  .post-date {
    margin-top: 6px;
    color: rgba(255, 255, 255, 0.4);
    font-size: 12px;
  }

  .creator-tags-bar {
    margin-bottom: 14px;
    min-width: 0;
    max-width: 100%;
  }

  .media-controls-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
    min-width: 0;
    max-width: 100%;
    --control-height: var(--control-height-md, 46px);
    --control-font-size: var(--control-font-md, 14px);
    --control-icon-size: var(--control-icon-md, 20px);
    --control-padding-x: var(--control-padding-md, 20px);
  }

  .creator-tabs-scroll {
    display: flex;
    align-items: center;
    flex: 1 1 auto;
    min-width: 0;
    max-width: 100%;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
    -webkit-mask-image: linear-gradient(to right, black calc(100% - 24px), transparent 100%);
    mask-image: linear-gradient(to right, black calc(100% - 24px), transparent 100%);
    padding-right: 20px;
  }

  .creator-tabs-scroll::-webkit-scrollbar {
    display: none;
  }

  .creator-tabs-scroll :global(.choice-group) {
    flex-wrap: nowrap !important;
    max-width: none !important;
    flex-shrink: 0 !important;
  }

  .media-controls-right {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    flex-shrink: 0;
  }

  :global(.page-shell.mobile) .media-controls-row:has(:global(.search-active)) .creator-tabs-scroll,
  :global(.page-shell.is-mobile) .media-controls-row:has(:global(.search-active)) .creator-tabs-scroll {
    display: none !important;
  }

  @media (max-width: 640px) {
    .media-controls-row:has(:global(.search-active)) .creator-tabs-scroll {
      display: none !important;
    }
  }

  :global(.page-shell.mobile) .media-controls-row:has(:global(.search-active)) .media-controls-right,
  :global(.page-shell.is-mobile) .media-controls-row:has(:global(.search-active)) .media-controls-right {
    width: 100% !important;
    flex: 1 !important;
    margin-left: 0 !important;
  }

  @media (max-width: 640px) {
    .media-controls-row:has(:global(.search-active)) .media-controls-right {
      width: 100% !important;
      flex: 1 !important;
      margin-left: 0 !important;
    }
  }

  :global(.creator-sections-choice) {
    flex-shrink: 0 !important;
  }

  :global(.creator-sort-select) {
    width: auto !important;
    max-width: none !important;
    flex-shrink: 0 !important;
  }

  :global(.creator-sort-select .select-trigger),
  :global(.creator-sort-select .select-trigger.icon-only) {
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

  :global(.creator-sort-select .select-trigger:hover),
  :global(.creator-sort-select .select-trigger.icon-only:hover) {
    background: color-mix(in srgb, var(--accent-container) 70%, var(--accent-primary)) !important;
    color: var(--choice-active-bg, var(--accent-on-container)) !important;
  }

  :global(.creator-sort-select .select-trigger:active),
  :global(.creator-sort-select .select-trigger.icon-only:active) {
    opacity: 0.85 !important;
  }

  :global(.creator-sort-select .select-trigger svg),
  :global(.creator-sort-select .select-trigger.icon-only svg) {
    width: 20px !important;
    height: 20px !important;
    color: var(--choice-active-bg, var(--accent-on-container)) !important;
    opacity: 1 !important;
  }

  :global(.sticky-header-bar) :global(.sticky-back-btn) {
    flex: 0 0 calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    border-radius: 50% !important;
    padding: 0 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
    flex-shrink: 0 !important;
  }

  :global(.sticky-header-bar) :global(.sticky-back-btn svg) {
    width: 20px !important;
    height: 20px !important;
    flex-shrink: 0 !important;
  }

  .sticky-avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    overflow: hidden;
    display: grid;
    place-items: center;
    background: var(--bg-card);
    flex-shrink: 0;
  }

  .sticky-avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .sticky-initial {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .sticky-post-title {
    font-family: var(--font-sans);
    color: var(--text-primary);
    font-size: 16px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.95;
    text-align: left;
    min-width: 0;
  }

  :global(.sticky-header-bar) :global(.sticky-fav-btn) {
    width: 36px !important;
    height: 36px !important;
    min-width: 36px !important;
    padding: 0 !important;
    border-radius: 50% !important;
    flex-shrink: 0 !important;
  }

  .creator-section-view {
    padding: 16px 0;
  }

  .status-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 6px;
    min-height: 280px;
    color: rgba(255, 255, 255, 0.42);
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

  .creator-cards-grid {
    position: relative;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--grid-card-width, 245px)), 1fr));
    gap: var(--grid-gap, 10px);
    width: 100%;
  }

  .fallback-initials {
    font-family: var(--font-sans);
    font-size: calc(30px * var(--grid-scale, 1));
    font-weight: 700;
    letter-spacing: 0.05em;
    color: rgba(255, 255, 255, 0.22);
  }

  .announcements-list {
    display: flex;
    flex-direction: column;
    gap: 32px;
    max-width: 840px;
  }

  .announcement-item {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-bottom: 32px;
    border-bottom: 1px solid var(--border-color);
  }

  .announcement-item:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .announcement-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13.5px;
    color: var(--text-muted);
  }

  .announcement-meta-label {
    color: var(--text-muted);
  }

  .announcement-meta-value {
    font-weight: 600;
    color: var(--text-primary);
  }

  .announcement-body {
    font-size: 15px;
    line-height: 1.7;
    color: var(--text-primary);
    user-select: text;
  }

  .announcement-body :global(img) {
    border-radius: var(--radius-lg, 12px);
    margin-top: 14px;
    max-width: 100%;
    height: auto;
  }

  .creator-error {
    min-height: 300px;
    display: grid;
    place-items: center;
    color: var(--text-muted);
    font-size: 14px;
  }


  .subscription-popover-form {
    padding: 8px 10px 6px;
    box-sizing: border-box;
  }
</style>
