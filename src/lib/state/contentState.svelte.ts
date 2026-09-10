import type { CreatorProfile, Post } from '$lib/types/content';
import { apiFetchCreatorPosts, apiFetchCreatorProfile, apiFetchPost, apiGetCachedPost } from '$lib/utils/ipc';
import { logger } from '$lib/utils/logger';
import { creatorsState } from '$lib/state/creatorsState.svelte';

const PAGE_SIZE = 50;

export interface CachedPost {
  post: Post | null;
  accentColor?: string;
  loading: boolean;
  loaded: boolean;
  error: string | null;
}

export interface CachedCreator {
  profile: CreatorProfile | null;
  accentColor?: string;
  posts: Post[];
  loading: boolean;
  loadingMore: boolean;
  loaded: boolean;
  error: string | null;
  offset: number;
  hasMore: boolean;
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function normalizePostId(postId: unknown): string {
  if (postId === null || postId === undefined) return '';
  if (typeof postId === 'string' || typeof postId === 'number') {
    const s = String(postId).trim();
    return s === '[object Object]' ? '' : s;
  }
  if (typeof postId === 'object') {
    const obj = postId as Record<string, unknown>;
    const candidate = obj.id ?? obj.post_id ?? obj.postId;
    if (candidate !== null && candidate !== undefined) {
      return normalizePostId(candidate);
    }
  }
  return '';
}

export function postCacheKey(service: string, creatorId: string | number, postId: unknown, providerId?: string) {
  const normId = normalizePostId(postId);
  const prov = (providerId && providerId !== 'auto') ? `:${providerId.toLowerCase()}` : '';
  return `${String(service || '').toLowerCase()}:${String(creatorId || '').toLowerCase()}:${normId}${prov}`;
}

export function creatorCacheKey(service: string, creatorId: string | number, providerId?: string) {
  const prov = (providerId && providerId !== 'auto') ? providerId.toLowerCase() : 'auto';
  return `${String(service || '').toLowerCase()}:${String(creatorId || '').toLowerCase()}:${prov}`;
}

export class ContentState {
  posts = $state<Record<string, CachedPost>>({});
  creators = $state<Record<string, CachedCreator>>({});

  getPostAccent(service: string, creatorId: string | number, postId: string | number, providerId?: string): string | undefined {
    const key = postCacheKey(service, creatorId, postId, providerId);
    return this.posts[key]?.accentColor;
  }

  setPostAccent(service: string, creatorId: string | number, postId: string | number, color: string, providerId?: string) {
    if (!color) return;
    const key = postCacheKey(service, creatorId, postId, providerId);
    const entry = this.getPost(service, creatorId, postId, providerId);
    this.posts[key] = {
      ...entry,
      accentColor: color
    };
  }

  getCreatorAccent(service: string, creatorId: string | number, providerId?: string): string | undefined {
    const key = creatorCacheKey(service, creatorId, providerId);
    return this.creators[key]?.accentColor;
  }

  setCreatorAccent(service: string, creatorId: string | number, color: string, providerId?: string) {
    if (!color) return;
    const key = creatorCacheKey(service, creatorId, providerId);
    const entry = this.getCreator(service, String(creatorId), providerId);
    this.creators[key] = {
      ...entry,
      accentColor: color
    };
  }

  seedPost(post: Post) {
    this.setPost(post);
  }

  setPost(post: Post) {
    if (!post?.id || !post?.service || !post?.user) return;
    const key = postCacheKey(post.service, post.user, post.id);
    const existing = this.posts[key];
    this.posts[key] = {
      ...(existing || {}),
      post,
      loaded: post.detail_fetched === true,
      loading: false,
      error: null
    };
    const provId = (post.extra as any)?.provider_id;
    if (typeof provId === 'string' && provId.trim()) {
      const provKey = postCacheKey(post.service, post.user, post.id, provId);
      const existingProv = this.posts[provKey];
      this.posts[provKey] = {
        ...(existingProv || {}),
        post,
        loaded: post.detail_fetched === true,
        loading: false,
        error: null
      };
    }
  }

  setPosts(posts: Post[]) {
    for (const post of posts) {
      this.setPost(post);
    }
  }

  getPost(service: string, creatorId: string | number, postId: unknown, providerId?: string) {
    const key = postCacheKey(service, creatorId, postId, providerId);
    this.posts[key] ??= { post: null, loading: false, loaded: false, error: null };
    return this.posts[key];
  }

