import type { Post } from '$lib/types/content';
import type { LibraryCollection } from '$lib/types/library';
import { i18n } from '$lib/i18n';
import {
  apiClearLibraryStash,
  apiCreateLibraryStash,
  apiDeleteLibraryStash,
  apiListLibraryCollections,
  apiListLibraryPosts,
  apiListPostCollections,
  apiListPostStashMemberships,
  apiListSavedPostIdentities,
  apiRemoveLibraryPost,
  apiRemoveLibraryPostFromStash,
  apiRenameLibraryStash,
  apiReorderLibraryStashes,
  apiSaveLibraryPost,
  apiUpdateLibraryStash
} from '$lib/utils/ipc';
import { logger } from '$lib/utils/logger';

const PAGE_SIZE = 50;

export function libraryPostKey(post: Pick<Post, 'service' | 'user' | 'id'>) {
  return `${(post.service || '').toLowerCase()}:${post.user}:${post.id}`;
}

export class LibraryState {
  collections = $state<LibraryCollection[]>([]);
  posts = $state<Post[]>([]);
  selectedCollectionId = $state<string | null>(null);
  loading = $state(false);
  collectionsLoading = $state(false);
  initialized = $state(false);
  hasMore = $state(true);
  error = $state<string | null>(null);
  pendingKeys = $state<Set<string>>(new Set());
  postStashes = $state<Record<string, string[]>>({});
  private savedKeys = $state<Set<string>>(new Set());
  private offset = 0;
  private requestId = 0;
  private initPromise: Promise<void> | null = null;

  get inbox() {
    return this.collections.find((collection) => collection.kind === 'inbox');
  }

  get allStashes(): LibraryCollection[] {
    return this.collections.filter((collection) => collection.kind === 'stash' || collection.kind === 'inbox');
  }

  getStashDisplayName(collection: LibraryCollection): string {
    if (collection.kind === 'inbox') {
      if (!collection.name || collection.name === 'Inbox' || collection.name === 'Main Stash') {
        return i18n.t('library.inbox');
      }
      return collection.name;
    }
    return collection.name;
  }

  get stashOptions() {
    return this.allStashes.map((collection) => ({
      value: collection.id,
      label: this.getStashDisplayName(collection),
      color: collection.color || undefined
    }));
  }

  stashesForPosts(posts: Pick<Post, 'service' | 'user' | 'id'>[]): string[] {
    if (posts.length === 0) return [];
    const counts = new Map<string, number>();
    for (const post of posts) {
      for (const id of this.getPostStashes(post)) {
        counts.set(id, (counts.get(id) || 0) + 1);
      }
    }
    return [...counts.entries()]
      .filter(([, count]) => count === posts.length)
      .map(([id]) => id);
  }

  get selectedCollection() {
    return this.selectedCollectionId
      ? this.collections.find((collection) => collection.id === this.selectedCollectionId) ?? null
      : null;
  }

  isSaved(post: Pick<Post, 'service' | 'user' | 'id'>) {
    return this.savedKeys.has(libraryPostKey(post));
  }

  isPending(post: Pick<Post, 'service' | 'user' | 'id'>) {
    return this.pendingKeys.has(libraryPostKey(post));
  }

  getPostStashes(post: Pick<Post, 'service' | 'user' | 'id'>): string[] {
    const key = libraryPostKey(post);
    const ids = this.postStashes[key];
    if (!ids || ids.length === 0) return [];
    const validCollections = new Set(this.collections.map((c) => c.id));
    return ids.filter((id) => validCollections.has(id));
  }

  getCustomPostStashes(post: Pick<Post, 'service' | 'user' | 'id'>): string[] {
    const key = libraryPostKey(post);
    const ids = this.postStashes[key];
    if (!ids || ids.length === 0) return [];
    const validStashes = new Set(this.collections.filter((c) => c.kind === 'stash').map((c) => c.id));
    return ids.filter((id) => validStashes.has(id));
  }

  getPostStashNames(post: Pick<Post, 'service' | 'user' | 'id'>): string[] {
    const stashIds = this.getPostStashes(post);
    const stashMap = new Map(this.collections.map((c) => [c.id, this.getStashDisplayName(c)]));
    return stashIds.map((id) => stashMap.get(id)).filter((name): name is string => Boolean(name));
  }

