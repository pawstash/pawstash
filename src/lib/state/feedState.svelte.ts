import type { Post, Creator } from '$lib/types/content';
import type { FilterMap, TriStateFilter } from '$lib/types/filter';
import { matchesTriStateFilter } from '$lib/types/filter';
import { apiFetchPopularPosts, apiFetchRecentPosts, apiSearchHash, apiFetchPost } from '$lib/utils/ipc';
import { getPostFormats } from '$lib/utils/media';
import { parseTags } from '$lib/utils/formatters';
import { logger } from '$lib/utils/logger';
import { accountState } from './accountState.svelte';
import { configState } from './configState.svelte';
import { providerState } from './providerState.svelte';

const PAGE_SIZE = 50;
export type FeedMode = 'recent' | 'popular';
export type PopularPeriod = 'day' | 'week' | 'month';

interface FeedBucket {
  posts: Post[];
  offset: number;
  hasMore: boolean;
  loaded: boolean;
  loading: boolean;
  error: string | null;
  requestId: number;
}

const emptyBucket = (): FeedBucket => ({
  posts: [],
  offset: 0,
  hasMore: true,
  loaded: false,
  loading: false,
  error: null,
  requestId: 0
});

export class FeedState {
  creators = $state<Creator[]>([]);
  selectedCreator = $state<Creator | null>(null);

  private _searchQuery = $state<string>('');
  private searchTimer: ReturnType<typeof setTimeout> | undefined;
  searchBucket = $state<FeedBucket>(emptyBucket());

  mode = $state<FeedMode>('recent');
  popularPeriod = $state<PopularPeriod>('day');
  popularDate = $state<string>('');
  recent = $state<FeedBucket>(emptyBucket());
  popularBuckets = $state<Record<string, FeedBucket>>({});

  get searchQuery(): string {
    return this._searchQuery;
  }

  set searchQuery(val: string) {
    this._searchQuery = val;
    this.scheduleSearch();
  }

  isSearchActive = $derived(this.mode === 'recent' && this._searchQuery.trim().length >= 2);

  getPopularBucket(period: PopularPeriod, date: string) {
    const key = `${period}:${date || 'current'}`;
    this.popularBuckets[key] ??= emptyBucket();
    return this.popularBuckets[key];
  }

  providerFilters = $state<FilterMap>({});
  serviceFilters = $state<FilterMap>({});
  formatFilters = $state<FilterMap>({});
  aiFilter = $state<TriStateFilter>('neutral');
  onlyWithAttachments = $state(false);
  favoritesOnly = $state(false);

  current = $derived(
    this.isSearchActive
      ? this.searchBucket
      : this.mode === 'recent' 
        ? this.recent 
        : this.getPopularBucket(this.popularPeriod, this.popularDate)
  );

  posts = $derived(this.current.posts);
  hasMore = $derived(this.current.hasMore);
  isLoading = $derived(this.current.loading);
  error = $derived(this.current.error);

  filteredPosts = $derived(
    this.posts.filter((post) => {
      if (Object.keys(this.providerFilters).length > 0) {
        const postProvider = (post.extra as any)?.provider_id || providerState.getProviderIdForService(post.service);
        const matchesProvider = matchesTriStateFilter([postProvider], this.providerFilters);
        if (!matchesProvider) return false;
      }

      const matchesService = matchesTriStateFilter([post.service], this.serviceFilters);

      const hasAttachments = (post.attachment_count ?? post.attachments?.length ?? 0) > 0 || Boolean(post.file?.path);
      const matchesAttachments = !this.onlyWithAttachments || hasAttachments;

      const postFormats = getPostFormats(post);
      const matchesFormat = matchesTriStateFilter(postFormats, this.formatFilters);

      const postTags = parseTags(post.tags);
      const isPostAi = Boolean(
        postTags.some((t) => {
          const l = t.toLowerCase();
          return l === 'ai' || l.includes('ai generated') || l.includes('artificial intelligence');
        }) ||
        post.title?.toLowerCase().includes('[ai]') ||
        post.title?.toLowerCase().includes('(ai)')
      );

      if (configState.settings.pawchive_hide_ai || this.aiFilter === 'exclude') {
        if (isPostAi) return false;
      } else if (this.aiFilter === 'include') {
        if (this.mode === 'popular' && !isPostAi) return false;
      }

      const isFavCreator = accountState.favoriteCreators?.some(
        (c: any) => c.id.toLowerCase() === post.user.toLowerCase() && c.service.toLowerCase() === post.service.toLowerCase()
      ) ?? false;
      const matchesFavorites = !this.favoritesOnly || isFavCreator;

      let matchesQuery = true;
      const q = this._searchQuery.trim().toLowerCase();
      if (q) {
        if (this.mode === 'popular' || q.length < 2) {
          matchesQuery = (
            post.title?.toLowerCase().includes(q) ||
            post.content?.toLowerCase().includes(q) ||
            post.user?.toLowerCase().includes(q) ||
            false
          );
        }
      }

      return matchesService && matchesAttachments && matchesFormat && matchesFavorites && matchesQuery;
    })
  );

  setAiFilter(filter: TriStateFilter) {
    if (this.aiFilter === filter) return;
    this.aiFilter = filter;
    this.recent = emptyBucket();
    this.searchBucket = emptyBucket();
    if (this.isSearchActive) {
      void this.executeSearch(true);
    } else if (this.mode === 'recent') {
      void this.load(true);
    }
  }