  async loadPost(service: string, creatorId: string | number, rawPostId: unknown, force = false, providerId?: string) {
    const postId = normalizePostId(rawPostId);
    if (!postId || !service || !creatorId) return;

    const key = postCacheKey(service, creatorId, postId, providerId);
    const entry = this.getPost(service, creatorId, postId, providerId);
    if (!force && ((entry.loaded && entry.post?.detail_fetched) || entry.loading || entry.error)) return;
    if (entry.loading) return;

    if (!entry.post || !entry.post.detail_fetched) {
      try {
        const cached = await apiGetCachedPost(String(service), String(creatorId), String(postId), providerId);
        if (cached && cached.detail_fetched) {
          this.posts[key] = {
            post: {
              ...(entry.post || {}),
              ...cached,
              detail_fetched: true
            },
            loading: false,
            loaded: true,
            error: null
          };
          logger.debug(`[Content] Hydrated post ${service}:${creatorId}:${postId}${providerId ? `:${providerId}` : ''} from local cache`);
          return;
        } else if (cached && !entry.post) {
          this.posts[key] = {
            post: cached,
            loading: false,
            loaded: cached.detail_fetched === true,
            error: null
          };
        }
      } catch {
        // ignore fast-path probe failure
      }
    }

    const currentEntry = this.posts[key] ?? entry;
    if (!force && currentEntry.loaded && currentEntry.post?.detail_fetched) return;

    this.posts[key] = {
      ...currentEntry,
      loading: true,
      error: null
    };

    try {
      const detail = await apiFetchPost(String(service), String(creatorId), String(postId), providerId);
      this.posts[key] = {
        post: {
          ...(currentEntry.post || {}),
          ...detail,
          favorite_count: currentEntry.post?.favorite_count ?? detail.favorite_count,
          attachment_count: detail.attachments?.length ?? currentEntry.post?.attachment_count,
          detail_fetched: true
        },
        loading: false,
        loaded: true,
        error: null
      };
    } catch (error) {
      const msg = errorMessage(error);
      const fallbackPost = currentEntry.post || this.getPost(service, creatorId, postId).post;
      if (fallbackPost) {
        this.posts[key] = {
          ...currentEntry,
          post: fallbackPost,
          loading: false,
          error: msg
        };
      } else {
        this.posts[key] = {
          post: null,
          loading: false,
          loaded: true,
          error: msg
        };
      }
      logger.error(`Failed to load post ${service}:${creatorId}:${postId}${providerId ? `:${providerId}` : ''}`, error);
    }
  }

  private prefetchQueue: Array<{ service: string; creatorId: string; postId: string }> = [];
  private prefetchRunning = 0;
  private maxPrefetchConcurrency = 2;
  private queuedKeys = new Set<string>();

  enqueueDetailPrefetch(service: string, creatorId: string | number, rawPostId: unknown) {
    const postId = normalizePostId(rawPostId);
    if (!postId || !service || !creatorId) return;
    const key = postCacheKey(service, creatorId, postId);

    const entry = this.posts[key];
    if (entry?.loaded && entry.post?.detail_fetched) return;
    if (entry?.loading) return;
    if (this.queuedKeys.has(key)) return;

    this.queuedKeys.add(key);
    this.prefetchQueue.push({ service: String(service), creatorId: String(creatorId), postId });
    this.processPrefetchQueue();
  }

  cancelPrefetch(service: string, creatorId: string | number, rawPostId: unknown) {
    const postId = normalizePostId(rawPostId);
    if (!postId) return;
    const key = postCacheKey(service, creatorId, postId);
    this.queuedKeys.delete(key);
    this.prefetchQueue = this.prefetchQueue.filter(
      (item) => !(item.service === service && item.creatorId === String(creatorId) && item.postId === postId)
    );
  }

  private async processPrefetchQueue() {
    while (this.prefetchRunning < this.maxPrefetchConcurrency && this.prefetchQueue.length > 0) {
      const item = this.prefetchQueue.shift();
      if (!item) break;
      const key = postCacheKey(item.service, item.creatorId, item.postId);
      this.queuedKeys.delete(key);

      const entry = this.posts[key];
      if (entry?.loaded && entry.post?.detail_fetched) continue;

      this.prefetchRunning++;
      void (async () => {
        try {
          await this.loadPost(item.service, item.creatorId, item.postId);
        } catch {} finally {
          this.prefetchRunning--;
          setTimeout(() => this.processPrefetchQueue(), 50);
        }
      })();
    }
  }

  getCreator(service: string, creatorId: string, providerId?: string) {
    const key = creatorCacheKey(service, creatorId, providerId);
    this.creators[key] ??= {
      profile: null,
      posts: [],
      loading: false,
      loadingMore: false,
      loaded: false,
      error: null,
      offset: 0,
      hasMore: true
    };
    return this.creators[key];
  }