  async loadPostStashes(post: Pick<Post, 'service' | 'user' | 'id'>): Promise<string[]> {
    const key = libraryPostKey(post);
    try {
      const collectionIds = await apiListPostCollections(post.service, post.user, post.id);
      this.postStashes[key] = collectionIds;
      return this.getCustomPostStashes(post);
    } catch {
      return [];
    }
  }

  init() {
    if (this.initPromise) return this.initPromise;
    this.initPromise = this.initialize()
      .then(() => {
        this.initialized = true;
      })
      .catch((error) => {
        this.initPromise = null;
        throw error;
      });
    return this.initPromise;
  }

  private async initialize() {
    await Promise.all([this.refreshCollections(), this.refreshSavedKeys()]);
  }

  async refreshCollections() {
    this.collectionsLoading = true;
    try {
      this.collections = await apiListLibraryCollections();
    } finally {
      this.collectionsLoading = false;
    }
  }

  async refreshSavedKeys() {
    try {
      const [identities, memberships] = await Promise.all([
        apiListSavedPostIdentities(),
        apiListPostStashMemberships()
      ]);
      this.savedKeys = new Set(
        identities.map((identity) => `${(identity.service || '').toLowerCase()}:${identity.creator_id}:${identity.post_id}`)
      );
      const map: Record<string, string[]> = {};
      for (const m of memberships) {
        const k = `${(m.service || '').toLowerCase()}:${m.creator_id}:${m.post_id}`;
        if (!map[k]) map[k] = [];
        map[k].push(m.collection_id);
      }
      this.postStashes = map;
    } catch (e) {
      logger.warn('Failed to refresh saved keys/memberships', e);
    }
  }

  async selectCollection(collectionId: string | null) {
    if (this.selectedCollectionId === collectionId && this.posts.length > 0) return;
    this.selectedCollectionId = collectionId;
    await this.refresh();
  }

  async load(reset = false) {
    if (this.loading && !reset) return;
    const requestId = ++this.requestId;
    const offset = reset ? 0 : this.offset;
    const collectionId = this.selectedCollectionId ?? undefined;
    this.loading = true;
    this.error = null;
    try {
      const posts = await apiListLibraryPosts(collectionId, offset, PAGE_SIZE);
      if (requestId !== this.requestId) return;
      const combined = reset ? posts : [...this.posts, ...posts];
      this.posts = [...new Map(combined.map((post) => [libraryPostKey(post), post])).values()];
      this.offset = offset + posts.length;
      this.hasMore = posts.length === PAGE_SIZE;

      if (collectionId && this.selectedCollection?.kind === 'stash') {
        for (const p of posts) {
          const k = libraryPostKey(p);
          const current = this.postStashes[k] || [];
          if (!current.includes(collectionId)) {
            this.postStashes[k] = [...current, collectionId];
          }
        }
      }
    } catch (error) {
      if (requestId === this.requestId) {
        this.error = error instanceof Error ? error.message : String(error);
      }
    } finally {
      if (requestId === this.requestId) this.loading = false;
    }
  }

  refresh() {
    this.offset = 0;
    this.hasMore = true;
    return this.load(true);
  }

  loadMore() {
    return this.load(false);
  }

  async save(post: Post, collectionId?: string) {
    const key = libraryPostKey(post);
    if (this.pendingKeys.has(key)) return;
    this.pendingKeys = new Set(this.pendingKeys).add(key);

    const previousSaved = this.savedKeys.has(key);
    const previousStashes = this.postStashes[key] ? [...this.postStashes[key]] : [];
    this.savedKeys = new Set(this.savedKeys).add(key);
    if (collectionId) {
      const current = this.postStashes[key] || [];
      if (!current.includes(collectionId)) {
        this.postStashes[key] = [...current, collectionId];
      }
    }

    try {
      await apiSaveLibraryPost(post, collectionId);
      void this.refreshCollections();
      const target = collectionId ?? this.inbox?.id;
      if (this.selectedCollectionId === null || this.selectedCollectionId === target) {
        void this.refresh();
      }
    } catch (err) {
      if (!previousSaved) {
        const rollbackSaved = new Set(this.savedKeys);
        rollbackSaved.delete(key);
        this.savedKeys = rollbackSaved;
      }
      this.postStashes[key] = previousStashes;
      throw err;
    } finally {
      const pending = new Set(this.pendingKeys);
      pending.delete(key);
      this.pendingKeys = pending;
    }
  }

