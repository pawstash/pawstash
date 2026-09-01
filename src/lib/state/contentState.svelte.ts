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

export function postCacheKey(service: string, creatorId: string | number, postId: unknown) {
  const normId = normalizePostId(postId);
  return `${String(service || '').toLowerCase()}:${String(creatorId || '').toLowerCase()}:${normId}`;
}

export function creatorCacheKey(service: string, creatorId: string | number) {
  return `${String(service || '').toLowerCase()}:${String(creatorId || '').toLowerCase()}`;
}

export class ContentState {
  posts = $state<Record<string, CachedPost>>({});
  creators = $state<Record<string, CachedCreator>>({});

  getPostAccent(service: string, creatorId: string | number, postId: string | number): string | undefined {
    const key = postCacheKey(service, creatorId, postId);
    return this.posts[key]?.accentColor;
  }

  setPostAccent(service: string, creatorId: string | number, postId: string | number, color: string) {
    if (!color) return;
    const key = postCacheKey(service, creatorId, postId);
    const entry = this.getPost(service, creatorId, postId);
    this.posts[key] = {
      ...entry,
      accentColor: color
    };
  }

  getCreatorAccent(service: string, creatorId: string | number): string | undefined {
    const key = creatorCacheKey(service, creatorId);
    return this.creators[key]?.accentColor;
  }

  setCreatorAccent(service: string, creatorId: string | number, color: string) {
    if (!color) return;
    const key = creatorCacheKey(service, creatorId);
    const entry = this.getCreator(service, String(creatorId));
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
  }

  setPosts(posts: Post[]) {
    for (const post of posts) {
      this.setPost(post);
    }
  }

  getPost(service: string, creatorId: string | number, postId: unknown) {
    const key = postCacheKey(service, creatorId, postId);
    this.posts[key] ??= { post: null, loading: false, loaded: false, error: null };
    return this.posts[key];
  }

  async loadPost(service: string, creatorId: string | number, rawPostId: unknown, force = false) {
    const postId = normalizePostId(rawPostId);
    if (!postId || !service || !creatorId) return;

    const key = postCacheKey(service, creatorId, postId);
    const entry = this.getPost(service, creatorId, postId);
    if (!force && ((entry.loaded && entry.post?.detail_fetched) || entry.loading || entry.error)) return;
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
          loaded: true,
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
    const key = creatorCacheKey(service, creatorId);
    const entry = this.getCreator(service, creatorId);
    if (entry.loaded || entry.loading) return;

    const cachedName = creatorsState.creatorsMap.get(`${service.toLowerCase()}:${creatorId.toLowerCase()}`);
    const profile = entry.profile && entry.profile.name !== creatorId ? entry.profile : {
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
      profile,
      loading: true,
      error: null
    };

    try {
      void apiFetchCreatorProfile(service, creatorId).then((p) => {
        if (p && p.name && p.name !== creatorId) {
          const cur = this.creators[key] ?? entry;
          this.creators[key] = {
            ...cur,
            profile: p
          };
        }
      }).catch(() => {});

      const posts = await apiFetchCreatorPosts(service, creatorId, undefined, 0);
      const cur = this.creators[key] ?? entry;
      let finalProfile = cur.profile || profile;

      if (finalProfile && (finalProfile.name === creatorId || !finalProfile.name)) {
        const foundName = creatorsState.creatorsMap.get(`${service.toLowerCase()}:${creatorId.toLowerCase()}`);
        if (foundName && foundName !== creatorId) {
          finalProfile = { ...finalProfile, name: foundName };
        } else {
          void creatorsState.load().then(() => {
            const resolved = creatorsState.creatorsMap.get(`${service.toLowerCase()}:${creatorId.toLowerCase()}`);
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

      this.creators[key] = {
        ...cur,
        profile: finalProfile,
        posts,
        offset: PAGE_SIZE,
        hasMore: posts.length === PAGE_SIZE,
        loaded: true,
        loading: false,
        error: null
      };
      logger.info(`[Content] Loaded ${posts.length} posts for ${service}:${creatorId}`);
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

  async refreshCreator(service: string, creatorId: string) {
    const key = creatorCacheKey(service, creatorId);
    const entry = this.getCreator(service, creatorId);
    if (entry.loading) return;

    this.creators[key] = {
      ...entry,
      loading: true,
      error: null
    };

    try {
      void apiFetchCreatorProfile(service, creatorId).then((p) => {
        if (p && p.name && p.name !== creatorId) {
          const cur = this.creators[key] ?? entry;
          this.creators[key] = {
            ...cur,
            profile: p
          };
        }
      }).catch(() => {});

      const posts = await apiFetchCreatorPosts(service, creatorId, undefined, 0);
      const cur = this.creators[key] ?? entry;
      this.creators[key] = {
        ...cur,
        posts,
        offset: PAGE_SIZE,
        hasMore: posts.length === PAGE_SIZE,
        loaded: true,
        loading: false,
        error: null
      };
      logger.info(`[Content] Refreshed ${posts.length} posts for ${service}:${creatorId}`);
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

  async loadMoreCreatorPosts(service: string, creatorId: string) {
    const key = creatorCacheKey(service, creatorId);
    const entry = this.getCreator(service, creatorId);
    if (entry.loadingMore || !entry.hasMore) return;

    this.creators[key] = {
      ...entry,
      loadingMore: true,
      error: null
    };

    try {
      const posts = await apiFetchCreatorPosts(service, creatorId, undefined, entry.offset);
      const cur = this.creators[key] ?? entry;
      this.creators[key] = {
        ...cur,
        posts: [...cur.posts, ...posts],
        offset: cur.offset + PAGE_SIZE,
        hasMore: posts.length === PAGE_SIZE,
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
}

export const contentState = new ContentState();
