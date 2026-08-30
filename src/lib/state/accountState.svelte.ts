import type { AccountSession, Creator, Favorite, Post } from '$lib/types/content';
import { apiGetAccountSession, apiLoginAccount, apiLogoutAccount, apiFetchAccountFavorites } from '$lib/utils/ipc';

class AccountState {
  session = $state<AccountSession>({ authenticated: false });
  loading = $state(false);
  checked = $state(false);

  favoritePosts = $state<Favorite[] | null>(null);
  favoriteCreators = $state<Favorite[] | null>(null);

  async refresh() {
    this.loading = true;
    try {
      this.session = await apiGetAccountSession();
      void this.fetchFavorites('post');
      void this.fetchFavorites('creator');
    } finally {
      this.loading = false;
      this.checked = true;
    }
  }

  async login(username: string, password: string) {
    this.loading = true;
    try {
      this.session = await apiLoginAccount(username, password);
      this.checked = true;
      this.clearFavorites();
      if (this.session.authenticated) {
        void this.fetchFavorites('post', true);
        void this.fetchFavorites('creator', true);
      }
    } finally {
      this.loading = false;
    }
  }

  async logout() {
    this.loading = true;
    try {
      this.session = await apiLogoutAccount();
      this.checked = true;
      this.clearFavorites();
      void this.fetchFavorites('post', true);
      void this.fetchFavorites('creator', true);
    } finally {
      this.loading = false;
    }
  }

  clearFavorites() {
    this.favoritePosts = null;
    this.favoriteCreators = null;
  }

  async fetchFavorites(type: 'post' | 'creator', force = false) {
    if (type === 'post') {
      if (this.favoritePosts !== null && !force) return this.favoritePosts;
      const res = await apiFetchAccountFavorites('post');
      this.favoritePosts = res;
      return res;
    } else {
      if (this.favoriteCreators !== null && !force) return this.favoriteCreators;
      const res = await apiFetchAccountFavorites('artist');
      this.favoriteCreators = res;
      return res;
    }
  }

  isPostFavorite(service: string, creatorId: string, postId: string): boolean {
    if (!this.favoritePosts) return false;
    const s = service.toLowerCase();
    const c = String(creatorId).toLowerCase();
    const p = String(postId).toLowerCase();
    return this.favoritePosts.some((f: any) => {
      const fService = String(f.service ?? '').toLowerCase();
      const fUser = String(f.user ?? f.user_id ?? f.creator_id ?? f.extra?.user ?? f.extra?.user_id ?? '').toLowerCase();
      const fId = String(f.id ?? f.post_id ?? '').toLowerCase();
      return fService === s && fUser === c && fId === p;
    });
  }

  isCreatorFavorite(service: string, creatorId: string): boolean {
    if (!this.favoriteCreators) return false;
    const s = service.toLowerCase();
    const c = String(creatorId).toLowerCase();
    return this.favoriteCreators.some((f: any) => {
      const fService = String(f.service ?? '').toLowerCase();
      const fId = String(f.id ?? f.user ?? f.user_id ?? f.creator_id ?? f.extra?.id ?? '').toLowerCase();
      return fService === s && fId === c;
    });
  }

  addPostFavoriteOptimistic(post: Post | Favorite) {
    const nowIso = new Date().toISOString();
    if (!this.favoritePosts) {
      this.favoritePosts = [{ ...post, faved_seq: 1, faved_at: nowIso } as Favorite];
      return;
    }
    const exists = this.isPostFavorite(
      post.service ?? '',
      String((post as any).user ?? (post as any).user_id ?? ''),
      String(post.id ?? '')
    );
    if (!exists) {
      const highestSeq = Math.max(0, ...this.favoritePosts.map((f) => Number(f.faved_seq ?? 0)));
      const newFav: Favorite = {
        ...post,
        faved_seq: highestSeq + 1,
        faved_at: nowIso
      } as any;
      this.favoritePosts = [newFav, ...this.favoritePosts];
    }
  }

  removePostFavoriteOptimistic(service: string, creatorId: string, postId: string) {
    if (!this.favoritePosts) return;
    const s = service.toLowerCase();
    const c = String(creatorId).toLowerCase();
    const p = String(postId).toLowerCase();
    this.favoritePosts = this.favoritePosts.filter((f: any) => {
      const fService = String(f.service ?? '').toLowerCase();
      const fUser = String(f.user ?? f.user_id ?? f.creator_id ?? f.extra?.user ?? f.extra?.user_id ?? '').toLowerCase();
      const fId = String(f.id ?? f.post_id ?? '').toLowerCase();
      return !(fService === s && fUser === c && fId === p);
    });
  }

  addCreatorFavoriteOptimistic(creator: Creator | Favorite) {
    const nowIso = new Date().toISOString();
    if (!this.favoriteCreators) {
      this.favoriteCreators = [{ ...creator, faved_seq: 1, faved_at: nowIso } as Favorite];
      return;
    }
    const exists = this.isCreatorFavorite(creator.service ?? '', String(creator.id ?? ''));
    if (!exists) {
      const highestSeq = Math.max(0, ...this.favoriteCreators.map((f) => Number(f.faved_seq ?? 0)));
      const newFav: Favorite = {
        ...creator,
        faved_seq: highestSeq + 1,
        faved_at: nowIso
      } as any;
      this.favoriteCreators = [newFav, ...this.favoriteCreators];
    }
  }

  removeCreatorFavoriteOptimistic(service: string, creatorId: string) {
    if (!this.favoriteCreators) return;
    const s = service.toLowerCase();
    const c = String(creatorId).toLowerCase();
    this.favoriteCreators = this.favoriteCreators.filter((f: any) => {
      const fService = String(f.service ?? '').toLowerCase();
      const fId = String(f.id ?? f.user ?? f.user_id ?? f.creator_id ?? f.extra?.id ?? '').toLowerCase();
      return !(fService === s && fId === c);
    });
  }
}

export const accountState = new AccountState();
