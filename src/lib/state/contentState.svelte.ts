import type { CreatorProfile, PawchivePost } from '$lib/types/pawchive';
import { apiFetchCreatorPosts, apiFetchCreatorProfile, apiFetchPost } from '$lib/utils/ipc';

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

export function postCacheKey(service: string, creatorId: string, postId: string) {
  return `${service}:${creatorId}:${postId}`;
}

export function creatorCacheKey(service: string, creatorId: string) {
  return `${service}:${creatorId}`;
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
      existing.post = { ...existing.post, ...post };
    }
  }

  getPost(service: string, creatorId: string, postId: string) {
    const key = postCacheKey(service, creatorId, postId);
    this.posts[key] ??= { post: null, loading: false, loaded: false, error: null };
    return this.posts[key];
  }

  async loadPost(service: string, creatorId: string, postId: string) {
    const entry = this.getPost(service, creatorId, postId);
    if (entry.loaded || entry.loading) return;
    entry.loading = true;
    entry.error = null;
    try {
      const detail = await apiFetchPost(service, creatorId, postId);
      entry.post = {
        ...entry.post,
        ...detail,
        favorite_count: entry.post?.favorite_count ?? detail.favorite_count,
        attachment_count: detail.attachments?.length ?? entry.post?.attachment_count,
        detail_fetched: true
      };
      entry.loaded = true;
    } catch (error) {
      entry.error = errorMessage(error);
    } finally {
      entry.loading = false;
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
      const [profile, posts] = await Promise.all([
        apiFetchCreatorProfile(service, creatorId),
        apiFetchCreatorPosts(service, creatorId, undefined, 0)
      ]);
      entry.profile = profile;
      entry.posts = posts;
      entry.offset = PAGE_SIZE;
      entry.hasMore = posts.length === PAGE_SIZE;
      entry.loaded = true;
    } catch (error) {
      entry.error = errorMessage(error);
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
      const [profile, posts] = await Promise.all([
        apiFetchCreatorProfile(service, creatorId),
        apiFetchCreatorPosts(service, creatorId, undefined, 0)
      ]);
      entry.profile = profile;
      entry.posts = posts;
      entry.offset = PAGE_SIZE;
      entry.hasMore = posts.length === PAGE_SIZE;
      entry.loaded = true;
    } catch (error) {
      entry.error = errorMessage(error);
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
    } finally {
      entry.loadingMore = false;
    }
  }
}

export const contentState = new ContentState();
