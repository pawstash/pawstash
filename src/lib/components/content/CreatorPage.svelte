<script lang="ts">
  import { edgeFade } from '$lib/actions/edgeFade';
  import { onMount, tick, untrack } from 'svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import { contentState, creatorCacheKey, type CachedCreator } from '$lib/state/contentState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import { favoriteActions } from '$lib/state/favoriteActions.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { providerState } from '$lib/state/providerState.svelte';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import {
    apiRevalidateCreatorArtwork,
    apiFetchCreatorProfile,
    apiFetchCreatorPosts,
    apiFetchCreatorLinks,
    apiFetchSimilarCreators,
    apiFetchCreatorTags,
    apiFetchAnnouncements,
    apiFetchFancards,
    openExternalUrl,
    apiResolveCreatorUrl
  } from '$lib/utils/ipc';
  import {
    creatorAvatarSrc,
    creatorPlaceholderUrl,
    creatorBannerSrc,
    fancardMediaUrl,
    fancardThumbnailUrl,
    formatProviderName,
    getPlatformProfileUrl,
    matchesPostFormatFilter,
    formatServiceName
  } from '$lib/utils/media';
  import { extraField } from '$lib/utils/fields';
  import { seedFromImageUrl, seedFromThumbHash } from '$lib/theme/seedColor';
  import { parseTags, getPostTags, formatDate, formatBytes, parseDateTimestamp } from '$lib/utils/formatters';
  import { logger } from '$lib/utils/logger';
  import type { Post, CreatorProfile, Announcement, Fancard } from '$lib/types/content';
  import type { FilterMap } from '$lib/types/filter';
  import { countActiveFilters, matchesTriStateFilter, toggleFilterKey } from '$lib/types/filter';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import SortControl from '$lib/components/layout/SortControl.svelte';
  import { defaultSortOption, sortOption } from '$lib/utils/sort';
  import type { MenuItem } from '$lib/components/layout/OverflowMenu.svelte';
  import HeroBackdrop from '$lib/components/ui/HeroBackdrop.svelte';
  import TagList from '$lib/components/ui/TagList.svelte';
  import PostGrid from './PostGrid.svelte';
  import RichContent from './RichContent.svelte';
  import MediaViewer, { type MediaViewerItem } from './MediaViewer.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import ChoiceGroup, { type ChoiceOption } from '$lib/components/ui/ChoiceGroup.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import { ripple } from '$lib/motion';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import IconAlert from '~icons/fluent/alert-24-regular';
  import IconAlertOn from '~icons/fluent/alert-on-24-filled';
  import IconSource from '~icons/fluent/database-24-regular';
  import IconNumberSymbol from '~icons/fluent/number-symbol-24-regular';
  import ChipSelect from '$lib/components/layout/ChipSelect.svelte';
  import IconHeart from '~icons/fluent/heart-24-regular';
  import IconHeartFilled from '~icons/fluent/heart-24-filled';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconBookmarkAdd from '~icons/fluent/bookmark-add-24-regular';
  import IconOptions from '~icons/fluent/options-24-regular';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconDraft from '~icons/fluent/drafts-24-regular';
  import IconText from '~icons/fluent/document-text-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconNews from '~icons/fluent/news-24-regular';
  import IconImageOff from '~icons/fluent/image-off-24-regular';
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import IconWarning from '~icons/fluent/warning-24-regular';
  import { notify } from '$lib/utils/toast';
  import { copyText } from '$lib/utils/clipboard';
  import { notifyAddedToStash, notifyRemovedFromStash } from '$lib/utils/stashToast';
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
    providerFilters?: FilterMap;
    onlyWithAttachments?: boolean;
    activeTab?: 'posts' | 'similar' | 'links' | 'announcements' | 'fancards';
    sortOrder?: 'default' | 'newest' | 'oldest' | 'popular';
    selectedTag?: string | null;
    selectedProviderId?: string;
  }>(navigationState.entryKey);

  onMount(() => {
    if (savedState?.selectedProviderId) {
      providerState.setSelectedProvider(service, creatorId, '*', savedState.selectedProviderId);
    }
  });

  let candidateProviders = $derived(providerState.getProvidersForService(service));
  let activeProviderId = $derived(
    candidateProviders.length === 1
      ? candidateProviders[0].id
      : providerState.getSelectedProvider(service, creatorId, '*')
  );
  let providerSelectOptions = $derived.by(() => {
    if (candidateProviders.length <= 1) {
      return candidateProviders.map((p) => ({
        value: p.id,
        label: formatProviderName(p.name)
      }));
    }
    return [
      { value: 'auto', label: i18n.t('post.source_auto') },
      ...candidateProviders.map((p) => ({
        value: p.id,
        label: formatProviderName(p.name)
      }))
    ];
  });

  function handleProviderChange(val: string) {
    if (activeProviderId === val) return;
    providerState.setSelectedProvider(service, creatorId, '*', val);
    providerFilters = {};
    selectedTag = null;
    postSearchQuery = '';
    postSearchResults = [];
    avatarErrorIndex = 0;
    avatarLoaded = false;
    bannerFailed = false;
  }

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

  let autoEntry = $derived(contentState.creators[creatorCacheKey(service, creatorId)]);
  let entry = $derived.by(() => contentState.creators[creatorCacheKey(service, creatorId, activeProviderId)] ?? emptyEntry);
  let canonicalProfile = $derived(autoEntry?.profile ?? contentState.creators[creatorCacheKey(service, creatorId)]?.profile ?? null);
  let profile = $derived(entry.profile ?? canonicalProfile ?? null);
  let subscription = $derived(subscriptionState.forCreator(service, creatorId));
  let isFavorited = $derived(accountState.isCreatorFavorite(service, creatorId));

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
  let hideAi = $state(false);
  let providerFilters = $state<FilterMap>(savedState?.providerFilters ?? {});
  let onlyWithAttachments = $state<boolean>(savedState?.onlyWithAttachments ?? false);
  let filtersOpen = $state(false);
  let creatorTitleEl = $state<HTMLElement | null>(null);
  let activeFilterCount = $derived(
    countActiveFilters([formatFilters, providerFilters]) + (onlyWithAttachments ? 1 : 0) + (hideAi ? 1 : 0)
  );

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
    if (fmt === 'ai') {
      hideAi = !hideAi;
      return;
    }
    formatFilters = toggleFilterKey(formatFilters, fmt);
  }

  function toggleProvider(providerId: string) {
    providerFilters = toggleFilterKey(providerFilters, providerId);
  }

  let formatList = $derived([
    { id: 'image', label: () => i18n.t('feed.format_photo'), icon: IconImage },
    { id: 'video', label: () => i18n.t('feed.format_video'), icon: IconVideo },
    { id: 'audio', label: () => i18n.t('feed.format_audio'), icon: IconMusic },
    { id: 'text', label: () => i18n.t('feed.format_text'), icon: IconText },
    { id: 'archive', label: () => i18n.t('feed.format_archive'), icon: IconDocument },
    { id: 'wip', label: () => i18n.t('feed.format_wip'), icon: IconDraft },
    ...(!configState.settings.pawchive_hide_ai ? [{ id: 'ai', label: () => i18n.t('feed.hide_ai'), icon: IconSparkle }] : [])
  ]);

  $effect(() => {
    navigationState.saveViewState(navigationState.entryKey, {
      postSearchQuery,
      postSearchOpen,
      postSearchResults,
      formatFilters: $state.snapshot(formatFilters),
      providerFilters: $state.snapshot(providerFilters),
      onlyWithAttachments,
      activeTab,
      sortOrder,
      selectedTag,
      selectedProviderId: activeProviderId
    });
  });

  let creatorName = $derived.by<string>(() => {
    if (typeof profile?.name === 'string' && profile.name !== creatorId) {
      return profile.name;
    }
    const fromMap = creatorsState.creatorsMap.get(`${service.toLowerCase()}:${creatorId.toLowerCase()}`);
    if (fromMap && fromMap !== creatorId) {
      return fromMap;
    }
    return (typeof profile?.name === 'string' && profile.name) || String(creatorId);
  });
  let avatarLoaded = $state(false);
  let avatarErrorIndex = $state(0);
  let bannerFailed = $state(false);
  let avatarPlaceholder = $derived(creatorPlaceholderUrl(profile));
  let avatarUrl = $derived(creatorAvatarSrc(profile));
  let bannerUrl = $derived(creatorBannerSrc(profile));
  let candidateAvatarUrls = $derived.by<string[]>(() => {
    const extraCandidate = extraField(profile, 'candidate_avatar_urls');
    const candidates = [
      avatarUrl,
      ...(Array.isArray(extraCandidate) ? (extraCandidate as string[]) : []),
      profile !== canonicalProfile ? creatorAvatarSrc(canonicalProfile) : ''
    ];
    return [...new Set(candidates.filter(Boolean))];
  });
  let effectiveAvatar = $derived(candidateAvatarUrls[avatarErrorIndex] ?? null);
  let effectiveBanner = $derived(bannerFailed ? null : bannerUrl);

  function handleAvatarError() {
    avatarErrorIndex++;
  }

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

  let rawUpdated = $derived(profile?.updated || extraField<string | number>(profile, 'updated_at') || null);
  let rawIndexed = $derived(profile?.indexed || extraField<string | number>(profile, 'indexed_at') || null);
  let latestPost = $derived(entry.posts.length > 0 ? entry.posts[0] : null);
  let rawLatestPostDate = $derived(latestPost?.published || latestPost?.added || null);

  let latestPostDateStr = $derived(rawLatestPostDate ? formatDate(rawLatestPostDate) : '');
  let updatedDateStr = $derived(rawUpdated && rawUpdated !== 0 && rawUpdated !== '0' ? formatDate(rawUpdated) : '');
  let indexedDateStr = $derived(rawIndexed && rawIndexed !== 0 && rawIndexed !== '0' ? formatDate(rawIndexed) : '');

  let showUpdated = $derived(Boolean(updatedDateStr));

  let showIndexed = $derived(Boolean(
    indexedDateStr &&
    (!showUpdated || indexedDateStr !== updatedDateStr)
  ));

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
      const direct = parseTags(extraField(profile, 'tags') || extraField(profile, 'categories'));
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

  const POPULARITY_FIELDS = ['favorite_count', 'favorites', 'favs', 'likes', 'likeCount', 'like_count', 'bookmarked', 'bookmarks', 'fav_count', 'score'];

  function popularity(post: Post): number {
    for (const key of POPULARITY_FIELDS) {
      const value = extraField(post, key);
      if (value !== undefined) return Number(value) || 0;
    }
    return 0;
  }

  let normalizedPostSearch = $derived(postSearchQuery.trim().toLocaleLowerCase());
  let visibleCreatorPosts = $derived.by(() => {
    let posts = entry.posts;

    if (configState.settings.pawchive_hide_ai || hideAi) {
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
      posts = posts.filter((post) => matchesPostFormatFilter(post, formatFilters));
    }

    if (Object.keys(providerFilters).length > 0) {
      posts = posts.filter((post) => {
        const raw = extraField(post, 'available_providers');
        const postProviders: string[] = Array.isArray(raw) && raw.length > 0
          ? (raw as string[])
          : [extraField<string>(post, 'provider_id') || providerState.getProviderIdForService(post.service)];
        return matchesTriStateFilter(postProviders, providerFilters);
      });
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
        const diff = popularity(b) - popularity(a);
        if (diff !== 0) return diff;
        const da = parseDateTimestamp(a.published || a.added);
        const db = parseDateTimestamp(b.published || b.added);
        return db - da;
      });
    }

    return posts;
  });

  let hasPopularityData = $derived(entry.posts.some((p) => popularity(p) > 0));

  let sortOptions = $derived.by(() => {
    const opts = [
      defaultSortOption('default'),
      sortOption('newest', 'published', 'desc'),
      sortOption('oldest', 'published', 'asc')
    ];
    if (hasPopularityData) {
      opts.push(sortOption('popular', 'popularity', 'desc'));
    }
    return opts;
  });

  $effect(() => {
    if (sortOrder === 'popular' && entry.loaded && !hasPopularityData) {
      sortOrder = 'default';
    }
  });

  let isPostsFiltered = $derived(
    Boolean(
      selectedTag ||
      onlyWithAttachments ||
      Object.keys(formatFilters).length > 0 ||
      Object.keys(providerFilters).length > 0 ||
      normalizedPostSearch
    )
  );

  let creatorTabOptions = $derived.by(() => {
    let postsCount: string | number | undefined;
    const rawPostCount = (profile?.extra as any)?.post_count ?? (profile?.extra as any)?.postCount;
    const totalProfileCount = typeof rawPostCount === 'number' && rawPostCount > 0
      ? rawPostCount
      : (typeof rawPostCount === 'string' && Number(rawPostCount) > 0 ? Number(rawPostCount) : undefined);

    if (isPostsFiltered) {
      postsCount = visibleCreatorPosts.length;
    } else if (entry.posts.length > 0) {
      if (totalProfileCount && totalProfileCount > entry.posts.length && entry.hasMore) {
        postsCount = `${entry.posts.length} / ${totalProfileCount}`;
      } else if (totalProfileCount) {
        postsCount = totalProfileCount;
      } else {
        postsCount = `${entry.posts.length}${entry.hasMore ? '+' : ''}`;
      }
    } else if (entry.loaded || entry.error) {
      postsCount = 0;
    } else if (totalProfileCount) {
      postsCount = totalProfileCount;
    }

    const list: Array<ChoiceOption<'posts' | 'similar' | 'links' | 'announcements' | 'fancards'>> = [
      {
        value: 'posts',
        label: i18n.t('creator.posts'),
        count: postsCount
      }
    ];

    if (similarCreators.length > 0) {
      list.push({
        value: 'similar',
        label: i18n.t('creator.similar_artists'),
        count: similarCreators.length
      });
    }

    if (creatorLinks.length > 0) {
      list.push({
        value: 'links',
        label: i18n.t('creator.linked_accounts'),
        count: creatorLinks.length
      });
    }

    if (announcements.length > 0) {
      list.push({
        value: 'announcements',
        label: i18n.t('creator.announcements'),
        count: announcements.length
      });
    }

    if (fancards.length > 0) {
      list.push({
        value: 'fancards',
        label: i18n.t('creator.fancards'),
        count: fancards.length
      });
    }

    return list;
  });

  $effect(() => {
    const isTabAvailable = creatorTabOptions.some((opt) => opt.value === activeTab);
    if (!isTabAvailable) {
      activeTab = 'posts';
    }
  });

  $effect(() => {
    if (!postSearchOpen) return;
    void tick().then(() => {
      const input = document.querySelector<HTMLInputElement>('.media-controls-right .search-input-field');
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
      const posts = await apiFetchCreatorPosts(service, creatorId, query, offset, activeProviderId);
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
              const nextBatch = await apiFetchCreatorPosts(service, creatorId, query, postSearchOffset, activeProviderId);
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

  async function loadExtraData(providerId = activeProviderId) {
    apiFetchSimilarCreators(service, creatorId, providerId).then((res) => {
      const map = new Map<string, CreatorProfile>();
      for (const item of (res || [])) {
        const key = `${item.service || service}:${item.id}`;
        if (!map.has(key) && !(item.id === creatorId && (item.service || service) === service)) {
          map.set(key, item);
        }
      }
      similarCreators = Array.from(map.values());
    }).catch(() => {});

    apiFetchCreatorLinks(service, creatorId, providerId).then((res) => {
      creatorLinks = res || [];
    }).catch(() => {});

    apiFetchAnnouncements(service, creatorId, providerId).then((res) => {
      announcements = res || [];
    }).catch(() => {});

    apiFetchFancards(service, creatorId, providerId).then((res) => {
      fancards = res || [];
    }).catch(() => {});

    apiFetchCreatorTags(service, creatorId, providerId).then((res) => {
      if (res && res.length > 0) {
        apiCreatorTags = res;
      }
    }).catch(() => {});
  }

  let lastLoadedCreatorKey = '';
  $effect(() => {
    const currentService = service;
    const currentCreatorId = creatorId;
    const currentProvider = activeProviderId;
    const currentKey = `${currentService}:${currentCreatorId}:${currentProvider}`;
    if (currentService && currentCreatorId) {
      if (lastLoadedCreatorKey !== currentKey) {
        lastLoadedCreatorKey = currentKey;
        avatarLoaded = false;
        avatarErrorIndex = 0;
        bannerFailed = false;
        similarCreators = [];
        creatorLinks = [];
        announcements = [];
        fancards = [];
        apiCreatorTags = [];
      }
      untrack(() => {
        void contentState.loadCreator(currentService, currentCreatorId, true, currentProvider).then(() => {
          void apiRevalidateCreatorArtwork(currentService, currentCreatorId, 'avatar', currentProvider).then((newPath) => {
            if (newPath && entry.profile && entry.profile.avatar_path !== newPath) {
              entry.profile.avatar_path = newPath;
            }
          }).catch(() => {});
          void apiRevalidateCreatorArtwork(currentService, currentCreatorId, 'banner', currentProvider).then((newPath) => {
            if (newPath && entry.profile && entry.profile.banner_path !== newPath) {
              entry.profile.banner_path = newPath;
            }
          }).catch(() => {});
        });
        void loadExtraData(currentProvider);
        void checkFavoriteStatus();
      });
    }
  });

  $effect(() => {
    const s = service;
    const c = creatorId;
    const p = activeProviderId;
    return () => {
      contentState.stopAutoFetchCreatorPosts(s, c, p);
    };
  });

  $effect(() => {
    if (creatorName === creatorId) {
      void apiFetchCreatorProfile(service, creatorId, activeProviderId).then((p) => {
        if (p && p.name && p.name !== creatorId) {
          const key = creatorCacheKey(service, creatorId, activeProviderId);
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
        contentState.refreshCreator(service, creatorId, activeProviderId).then(() => {
          void apiRevalidateCreatorArtwork(service, creatorId, 'avatar', activeProviderId).then((newPath) => {
            if (newPath && entry.profile && entry.profile.avatar_path !== newPath) {
              entry.profile.avatar_path = newPath;
            }
          }).catch(() => {});
          void apiRevalidateCreatorArtwork(service, creatorId, 'banner', activeProviderId).then((newPath) => {
            if (newPath && entry.profile && entry.profile.banner_path !== newPath) {
              entry.profile.banner_path = newPath;
            }
          }).catch(() => {});
        }),
        loadExtraData(activeProviderId),
        checkFavoriteStatus()
      ]);
    } catch (error) {
      notify.error(i18n.t('feed.refresh_failed'), error);
    }
  }

  async function getCreatorAccentColor(hasBanner: boolean, hasAvatar: boolean) {
    if (hasBanner && effectiveBanner) {
      try {
        const color = await seedFromImageUrl(effectiveBanner);
        if (color) return color;
      } catch (error) {
        logger.warn(`Failed to extract creator banner accent for ${service}:${creatorId}`, error);
      }
    }
    if (hasAvatar && effectiveAvatar) {
      try {
        const color = await seedFromImageUrl(effectiveAvatar);
        if (color) return color;
      } catch (error) {
        logger.warn(`Failed to extract creator avatar accent for ${service}:${creatorId}`, error);
      }
    }
    return '';
  }

  $effect(() => {
    const dynamicAccent = configState.settings.dynamic_accent;
    if (!dynamicAccent) return;

    let cancelled = false;
    const cachedAccent = contentState.getCreatorAccent(service, creatorId);
    const headerThumb = (profile?.extra as any)?.header_thumbhash;
    const avatarThumb = (profile?.extra as any)?.avatar_thumbhash;
    const thumbColor = cachedAccent || seedFromThumbHash(headerThumb) || seedFromThumbHash(avatarThumb);

    if (thumbColor) {
      themeState.setOverrideAccent(thumbColor);
    }

    const hasBanner = Boolean(effectiveBanner);
    const hasAvatar = Boolean(effectiveAvatar);

    if (!cachedAccent && (hasBanner || hasAvatar)) {
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
    } catch (error) {
      logger.error(`Failed to check creator favorite status for ${service}:${creatorId}`, error);
    }
  }

  function toggleFavorite() {
    void favoriteActions.toggleCreator({ id: creatorId, service, name: creatorName });
  }

  async function handleLoadMore() {
    if (normalizedPostSearch.length >= 2) {
      if (!postSearchLoading && postSearchHasMore) {
        await searchCreatorPosts();
      }
      return;
    }
    if (!entry.loadingMore && entry.hasMore) {
      await contentState.loadMoreCreatorPosts(service, creatorId, activeProviderId);
    }
  }

  let currentProviderName = $derived.by(() => {
    if (activeProviderId && activeProviderId !== 'auto') {
      const p = providerState.getProviderById(activeProviderId);
      if (p) return formatProviderName(p.name);
    }
    const defaultProv = providerState.getProviderForService(service);
    if (defaultProv) return formatProviderName(defaultProv.name || defaultProv.id);
    return 'Provider';
  });

  function openInProvider(targetProvId?: string) {
    const effectiveProvId = targetProvId || (activeProviderId && activeProviderId !== 'auto' ? activeProviderId : undefined);
    const pageUrl = typeof profile?.page_url === 'string' ? profile.page_url : undefined;
    if (!effectiveProvId && pageUrl) {
      void openExternalUrl(pageUrl);
      return;
    }
    void apiResolveCreatorUrl(service, creatorId, effectiveProvId)
      .then((url) => {
        if (url) void openExternalUrl(url);
      })
      .catch((err) => {
        logger.warn('Failed to resolve creator URL in provider', err);
        notify.error(i18n.t('common.open_link_failed'), err);
      });
  }

  function openOriginalProfile() {
    const url = getPlatformProfileUrl(service, creatorId, profile?.public_id);
    if (url) void openExternalUrl(url);
  }

  let isViewerOpen = $state(false);
  let viewerActiveIndex = $state(0);
  let viewerItems = $derived.by((): MediaViewerItem[] =>
    fancards.map((card, idx) => {
      const ext = (card.ext || card.mime?.split('/').pop() || 'jpg').replace(/^\.+/, '');
      return {
        id: String(card.id || card.hash || idx),
        name: `fancard_${card.id || idx + 1}.${ext}`,
        kind: 'image',
        url: fancardMediaUrl(card),
        poster: fancardThumbnailUrl(card),
        size: card.size
      };
    })
  );

  function openFancardViewer(index: number) {
    viewerActiveIndex = index;
    isViewerOpen = true;
  }

  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'posts');

  let pageMenu = $derived<MenuItem[]>([
    {
      group: 'page',
      label: i18n.t(isSelectionActive ? 'selection.exit' : 'selection.select_mode'),
      icon: IconCheckboxChecked,
      active: isSelectionActive,
      onclick: () => (isSelectionActive ? selectionState.exit() : selectionState.enter('posts'))
    },
    {
      group: 'page',
      label: i18n.t('common.refresh'),
      icon: IconArrowClockwise,
      disabled: entry.loading,
      onclick: () => void refreshCreator()
    },
    { group: 'open', label: i18n.t('creator.open_original_profile'), detail: formatServiceName(service), icon: IconOpen, onclick: openOriginalProfile },
    ...(activeProviderId === 'auto' && candidateProviders.length > 1
      ? candidateProviders.map((p) => ({ id: p.id as string | undefined, name: formatProviderName(p.name || p.id) }))
      : [{ id: activeProviderId && activeProviderId !== 'auto' ? activeProviderId : undefined, name: currentProviderName }]
    ).map((target) => ({
      group: 'open',
      label: i18n.t('creator.open_in_provider'),
      detail: target.name,
      icon: IconGlobe,
      onclick: () => openInProvider(target.id)
    })),
    { group: 'copy', label: i18n.t('post.copy_id'), detail: creatorId, detailMono: true, icon: IconNumberSymbol, onclick: () => copyText(creatorId) }
  ]);

  let selectedPosts = $derived(isSelectionActive ? selectionState.getItems<Post>() : []);
  const stashOptions = $derived(libraryState.stashOptions);

  const batchSelectedStashes = $derived(libraryState.stashesForPosts(selectedPosts));

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
</script>

{#snippet subscriptionControl()}
  <Button
    variant="ghost"
    onclick={() => subscriptionState.openDialog({ service, creatorId, creatorName })}
    class={layoutState.isMobile ? 'btn-icon' : ''}
    title={i18n.t(subscription ? 'subscriptions.subscription' : 'subscriptions.subscribe')}
    aria-label={i18n.t(subscription ? 'subscriptions.subscription' : 'subscriptions.subscribe')}
    aria-pressed={Boolean(subscription)}
  >
    {#if subscription}<IconAlertOn />{:else}<IconAlert />{/if}
    {#if !layoutState.isMobile}
      <span>{i18n.t(subscription ? 'subscriptions.subscription' : 'subscriptions.subscribe')}</span>
    {/if}
  </Button>
{/snippet}

{#snippet creatorBarActions()}
  <Button
    variant="ghost"
    class="btn-icon"
    disabled={favoriteActions.isCreatorPending({ id: creatorId, service })}
    onclick={toggleFavorite}
    title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
    aria-label={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
    aria-pressed={isFavorited}
  >
    {#if isFavorited}<IconHeartFilled />{:else}<IconHeart />{/if}
  </Button>

  {@render subscriptionControl()}
{/snippet}

{#snippet filterInnerContent()}
  {#if candidateProviders.length > 1}
    <div class="filter-section-title">
      <div class="filter-section-icon"><IconGlobe class="w-3.5 h-3.5" /></div>
      <span>{i18n.t('providers.title')}</span>
    </div>
    <div class="service-options">
      {#each candidateProviders as provider}
        {@const state = providerFilters[provider.id] ?? 'neutral'}
        {@const cleanName = formatProviderName(provider.name || provider.id)}
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
    <div class="filter-section-icon"><IconDocument class="w-3.5 h-3.5" /></div>
    <span>{i18n.t('feed.format')}</span>
  </div>
  <div class="service-options">
    {#each formatList as fmt}
      {@const state = fmt.id === 'ai' ? (hideAi ? 'on' : 'neutral') : (formatFilters[fmt.id] ?? 'neutral')}
      {@const IconComponent = fmt.icon}
      <Button
        variant="ghost"
        size="sm"
        onclick={() => toggleFormat(fmt.id)}
        class="filter-chip {state === 'include' || state === 'on' ? 'state-include' : state === 'exclude' ? 'state-exclude' : ''}"
      >
        <IconComponent class="w-5 h-5" />
        <span>{fmt.label()}</span>
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
{/snippet}

<PageShell back title={creatorName} titleTrigger={creatorTitleEl} menu={pageMenu} scrollKey={navigationState.entryKey} onrefresh={refreshCreator}
  selectionTotal={visibleCreatorPosts.length}
  onSelectAll={handleSelectAllPosts}
  {selectionActions}
>
  {#snippet actions()}
    {@render creatorBarActions()}
  {/snippet}

  {#if effectiveBanner}
    <HeroBackdrop src={effectiveBanner} />
  {/if}

  <div class="post-content-wrapper">
    <header class="detail-header">
      <div class="creator-title-row">
        <div class="creator-header-avatar">
          {#if effectiveAvatar}
            {#if avatarPlaceholder && !avatarLoaded}
              <img src={avatarPlaceholder} alt="" class="creator-avatar-placeholder" aria-hidden="true" />
            {/if}
            <img
              src={effectiveAvatar}
              alt={creatorName}
              class="creator-avatar-img"
              onload={() => avatarLoaded = true}
              onerror={handleAvatarError}
            />
          {:else}
            <div class="creator-avatar-initial">
              {initialLetter}
            </div>
          {/if}
        </div>

        <h1 bind:this={creatorTitleEl}>{creatorName}</h1>
      </div>

      {#if latestPostDateStr || showUpdated || showIndexed}
        <div class="post-date post-dates-row flex items-center flex-wrap gap-2 mt-2 text-sm text-[var(--text-secondary)]">
          {#if latestPostDateStr}
            <div class="flex items-center gap-1.5 shrink-0">
              <span class="text-[var(--text-muted)]">{i18n.t('creator.latest_post_at')}:</span>
              <strong class="font-semibold text-[var(--text-primary)]">{latestPostDateStr}</strong>
            </div>
          {/if}

          {#if showUpdated}
            {#if latestPostDateStr}<span class="text-[var(--text-muted)]">·</span>{/if}
            <div class="flex items-center gap-1.5 shrink-0">
              <span class="text-[var(--text-muted)]">{i18n.t('creator.updated_at')}:</span>
              <strong class="font-medium text-[var(--text-primary)]">{updatedDateStr}</strong>
            </div>
          {/if}

          {#if showIndexed}
            {#if latestPostDateStr || showUpdated}<span class="text-[var(--text-muted)]">·</span>{/if}
            <div class="flex items-center gap-1.5 shrink-0">
              <span class="text-[var(--text-muted)]">{i18n.t('creator.indexed_at')}:</span>
              <strong class="font-medium text-[var(--text-primary)]">{indexedDateStr}</strong>
            </div>
          {/if}
        </div>
      {/if}

      {#if candidateProviders.length > 1}
        <div class="post-date post-meta-row flex items-center flex-wrap gap-2 mt-2 -ml-[10px] min-h-[38px]">
          <ChipSelect
            options={providerSelectOptions}
            value={activeProviderId ?? 'auto'}
            onchange={handleProviderChange}
            icon={IconSource}
            label={i18n.t('post.source')}
          />
        </div>
      {/if}
    </header>

    <div class="media-controls-row mt-2">
      <div class="creator-tabs-scroll" use:edgeFade>
        <ChoiceGroup
          options={creatorTabOptions}
          value={activeTab}
          onchange={(val) => activeTab = val as typeof activeTab}
          align="left"
          class="creator-sections-choice"
          hasAddon={(option) => option.value === 'posts'}
        >
          {#snippet addon()}
            <SortControl options={sortOptions} value={sortOrder} onchange={(value) => (sortOrder = value as typeof sortOrder)} />
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
            <PopoverMenu
              bind:open={filtersOpen}
              title={i18n.t('feed.filters')}
              icon={IconOptions}
              badge={activeFilterCount}
              active={activeFilterCount > 0}
            >
              {@render filterInnerContent()}
            </PopoverMenu>
          </HeaderActions>
        {/if}
      </div>
    </div>

    {#if activeTab === 'posts'}
      <div class="creator-posts-section">
        {#if creatorTags.length > 0}
          <div class="creator-tags-bar mb-3">
            <TagList
              tags={creatorTags}
              activeTag={selectedTag}
              showAll={true}
              allLabel={i18n.t('common.all')}
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
          <EmptyState icon={IconSearch} title={i18n.t('creator.no_posts_with_tag', { tag: selectedTag })}>
            <Button variant="tonal" onclick={() => (selectedTag = null)}>{i18n.t('creator.clear_tag_filter')}</Button>
          </EmptyState>
        {:else if postSearchError && visibleCreatorPosts.length === 0}
          <EmptyState icon={IconWarning} tone="error" title={i18n.t('creator.search_error')} description={postSearchError} />
        {:else if (entry.error || (entry.loaded && entry.posts.length === 0)) && activeProviderId && activeProviderId !== 'auto' && !isPostsFiltered && !entry.loading}
          <EmptyState
            icon={IconSource}
            title={i18n.t('creator.not_found_on_provider', { provider: currentProviderName })}
            description={autoEntry && autoEntry.posts.length > 0
              ? i18n.t('creator.available_on_other_sources', { count: autoEntry.posts.length })
              : i18n.t('creator.not_found_on_provider_desc', { provider: currentProviderName })}
          >
            {#if candidateProviders.length > 1}
              <Button variant="tonal" onclick={() => handleProviderChange('auto')}>{i18n.t('post.switch_to_merged')}</Button>
            {/if}
          </EmptyState>
        {:else if entry.error && entry.posts.length === 0 && !entry.loading}
          <EmptyState icon={IconWarning} tone="error" title={i18n.t('creator.load_error')} description={entry.error}>
            <Button variant="tonal" onclick={() => contentState.refreshCreator(service, creatorId, activeProviderId)}>
              <IconArrowClockwise />
              <span>{i18n.t('common.retry')}</span>
            </Button>
          </EmptyState>
        {:else}
          <PostGrid
            posts={visibleCreatorPosts}
            loading={entry.loading || entry.loadingMore || (normalizedPostSearch.length >= 2 && postSearchLoading)}
            hasMore={normalizedPostSearch.length >= 2 ? postSearchHasMore : entry.hasMore}
            emptyTitle={postSearchQuery ? i18n.t('feed.no_results') : i18n.t('feed.empty')}
            ariaLabel={creatorName}
            showCreator={false}
            onLoadMore={handleLoadMore}
          />
        {/if}
      </div>
    {:else if activeTab === 'similar'}
      <div class="creator-section-view">
        {#if similarCreators.length === 0}
          <EmptyState icon={IconSparkle} title={i18n.t('creator.no_similar_artists')} description={i18n.t('creator.no_similar_artists_desc')} />
        {:else}
          <div class="creator-cards-grid" style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-gap: ${gap}px;`}>
            {#each similarCreators as sim}
              {@const sService = String(sim.service ?? service)}
              {@const sId = String(sim.id ?? '')}
              {@const sName = String(sim.name ?? sId)}
              {@const simAvatar = creatorAvatarSrc(sim)}
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
                  <h2 class="grid-tile-title">{sName}</h2>

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

                    <span class="grid-tile-author-name is-service">{sService}</span>
                  </div>

                  <div class="grid-tile-meta">
                    <span>{sId}</span>
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
          <EmptyState icon={IconGlobe} title={i18n.t('creator.no_links')} description={i18n.t('creator.no_links_desc')} />
        {:else}
          <div class="creator-cards-grid" style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-gap: ${gap}px;`}>
            {#each creatorLinks as link}
              {@const lService = String(link.service ?? '')}
              {@const lId = String(link.id ?? '')}
              {@const lName = String(link.name ?? lId)}
              {@const linkAvatar = creatorAvatarSrc(link)}
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
                  <h2 class="grid-tile-title">{lName}</h2>

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

                    <span class="grid-tile-author-name is-service">{lService}</span>
                  </div>

                  <div class="grid-tile-meta">
                    <span>{lId}</span>
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
          <EmptyState icon={IconNews} title={i18n.t('creator.no_announcements')} description={i18n.t('creator.no_announcements_desc')} />
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
          <EmptyState icon={IconImageOff} title={i18n.t('creator.no_fancards')} description={i18n.t('creator.no_fancards_desc')} />
        {:else}
          <div class="creator-cards-grid" style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-gap: ${gap}px;`}>
            {#each fancards as card, index}
              {@const cardThumb = fancardThumbnailUrl(card)}
              {@const cardFull = fancardMediaUrl(card)}
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

                    <span class="grid-tile-author-name is-service">{service}</span>
                  </div>

                  <div class="grid-tile-meta">
                    <span>{formatDate(card.added)}</span>
                    <div class="grid-tile-meta-stats">
                      <span>{ext}</span>
                      {#if card.size}<span>{formatBytes(card.size)}</span>{/if}
                    </div>
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

{#snippet selectionActions()}
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
    onclick={() => void favoriteActions.favoritePosts(selectionState.getItems<Post>()).then((done) => done && selectionState.exit())}
    title={i18n.t('selection.favorite')}
  >
    <IconHeart />
  </Button>
{/snippet}

<style>
  .post-content-wrapper {
    position: relative;
    z-index: 2;
    min-width: 0;
    max-width: 100%;
    overflow-x: clip;
  }

  .media-controls-row :global(.btn) {
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 18px !important;
    font-size: calc(var(--control-font-size, 14px) * var(--ui-scale, 1)) !important;
    border-radius: var(--radius-full) !important;
    gap: 8px !important;
  }

  .media-controls-row :global(.btn.btn-icon) {
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 !important;
    border-radius: 50% !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  .media-controls-row :global(.btn svg) {
    width: 20px;
    height: 20px;
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
    position: relative;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    display: grid;
    place-items: center;
  }

  .creator-avatar-placeholder {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    pointer-events: none;
  }

  .creator-avatar-img {
    position: relative;
    z-index: 1;
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
    color: var(--text-primary);
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
    color: var(--text-muted);
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

  .creator-section-view {
    padding: 16px 0;
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
    color: var(--text-muted);
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


</style>