  async loadCreator(service: string, creatorId: string, autoFetchPosts = false, providerId?: string) {
    const key = creatorCacheKey(service, creatorId, providerId);
    const entry = this.getCreator(service, creatorId, providerId);
    if (entry.loaded || entry.loading) {
      if (entry.loaded && autoFetchPosts && entry.hasMore && !entry.loadingMore) {
        void this.autoFetchAllCreatorPosts(service, creatorId, providerId);
      }
      return;
    }

    const cachedName = creatorsState.creatorsMap.get(`${service.toLowerCase()}:${creatorId.toLowerCase()}`);
    const placeholderProfile: CreatorProfile = entry.profile && entry.profile.name !== creatorId ? entry.profile : {
      id: creatorId,
      name: cachedName || entry.profile?.name || creatorId,
      service,
      public_id: undefined,
      relation_id: undefined,
      indexed: undefined,
      updated: undefined,
      favorited: 0,
      ever_imported: false,
      extra: {}
    };

    this.creators[key] = {
      ...entry,
      profile: placeholderProfile,
      loading: true,
      error: null
    };

    try {
      const [profileResult, postsResult] = await Promise.allSettled([
        apiFetchCreatorProfile(service, creatorId, providerId),
        apiFetchCreatorPosts(service, creatorId, undefined, 0, providerId)
      ]);

      let finalProfile: CreatorProfile = placeholderProfile;
      if (profileResult.status === 'fulfilled' && profileResult.value) {
        finalProfile = profileResult.value;
      } else {
        const foundName = creatorsState.creatorsMap.get(`${service.toLowerCase()}:${creatorId.toLowerCase()}`);
        if (foundName && foundName !== creatorId) {
          finalProfile = { ...finalProfile, name: foundName };
        } else {
          void creatorsState.resolveName(service, creatorId).then((resolved) => {
            if (resolved) {
              const live = this.creators[key];
              if (live?.profile && (live.profile.name === creatorId || !live.profile.name)) {
                this.creators[key] = {
                  ...live,
                  profile: { ...live.profile, name: resolved }
                };
              }
            }
          });
        }
      }

      if (postsResult.status === 'rejected') {
        throw postsResult.reason;
      }

      const posts = postsResult.value;
      const cur = this.creators[key] ?? entry;
      this.creators[key] = {
        ...cur,
        profile: finalProfile,
        posts,
        offset: PAGE_SIZE,
        hasMore: posts.length >= PAGE_SIZE,
        loaded: true,
        loading: false,
        error: null
      };
      logger.info(`[Content] Loaded ${posts.length} posts for ${service}:${creatorId} (provider: ${providerId || 'auto'})`);
      if (autoFetchPosts && this.creators[key].hasMore) {
        void this.autoFetchAllCreatorPosts(service, creatorId, providerId);
      }
    } catch (error) {
      const cur = this.creators[key] ?? entry;
      this.creators[key] = {
        ...cur,
        loading: false,
        error: errorMessage(error)
      };
      logger.error(`Error loading creator ${service}:${creatorId}`, error);
    }
  }

  async refreshCreator(service: string, creatorId: string, providerId?: string) {
    const key = creatorCacheKey(service, creatorId, providerId);
    const entry = this.getCreator(service, creatorId, providerId);
    if (entry.loading) return;

    this.stopAutoFetchCreatorPosts(service, creatorId, providerId);

    this.creators[key] = {
      ...entry,
      loading: true,
      error: null
    };

    try {
      const [profileResult, postsResult] = await Promise.allSettled([
        apiFetchCreatorProfile(service, creatorId, providerId),
        apiFetchCreatorPosts(service, creatorId, undefined, 0, providerId)
      ]);

      let finalProfile: CreatorProfile = entry.profile || {
        id: creatorId,
        name: creatorId,
        service,
        public_id: undefined,
        relation_id: undefined,
        indexed: undefined,
        updated: undefined,
        favorited: 0,
        ever_imported: false,
        extra: {}
      };

      if (profileResult.status === 'fulfilled' && profileResult.value) {
        finalProfile = profileResult.value;
      }

      if (finalProfile.name === creatorId || !finalProfile.name) {
        const foundName = creatorsState.creatorsMap.get(`${service.toLowerCase()}:${creatorId.toLowerCase()}`);
        if (foundName && foundName !== creatorId) {
          finalProfile = { ...finalProfile, name: foundName };
        }
      }

      if (postsResult.status === 'rejected') {
        throw postsResult.reason;
      }

      const posts = postsResult.value;
      const cur = this.creators[key] ?? entry;
      this.creators[key] = {
        ...cur,
        profile: finalProfile,
        posts,
        offset: PAGE_SIZE,
        hasMore: posts.length >= PAGE_SIZE,
        loaded: true,
        loading: false,
        error: null
      };
      logger.info(`[Content] Refreshed ${posts.length} posts for ${service}:${creatorId} (provider: ${providerId || 'auto'})`);
      if (this.creators[key].hasMore) {
        void this.autoFetchAllCreatorPosts(service, creatorId, providerId);
      }
    } catch (error) {
      const cur = this.creators[key] ?? entry;
      this.creators[key] = {
        ...cur,
        loading: false,
        error: errorMessage(error)
      };
      logger.error(`Error refreshing creator ${service}:${creatorId}`, error);
    }
  }

