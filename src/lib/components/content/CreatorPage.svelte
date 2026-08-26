<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { contentState, creatorCacheKey, type CachedCreator } from '$lib/state/contentState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import { accountState } from '$lib/state/accountState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { providerState } from '$lib/state/providerState.svelte';
  import { themeState, getContrastColor } from '$lib/theme/themeState.svelte';
  import {
    apiFetchCreatorArtworkDataUrl,
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
  import { parseTags, formatDate, formatBytes, parseDateTimestamp, cleanPostTitle } from '$lib/utils/formatters';
  import type { DownloadScope, InitialImport } from '$lib/types/subscription';
  import type { PawchivePost, CreatorProfile, Announcement, Fancard } from '$lib/types/pawchive';
  import type { FilterMap } from '$lib/types/filter';
  import { countActiveFilters, matchesTriStateFilter, toggleFilterKey } from '$lib/types/filter';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import HeroBackdrop from '$lib/components/ui/HeroBackdrop.svelte';
  import SearchBar from '$lib/components/ui/SearchBar.svelte';
  import TagList from '$lib/components/ui/TagList.svelte';
  import CountBadge from '$lib/components/ui/CountBadge.svelte';
  import PostGrid from './PostGrid.svelte';
  import RichContent from './RichContent.svelte';
  import MediaViewer, { type MediaViewerItem } from './MediaViewer.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import { ripple } from '$lib/motion';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import SelectionActionBar from '$lib/components/ui/SelectionActionBar.svelte';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
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
  }

  let { service, creatorId }: Props = $props();

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
    postSearchResults?: PawchivePost[];
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

  let similarCreators = $state<CreatorProfile[]>([]);
  let creatorLinks = $state<CreatorProfile[]>([]);
  let announcements = $state<Announcement[]>([]);
  let fancards = $state<Fancard[]>([]);
  let apiCreatorTags = $state<string[]>([]);

  let postSearchOpen = $state(savedState?.postSearchOpen ?? Boolean(savedState?.postSearchQuery));
  let postSearchQuery = $state(savedState?.postSearchQuery ?? '');
  let postSearchResults = $state<PawchivePost[]>(savedState?.postSearchResults ?? []);
  let formatFilters = $state<FilterMap>(savedState?.formatFilters ?? {});
  let onlyWithAttachments = $state<boolean>(savedState?.onlyWithAttachments ?? false);
  let filtersOpen = $state(false);
  let stickyFiltersOpen = $state(false);
  let activeFilterCount = $derived(countActiveFilters([formatFilters]) + (onlyWithAttachments ? 1 : 0));

  let postSearchLoading = $state(false);
  let postSearchError = $state<string | null>(null);
  let postSearchOffset = $state(0);
  let postSearchHasMore = $state(false);
  let postSearchRequest = 0;
  const CREATOR_POST_PAGE_SIZE = 50;

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

  let creatorName = $derived(typeof entry.profile?.name === 'string' ? entry.profile.name : creatorId);
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

  let creatorTags = $derived.by(() => {
    const direct = parseTags(entry.profile?.tags || (entry.profile?.extra as any)?.tags || (entry.profile?.extra as any)?.categories);
    const combined = new Set<string>([...direct, ...apiCreatorTags]);

    const tagCounts = new Map<string, number>();
    for (const post of entry.posts) {
      const pTags = parseTags(post.tags);
      for (const t of pTags) {
        tagCounts.set(t, (tagCounts.get(t) || 0) + 1);
        combined.add(t);
      }
    }
    if (combined.size > 0) {
      return Array.from(combined)
        .sort((a, b) => (tagCounts.get(b) || 0) - (tagCounts.get(a) || 0))
        .slice(0, 32);
    }
    return [];
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
        const map = new Map<string, PawchivePost>();
        for (const p of localMatches) map.set(`${p.service}:${p.user}:${p.id}`, p);
        for (const p of postSearchResults) map.set(`${p.service}:${p.user}:${p.id}`, p);
        posts = Array.from(map.values());
      } else {
        posts = localMatches;
      }
    }

    if (selectedTag) {
      const normTag = selectedTag.trim().toLocaleLowerCase();
      posts = posts.filter((post) => {
        const pTags = parseTags(post.tags).map((t) => t.toLocaleLowerCase());
        return pTags.includes(normTag);
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

  const sortOptions = [
    { value: 'default', label: 'Default Order' },
    { value: 'newest', label: 'Newest First' },
    { value: 'oldest', label: 'Oldest First' },
    { value: 'popular', label: 'Most Favorited' }
  ];

  async function openPostSearch() {
    postSearchOpen = true;
    await tick();
    document.querySelector<HTMLInputElement>('.creator-post-search-input')?.focus();
  }

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

  onMount(() => {
    void contentState.loadCreator(service, creatorId);
    void apiFetchCreatorArtworkDataUrl(service, creatorId, 'avatar').then((url) => cachedAvatarUrl = url).catch(() => {});
    void apiFetchCreatorArtworkDataUrl(service, creatorId, 'banner').then((url) => cachedBannerUrl = url).catch(() => {});
    void loadExtraData();
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

  $effect(() => {
    if (service && creatorId) {
      void checkFavoriteStatus();
    }
  });

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
          console.warn('Creator artwork color extraction failed:', error);
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
        console.warn(`Failed to extract creator ${artworkKind} accent:`, error);
      }
    }
    return '';
  }

  $effect(() => {
    const hasBanner = Boolean(effectiveBanner);
    const hasAvatar = Boolean(effectiveAvatar);
    const dynamicAccent = configState.settings.dynamic_accent;
    let cancelled = false;

    if (dynamicAccent && (hasBanner || hasAvatar)) {
      void getCreatorAccentColor(hasBanner, hasAvatar).then((color) => {
        if (!color || cancelled) return;
        const root = document.documentElement;
        root.style.setProperty('--accent-primary', color);
        root.style.setProperty('--accent-primary-hover', color);
        root.style.setProperty('--accent-glow', color.replace('rgb', 'rgba').replace(')', ', 0.35)'));
        root.style.setProperty('--text-on-accent', getContrastColor(color));
      });
    }

    return () => {
      cancelled = true;
      themeState.applyCssTokens();
    };
  });

  async function checkFavoriteStatus() {
    try {
      const favorites = await accountState.fetchFavorites('creator');
      isFavorited = favorites.some((favorite) =>
        String(favorite.id).toLowerCase() === creatorId.toLowerCase() &&
        String(favorite.service ?? '').toLowerCase() === service.toLowerCase()
      );
    } catch (error) {
      console.error('Failed to check creator favorite status:', error);
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

  function openInBrowser() {
    const url = creatorPageUrl(service, creatorId);
    apiOpenInBrowser(url);
  }

  let copiedId = $state(false);
  async function copyCreatorId() {
    try {
      await navigator.clipboard.writeText(creatorId);
      copiedId = true;
      notify.success(i18n.t('common.copied') || 'Copied ID');
      setTimeout(() => { copiedId = false; }, 2000);
    } catch (e) {
      console.error(e);
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

  let selectedPosts = $derived(isSelectionActive ? selectionState.getItems<PawchivePost>() : []);
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
  <div class="sub-form flex flex-col gap-4 p-2">
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
        oninput={(e) => interval = Number((e.target as HTMLInputElement).value)}
      />
    </div>

    <div class="flex items-center justify-between gap-2 mt-2 pt-2 border-t border-[var(--border-color)]">
      {#if subscription}
        <Button variant="danger" size="sm" onclick={unsubscribe} disabled={saving}>
          {i18n.t('selection.unsubscribe')}
        </Button>
      {:else}
        <div></div>
      {/if}
      <Button variant="accent" size="sm" onclick={saveSubscription} disabled={saving}>
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
      title={i18n.t('subscriptions.settings')}
      width="320px"
    >
      {#snippet trigger()}
        <Button
          variant="accent"
          onclick={() => { initEditorFields(); subscriptionMenuOpen = !subscriptionMenuOpen; }}
          class="action-btn"
          title={i18n.t('subscriptions.settings')}
          aria-label={i18n.t('subscriptions.settings')}
        >
          <IconSettings class="w-[18px] h-[18px]" />
          <span class="btn-text">{i18n.t('subscriptions.settings')}</span>
        </Button>
      {/snippet}

      {@render subscriptionEditorFields()}
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
    <IconDocument class="view-option-icon w-[20px] h-[20px]" />
  </button>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey} onrefresh={refreshCreator}>
  {#snippet overlay()}
    <StickyHeader threshold={120}>
      <div class="sticky-post-info">
        <Button variant="ghost" onclick={() => navigationState.back()} class="sticky-back-btn" title={i18n.t('nav.back')}>
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
      </div>

      <div class="sticky-post-actions">
        <SearchBar
          bind:value={postSearchQuery}
          bind:open={postSearchOpen}
          placeholder={i18n.t('feed.search_placeholder')}
          onclose={closePostSearch}
        />

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
      </div>
    </StickyHeader>
  {/snippet}

  {#if effectiveBanner}
    <HeroBackdrop src={effectiveBanner} />
  {/if}

  <div class="post-content-wrapper">
    <!-- Top Action Bar -->
    <div class="post-actions-bar">
      <div class="left-actions flex items-center gap-2">
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

      <div class="right-actions flex items-center gap-2 ml-auto">
        <Button
          variant="ghost"
          onclick={openInBrowser}
          class="action-btn"
          title={i18n.t('post.open_in_browser')}
        >
          <IconOpen class="w-[18px] h-[18px]" />
          <span>{i18n.t('post.open_in_browser')}</span>
        </Button>
      </div>
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
          onclick={() => {
            const url = getPlatformProfileUrl(service, creatorId, entry.profile?.public_id as any);
            if (url) void apiOpenInBrowser(url);
          }}
          tooltip={i18n.t('creator.open_platform') || `Open on ${service}`}
          aria-label={`Open on ${service}`}
        >
          <ServiceIcon {service} class="w-4 h-4" />
          <span class="capitalize">{service}</span>
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

        {#if candidateProviders.length > 0}
          <span class="text-[var(--fg-subtle)]">·</span>
          <div class="inline-flex items-center shrink-0">
            <Select
              variant="ghost"
              disabled={candidateProviders.length === 1}
              options={providerSelectOptions}
              value={activeProviderId}
              onchange={(val) => providerState.setSelectedProvider(service, creatorId, '*', val)}
            />
          </div>
        {/if}
      </div>
    </header>

    <!-- Tags Row flush to the left edge before sections -->
    {#if creatorTags.length > 0}
      <div class="creator-tags-row mt-1 mb-5">
        <TagList
          tags={creatorTags}
          activeTag={selectedTag}
          size="md"
          maxVisible={14}
          onclick={(tag) => {
            if (selectedTag === tag) {
              selectedTag = null;
            } else {
              selectedTag = tag;
              activeTab = 'posts';
            }
          }}
        />
      </div>
    {/if}

    <!-- Sections Toolbar (Same as PostPage's media-controls-row) -->
    <div class="media-controls-row mt-2">
      <nav class="media-tabs" aria-label="Creator sections">
        <Button
          variant={activeTab === 'posts' ? 'accent' : 'ghost'}
          onclick={() => activeTab = 'posts'}
        >
          <IconGrid class="w-[16px] h-[16px]" />
          <span>{i18n.t('creator.posts')}</span>
          {#if entry.posts.length > 0}
            <CountBadge count={`${entry.posts.length}${entry.hasMore ? '+' : ''}`} />
          {/if}
        </Button>

        {#if similarCreators.length > 0}
          <Button
            variant={activeTab === 'similar' ? 'accent' : 'ghost'}
            onclick={() => activeTab = 'similar'}
          >
            <IconSparkle class="w-[16px] h-[16px]" />
            <span>{i18n.t('creator.similar_artists')}</span>
            <CountBadge count={similarCreators.length} />
          </Button>
        {/if}

        {#if creatorLinks.length > 0}
          <Button
            variant={activeTab === 'links' ? 'accent' : 'ghost'}
            onclick={() => activeTab = 'links'}
          >
            <IconLink class="w-[16px] h-[16px]" />
            <span>{i18n.t('creator.linked_accounts')}</span>
            <CountBadge count={creatorLinks.length} />
          </Button>
        {/if}

        {#if announcements.length > 0}
          <Button
            variant={activeTab === 'announcements' ? 'accent' : 'ghost'}
            onclick={() => activeTab = 'announcements'}
          >
            <IconNews class="w-[16px] h-[16px]" />
            <span>{i18n.t('creator.announcements')}</span>
            <CountBadge count={announcements.length} />
          </Button>
        {/if}

        {#if fancards.length > 0}
          <Button
            variant={activeTab === 'fancards' ? 'accent' : 'ghost'}
            onclick={() => activeTab = 'fancards'}
          >
            <IconCard class="w-[16px] h-[16px]" />
            <span>{i18n.t('creator.fancards')}</span>
            <CountBadge count={fancards.length} />
          </Button>
        {/if}
      </nav>

      <div class="media-controls-right flex items-center gap-2">
        {#if activeTab === 'posts'}
          <div class="media-sort-selector">
            <Select
              variant="ghost"
              options={sortOptions}
              value={sortOrder}
              onchange={(v) => sortOrder = v as typeof sortOrder}
            />
          </div>

          <SearchBar
            bind:value={postSearchQuery}
            bind:open={postSearchOpen}
            placeholder={i18n.t('feed.search_placeholder')}
            onclose={closePostSearch}
          />

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
        {/if}
      </div>
    </div>

    <!-- Active Tab Content -->
    {#if activeTab === 'posts'}
      <div class="creator-posts-section">
        {#if postSearchError && visibleCreatorPosts.length === 0}
          <div class="creator-error">{postSearchError}</div>
        {:else if entry.error && entry.posts.length === 0}
          <div class="creator-error">{entry.error}</div>
        {:else}
          <PostGrid
            posts={visibleCreatorPosts}
            loading={entry.loading && entry.posts.length === 0}
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
          <div class="creator-cards-grid">
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
          <div class="creator-cards-grid">
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
          <div class="creator-cards-grid">
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

<style>
  .post-content-wrapper {
    position: relative;
    z-index: 2;
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
  }

  .post-actions-bar :global(.btn),
  .media-controls-row :global(.btn),
  :global(.sticky-post-info .btn) {
    height: 44px !important;
    padding: 0 18px !important;
    font-size: 13.5px !important;
    border-radius: var(--radius-full) !important;
    gap: 8px !important;
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
  }

  .creator-title-row {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
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
  }

  .post-date {
    margin-top: 6px;
    color: rgba(255, 255, 255, 0.4);
    font-size: 12px;
  }

  .media-controls-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 16px;
    margin-bottom: 20px;
  }

  .media-tabs {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .media-tabs :global(.btn) {
    gap: 6px !important;
  }

  .media-sort-selector {
    width: 180px;
    flex-shrink: 0;
  }

  .media-sort-selector :global(.select-trigger) {
    height: 44px !important;
    font-size: 13.5px !important;
    padding: 0 18px !important;
    border-radius: var(--radius-full) !important;
  }

  .sticky-post-info {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    flex: 1;
  }

  .sticky-post-info :global(.sticky-back-btn) {
    flex: 0 0 44px !important;
    width: 44px !important;
    height: 44px !important;
    min-width: 44px !important;
    border-radius: 50% !important;
    padding: 0 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
    flex-shrink: 0 !important;
  }

  .sticky-post-info :global(.sticky-back-btn svg) {
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

  .sticky-post-info :global(.sticky-fav-btn) {
    width: 36px !important;
    height: 36px !important;
    min-width: 36px !important;
    padding: 0 !important;
    border-radius: 50% !important;
    flex-shrink: 0 !important;
  }

  .sticky-post-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-post-actions :global(.btn-text) {
    display: none;
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
    grid-template-columns: repeat(auto-fill, minmax(min(100%, calc(245px * var(--grid-scale, 1))), 1fr));
    gap: calc(10px * var(--grid-scale, 1));
    width: 100%;
  }

  .fallback-initials {
    font-family: var(--font-display, var(--font-sans));
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
</style>