  private scheduleSearch() {
    if (this.searchTimer) {
      clearTimeout(this.searchTimer);
      this.searchTimer = undefined;
    }
    if (this.mode !== 'recent') {
      return;
    }
    const query = this._searchQuery.trim();
    if (query.length < 2) {
      this.searchBucket = emptyBucket();
      return;
    }
    this.searchBucket.loading = true;
    this.searchTimer = setTimeout(() => {
      void this.executeSearch(true);
    }, 250);
  }

  async executeSearch(reset = false) {
    const query = this._searchQuery.trim();
    if (query.length < 2) {
      this.searchBucket = emptyBucket();
      return;
    }
    const bucket = this.searchBucket;
    if (!reset && bucket.loading) return;
    const offset = reset ? 0 : bucket.offset;
    const requestId = ++bucket.requestId;
    bucket.loading = true;
    bucket.error = null;

    try {
      // 1. Direct Hash Search Detection (SHA256/SHA1/MD5)
      const isCryptoHash = /^[a-fA-F0-9]{32,64}$/.test(query);
      if (isCryptoHash && reset) {
        try {
          const hashResult = await apiSearchHash(query);
          const allTargets = [
            ...(hashResult?.posts || []),
            ...(hashResult?.discord_posts || [])
          ];
          if (allTargets.length > 0) {
            const matchedPosts: Post[] = [];
            for (const target of allTargets) {
              const svc = target.service;
              const usr = target.user;
              const pid = target.id;
              if (!svc || !usr || !pid) continue;
              try {
                const post = await apiFetchPost(svc, usr, pid);
                if (post) {
                  post.extra = typeof post.extra === 'object' && post.extra !== null
                    ? { ...post.extra, hash_matched: true, matched_hash: query }
                    : { hash_matched: true, matched_hash: query };
                  matchedPosts.push(post);
                }
              } catch {}
            }
            if (matchedPosts.length > 0) {
              if (requestId !== bucket.requestId || query !== this._searchQuery.trim()) return;
              bucket.posts = matchedPosts;
              bucket.offset = 0;
              bucket.hasMore = false;
              bucket.loaded = true;
              return;
            }
          }
        } catch {}
      }

      // 2. Keyword Search with optional AI filter directive
      let effectiveQuery = query;
      if (this.aiFilter === 'exclude') {
        effectiveQuery = `${query} hide=ai`;
      } else if (this.aiFilter === 'include') {
        effectiveQuery = `${query} only=ai`;
      }

      const posts = await apiFetchRecentPosts(effectiveQuery, offset);
      if (requestId !== bucket.requestId || query !== this._searchQuery.trim()) return;
      const nextPosts = reset ? posts : [...bucket.posts, ...posts];
      bucket.posts = [...new Map(nextPosts.map((post) => [`${post.service}:${post.user}:${post.id}`, post])).values()];
      bucket.offset = offset + PAGE_SIZE;
      bucket.hasMore = posts.length === PAGE_SIZE;
      bucket.loaded = true;
    } catch (error) {
      if (requestId === bucket.requestId) {
        bucket.error = error instanceof Error ? error.message : String(error);
      }
    } finally {
      if (requestId === bucket.requestId) bucket.loading = false;
    }
  }

  async setMode(mode: FeedMode) {
    this.mode = mode;
    if (this._searchQuery.trim().length >= 2) {
      if (mode === 'recent') {
        this.scheduleSearch();
      }
    }
    if (!this.current.loaded) await this.refresh();
  }

  async setPopularPeriod(period: PopularPeriod) {
    this.popularPeriod = period;
    if (!this.current.loaded) await this.refresh();
  }

  async load(reset = false) {
    if (this.isSearchActive) {
      return this.executeSearch(reset);
    }
    const mode = this.mode;
    const period = this.popularPeriod;
    const date = this.popularDate;
    const bucket = this.current;
    if (bucket.loading) return;
    const offset = reset ? 0 : bucket.offset;
    const requestId = ++bucket.requestId;
    bucket.loading = true;
    bucket.error = null;
    try {
      let queryParam: string | undefined = undefined;
      if (this.aiFilter === 'exclude') {
        queryParam = 'hide=ai';
      } else if (this.aiFilter === 'include') {
        queryParam = 'only=ai';
      }

      const posts = mode === 'recent'
        ? await apiFetchRecentPosts(queryParam, offset)
        : await apiFetchPopularPosts(period, date || undefined, offset);
      if (requestId !== bucket.requestId) return;
      const nextPosts = reset ? posts : [...bucket.posts, ...posts];
      bucket.posts = [...new Map(nextPosts.map((post) => [`${post.service}:${post.user}:${post.id}`, post])).values()];
      bucket.offset = offset + PAGE_SIZE;
      bucket.hasMore = posts.length === PAGE_SIZE;
      bucket.loaded = true;
      logger.info(`[Feed] Fetched ${posts.length} posts (mode: ${mode}, offset: ${offset})`);
    } catch (error) {
      if (requestId === bucket.requestId) {
        bucket.error = error instanceof Error ? error.message : String(error);
        logger.error(`[Feed] Failed to load feed (mode: ${mode}, offset: ${offset})`, error);
      }
    } finally {
      if (requestId === bucket.requestId) bucket.loading = false;
    }
  }

  clearAll() {
    this.recent = emptyBucket();
    this.popularBuckets = {};
    this.searchBucket = emptyBucket();
  }

  refresh() {
    this.clearAll();
    if (this.isSearchActive) {
      return this.executeSearch(true);
    }
    return this.load(true);
  }

  loadMore() {
    if (this.isSearchActive) {
      return this.executeSearch(false);
    }
    return this.load(false);
  }
}

export const feedState = new FeedState();
