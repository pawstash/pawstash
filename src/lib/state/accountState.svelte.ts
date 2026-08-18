import type { AccountSession, Creator, Favorite, PawchivePost } from '$lib/types/pawchive';
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
    return this.favoritePosts.some(
      (f) =>
        f.service?.toLowerCase() === s &&
        String(f.user ?? f.user_id ?? '').toLowerCase() === c &&
        String(f.id ?? '').toLowerCase() === p
    );
  }

  isCreatorFavorite(service: string, creatorId: string): boolean {
    if (!this.favoriteCreators) return false;
    const s = service.toLowerCase();
    const c = String(creatorId).toLowerCase();
    return this.favoriteCreators.some(
      (f) =>
        f.service?.toLowerCase() === s &&
        String(f.id ?? f.user ?? f.user_id ?? '').toLowerCase() === c
    );
  }

  addPostFavoriteOptimistic(post: PawchivePost | Favorite) {
    if (!this.favoritePosts) {
      this.favoritePosts = [post as Favorite];
      return;
    }
    const exists = this.isPostFavorite(
      post.service ?? '',
      String((post as any).user ?? (post as any).user_id ?? ''),
      String(post.id ?? '')
    );
    if (!exists) {
      this.favoritePosts = [post as Favorite, ...this.favoritePosts];
    }
  }

  removePostFavoriteOptimistic(service: string, creatorId: string, postId: string) {
    if (!this.favoritePosts) return;
    const s = service.toLowerCase();
    const c = String(creatorId).toLowerCase();
    const p = String(postId).toLowerCase();
    this.favoritePosts = this.favoritePosts.filter(
      (f) =>
        !(
          f.service?.toLowerCase() === s &&
          String(f.user ?? f.user_id ?? '').toLowerCase() === c &&
          String(f.id ?? '').toLowerCase() === p
        )
    );
  }

  addCreatorFavoriteOptimistic(creator: Creator | Favorite) {
    if (!this.favoriteCreators) {
      this.favoriteCreators = [creator as Favorite];
      return;
    }
    const exists = this.isCreatorFavorite(creator.service ?? '', String(creator.id ?? ''));
    if (!exists) {
      this.favoriteCreators = [creator as Favorite, ...this.favoriteCreators];
    }
  }

  removeCreatorFavoriteOptimistic(service: string, creatorId: string) {
    if (!this.favoriteCreators) return;
    const s = service.toLowerCase();
    const c = String(creatorId).toLowerCase();
    this.favoriteCreators = this.favoriteCreators.filter(
      (f) =>
        !(
          f.service?.toLowerCase() === s &&
          String(f.id ?? f.user ?? f.user_id ?? '').toLowerCase() === c
        )
    );
  }
}

export const accountState = new AccountState();