  private creatorFetchTokens = new Map<string, number>();

  stopAutoFetchCreatorPosts(service: string, creatorId: string, providerId?: string) {
    const key = creatorCacheKey(service, creatorId, providerId);
    const token = (this.creatorFetchTokens.get(key) || 0) + 1;
    this.creatorFetchTokens.set(key, token);
    const cur = this.creators[key];
    if (cur && cur.loadingMore) {
      this.creators[key] = { ...cur, loadingMore: false };
    }
  }

  async autoFetchAllCreatorPosts(service: string, creatorId: string, providerId?: string) {
    const key = creatorCacheKey(service, creatorId, providerId);
    const token = (this.creatorFetchTokens.get(key) || 0) + 1;
    this.creatorFetchTokens.set(key, token);

    let consecutiveErrors = 0;

    while (true) {
      if (this.creatorFetchTokens.get(key) !== token) {
        break;
      }

      const entry = this.getCreator(service, creatorId, providerId);
      if (!entry.hasMore || entry.loading || entry.error) {
        break;
      }

      await new Promise((resolve) => setTimeout(resolve, 250));

      if (this.creatorFetchTokens.get(key) !== token) {
        break;
      }

      const curEntry = this.getCreator(service, creatorId, providerId);
      if (!curEntry.hasMore) break;
      this.creators[key] = { ...curEntry, loadingMore: true };

      try {
        const offset = curEntry.offset;
        const posts = await apiFetchCreatorPosts(service, creatorId, undefined, offset, providerId);

        if (this.creatorFetchTokens.get(key) !== token) {
          break;
        }

        consecutiveErrors = 0;
        const cur = this.creators[key] ?? curEntry;
        const hasMore = posts.length >= PAGE_SIZE;

        const existingIds = new Set(cur.posts.map((p) => p.id));
        const newPosts = posts.filter((p) => !existingIds.has(p.id));

        this.creators[key] = {
          ...cur,
          posts: [...cur.posts, ...newPosts],
          offset: cur.offset + PAGE_SIZE,
          hasMore,
          loadingMore: hasMore
        };

        if (!hasMore || posts.length === 0) {
          this.creators[key] = { ...this.creators[key], loadingMore: false };
          break;
        }
      } catch (err) {
        consecutiveErrors++;
        logger.warn(`[Content] Auto-fetch error (${consecutiveErrors}/3) for ${service}:${creatorId}:`, err);

        if (consecutiveErrors >= 3 || this.creatorFetchTokens.get(key) !== token) {
          const cur = this.creators[key] ?? curEntry;
          this.creators[key] = {
            ...cur,
            loadingMore: false
          };
          break;
        }

        await new Promise((resolve) => setTimeout(resolve, 1500 * consecutiveErrors));
      }
    }
  }

  async loadMoreCreatorPosts(service: string, creatorId: string, providerId?: string) {
    const key = creatorCacheKey(service, creatorId, providerId);
    const entry = this.getCreator(service, creatorId, providerId);
    if (entry.loadingMore || !entry.hasMore) return;

    this.creators[key] = {
      ...entry,
      loadingMore: true,
      error: null
    };

    try {
      const posts = await apiFetchCreatorPosts(service, creatorId, undefined, entry.offset, providerId);
      const cur = this.creators[key] ?? entry;
      this.creators[key] = {
        ...cur,
        posts: [...cur.posts, ...posts],
        offset: cur.offset + PAGE_SIZE,
        hasMore: posts.length >= PAGE_SIZE,
        loadingMore: false
      };
    } catch (error) {
      const cur = this.creators[key] ?? entry;
      this.creators[key] = {
        ...cur,
        loadingMore: false,
        error: errorMessage(error)
      };
      logger.error(`Failed to load more posts for creator ${service}:${creatorId} at offset ${entry.offset}`, error);
    }
  }

  clearCreatorsCache() {
    this.creatorFetchTokens.clear();
    this.creators = {};
    logger.info('[Content] Cleared creators cache');
  }

  clearAllCache() {
    this.posts = {};
    this.creators = {};
    logger.info('[Content] Cleared all content cache');
  }
}

export const contentState = new ContentState();

