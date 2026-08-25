import type { CreatorProfile, PawchivePost } from '$lib/types/pawchive';
import { apiFetchCreatorPosts, apiFetchCreatorProfile, apiFetchPost, apiGetCachedPost } from '$lib/utils/ipc';
import { logger } from '$lib/utils/logger';
import { creatorsState } from '$lib/state/creatorsState.svelte';

const PAGE_SIZE = 50;

export interface CachedPost {
  post: PawchivePost | null;
  loading: boolean;
  loaded: boolean;
  error: string | null;
}

export interface CachedCreator {
  profile: CreatorProfile | null;
  posts: PawchivePost[];
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

export function postCacheKey(service: string, creatorId: string | number, postId: string | number) {
  return `${String(service || '').toLowerCase()}:${String(creatorId || '').toLowerCase()}:${String(postId || '')}`;
}

export function creatorCacheKey(service: string, creatorId: string | number) {
  return `${String(service || '').toLowerCase()}:${String(creatorId || '').toLowerCase()}`;
}

export class ContentState {
  posts = $state<Record<string, CachedPost>>({});
  creators = $state<Record<string, CachedCreator>>({});

  seedPost(post: PawchivePost) {
    const key = postCacheKey(post.service, post.user, post.id);
    const existing = this.posts[key];
    if (!existing) {
      this.posts[key] = { post, loading: false, loaded: post.detail_fetched === true, error: null };
    } else if (!existing.loaded) {
      this.posts[key] = {
        ...existing,
        post: { ...(existing.post || {}), ...post }
      };
    }
  }

  getPost(service: string, creatorId: string | number, postId: string | number) {
    const key = postCacheKey(service, creatorId, postId);
    this.posts[key] ??= { post: null, loading: false, loaded: false, error: null };
    return this.posts[key];
  }

  async loadPost(service: string, creatorId: string | number, postId: string | number, force = false) {
    const key = postCacheKey(service, creatorId, postId);
    const entry = this.getPost(service, creatorId, postId);
    if (!force && entry.loaded && entry.post?.detail_fetched) return;
    if (entry.loading) return;

    if (!entry.post) {
      try {
        const cached = await apiGetCachedPost(String(service), String(creatorId), String(postId));
        if (cached) {
          this.posts[key] = {
            post: cached,
            loading: false,
            loaded: cached.detail_fetched === true,
            error: null
          };
          logger.debug(`[Content] Hydrated post ${service}:${creatorId}:${postId} from local cache`);
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
      const detail = await apiFetchPost(String(service), String(creatorId), String(postId));
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
      if (!currentEntry.post) {
        this.posts[key] = {
          post: null,
          loading: false,
          loaded: false,
          error: errorMessage(error)
        };
        logger.error(`Failed to load post ${service}:${creatorId}:${postId}`, error);
      } else {
        this.posts[key] = {
          ...currentEntry,
          loading: false
        };
      }
    }
  }

  getCreator(service: string, creatorId: string) {
    const key = creatorCacheKey(service, creatorId);
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

  async loadCreator(service: string, creatorId: string) {
    const entry = this.getCreator(service, creatorId);
    if (entry.loaded || entry.loading) return;
    entry.loading = true;
    entry.error = null;
    try {
      const [profileResult, postsResult] = await Promise.allSettled([
        apiFetchCreatorProfile(service, creatorId),
        apiFetchCreatorPosts(service, creatorId, undefined, 0)
      ]);

      if (profileResult.status === 'fulfilled') {
        entry.profile = profileResult.value;
      } else {
        logger.warn(`Could not fetch dedicated profile for ${service}:${creatorId}, using fallback metadata`, profileResult.reason);
        const cachedName = creatorsState.creatorsMap.get(`${service.toLowerCase()}:${creatorId.toLowerCase()}`);
        entry.profile = {
          id: creatorId,
          name: cachedName || creatorId,
          service,
          public_id: undefined,
          relation_id: undefined,
          indexed: undefined,
          updated: undefined,
          kemono_favorited: 0,
          ever_imported: false,
          extra: {}
        };
      }

      if (postsResult.status === 'fulfilled') {
        entry.posts = postsResult.value;
        entry.offset = PAGE_SIZE;
        entry.hasMore = postsResult.value.length === PAGE_SIZE;
        entry.loaded = true;
        logger.info(`[Content] Loaded ${postsResult.value.length} posts for ${service}:${creatorId}`);

        if (entry.profile && entry.profile.name === creatorId && postsResult.value.length > 0) {
          const firstPostUser = postsResult.value[0].user;
          if (firstPostUser && firstPostUser !== creatorId) {
            entry.profile.name = firstPostUser;
          }
        }
      } else {
        logger.error(`Failed to load posts for creator ${service}:${creatorId}`, postsResult.reason);
        if (entry.posts.length === 0) {
          throw postsResult.reason;
        }
      }
    } catch (error) {
      entry.error = errorMessage(error);
      logger.error(`Error loading creator ${service}:${creatorId}`, error);
    } finally {
      entry.loading = false;
    }
  }

  async refreshCreator(service: string, creatorId: string) {
    const entry = this.getCreator(service, creatorId);
    if (entry.loading) return;
    entry.loading = true;
    entry.error = null;
    try {
      const [profileResult, postsResult] = await Promise.allSettled([
        apiFetchCreatorProfile(service, creatorId),
        apiFetchCreatorPosts(service, creatorId, undefined, 0)
      ]);

      if (profileResult.status === 'fulfilled') {
        entry.profile = profileResult.value;
      } else {
        logger.warn(`Could not refresh dedicated profile for ${service}:${creatorId}, preserving metadata`, profileResult.reason);
        if (!entry.profile) {
          const cachedName = creatorsState.creatorsMap.get(`${service.toLowerCase()}:${creatorId.toLowerCase()}`);
          entry.profile = {
            id: creatorId,
            name: cachedName || creatorId,
            service,
            public_id: undefined,
            relation_id: undefined,
            indexed: undefined,
            updated: undefined,
            kemono_favorited: 0,
            ever_imported: false,
            extra: {}
          };
        }
      }

      if (postsResult.status === 'fulfilled') {
        entry.posts = postsResult.value;
        entry.offset = PAGE_SIZE;
        entry.hasMore = postsResult.value.length === PAGE_SIZE;
        entry.loaded = true;
      } else {
        logger.error(`Failed to refresh posts for creator ${service}:${creatorId}`, postsResult.reason);
        if (entry.posts.length === 0) {
          throw postsResult.reason;
        }
      }
    } catch (error) {
      entry.error = errorMessage(error);
      logger.error(`Error refreshing creator ${service}:${creatorId}`, error);
    } finally {
      entry.loading = false;
    }
  }

  async loadMoreCreatorPosts(service: string, creatorId: string) {
    const entry = this.getCreator(service, creatorId);
    if (entry.loadingMore || !entry.hasMore) return;
    entry.loadingMore = true;
    entry.error = null;
    try {
      const posts = await apiFetchCreatorPosts(service, creatorId, undefined, entry.offset);
      entry.posts = [...entry.posts, ...posts];
      entry.offset += PAGE_SIZE;
      entry.hasMore = posts.length === PAGE_SIZE;
    } catch (error) {
      entry.error = errorMessage(error);
      logger.error(`Failed to load more posts for creator ${service}:${creatorId} at offset ${entry.offset}`, error);
    } finally {
      entry.loadingMore = false;
    }
  }
}

export const contentState = new ContentState();