  async remove(post: Pick<Post, 'service' | 'user' | 'id'>) {
    const key = libraryPostKey(post);
    if (this.pendingKeys.has(key)) return;
    this.pendingKeys = new Set(this.pendingKeys).add(key);

    const previousSaved = this.savedKeys.has(key);
    const previousStashes = this.postStashes[key] ? [...this.postStashes[key]] : [];
    const previousPosts = [...this.posts];

    const saved = new Set(this.savedKeys);
    saved.delete(key);
    this.savedKeys = saved;
    delete this.postStashes[key];
    this.posts = this.posts.filter((item) => libraryPostKey(item) !== key);

    try {
      await apiRemoveLibraryPost(post.service, post.user, post.id);
      void this.refreshCollections();
    } catch (err) {
      if (previousSaved) {
        this.savedKeys = new Set(this.savedKeys).add(key);
      }
      this.postStashes[key] = previousStashes;
      this.posts = previousPosts;
      throw err;
    } finally {
      const pending = new Set(this.pendingKeys);
      pending.delete(key);
      this.pendingKeys = pending;
    }
  }

  toggle(post: Post) {
    return this.isSaved(post) ? this.remove(post) : this.save(post);
  }

  async createStash(name: string, color?: string | null) {
    const collection = await apiCreateLibraryStash(name, color);
    this.collections = [...this.collections, collection];
    return collection;
  }

  async updateStash(collectionId: string, updates: { name?: string; color?: string | null }) {
    const updated = await apiUpdateLibraryStash(collectionId, updates);
    this.collections = this.collections.map((c) => (c.id === collectionId ? updated : c));
    return updated;
  }

  async renameStash(collectionId: string, name: string) {
    const renamed = await apiRenameLibraryStash(collectionId, name);
    if (!renamed) return false;
    this.collections = this.collections.map((c) => (c.id === collectionId ? { ...c, name } : c));
    return true;
  }

  async reorderStashes(collectionIds: string[]) {
    const stashMap = new Map(this.collections.map((c) => [c.id, c]));
    const nonStashes = this.collections.filter((c) => c.kind !== 'stash');
    const orderedStashes: LibraryCollection[] = [];
    for (const id of collectionIds) {
      const c = stashMap.get(id);
      if (c) orderedStashes.push(c);
    }
    for (const c of this.collections) {
      if (c.kind === 'stash' && !collectionIds.includes(c.id)) {
        orderedStashes.push(c);
      }
    }
    this.collections = [...nonStashes, ...orderedStashes];
    try {
      await apiReorderLibraryStashes(collectionIds);
    } catch (e) {
      logger.error('Failed to persist stash reorder', e);
      await this.refreshCollections();
    }
  }

  async clearStash(collectionId: string) {
    const clearedCount = await apiClearLibraryStash(collectionId);
    await this.refreshCollections();
    await this.refreshSavedKeys();
    if (this.selectedCollectionId === collectionId) {
      this.posts = [];
      this.offset = 0;
      this.hasMore = false;
    }
    return clearedCount;
  }

  async removeFromStash(collectionId: string, post: Pick<Post, 'service' | 'user' | 'id'>) {
    const key = libraryPostKey(post);
    const previous = this.postStashes[key] || [];
    const previousPosts = [...this.posts];
    this.postStashes[key] = previous.filter((id) => id !== collectionId);
    if (this.selectedCollectionId === collectionId) {
      this.posts = this.posts.filter((item) => libraryPostKey(item) !== key);
    }
    try {
      const removed = await apiRemoveLibraryPostFromStash(collectionId, post.service, post.user, post.id);
      if (!removed) {
        this.postStashes[key] = previous;
        this.posts = previousPosts;
        return false;
      }
      void this.refreshCollections();
      void this.refreshSavedKeys();
      return true;
    } catch (err) {
      this.postStashes[key] = previous;
      this.posts = previousPosts;
      throw err;
    }
  }

  async deleteStash(collectionId: string) {
    const deleted = await apiDeleteLibraryStash(collectionId);
    if (!deleted) return false;
    this.collections = this.collections.filter((collection) => collection.id !== collectionId);
    if (this.selectedCollectionId === collectionId) await this.selectCollection(null);
    return true;
  }
}

export const libraryState = new LibraryState();
