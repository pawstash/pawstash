import { logger } from '$lib/utils/logger';

export type RootRouteName = 'feed' | 'favorites' | 'library' | 'creators' | 'downloads' | 'profile' | 'settings';

export type AppRoute =
  | { name: RootRouteName }
  | { name: 'post'; service: string; creatorId: string; postId: string; initialMedia?: string; openViewer?: boolean }
  | { name: 'creator'; service: string; creatorId: string };

interface HistoryEntry {
  pawstash: true;
  key: string;
  index: number;
  root?: RootRouteName;
}

function rootForRoute(route: AppRoute): RootRouteName {
  return route.name === 'post' || route.name === 'creator' ? 'feed' : route.name;
}

function encode(value: string) {
  return encodeURIComponent(value);
}

function parseRoute(hash: string): AppRoute {
  const [routePath, queryString] = hash.replace(/^#\/?/, '').split('?');
  const parts = routePath.split('/').filter(Boolean).map(decodeURIComponent);
  const searchParams = new URLSearchParams(queryString || '');
  if (parts[0] === 'post' && parts.length >= 4) {
    const initialMedia = searchParams.get('media') || undefined;
    const openViewer = searchParams.get('viewer') === '1' ? true : undefined;
    return {
      name: 'post',
      service: parts[1],
      creatorId: parts[2],
      postId: parts[3],
      initialMedia,
      openViewer
    };
  }
  if (parts[0] === 'creator' && parts.length >= 3) {
    return { name: 'creator', service: parts[1], creatorId: parts[2] };
  }
  if (
    parts[0] === 'library' ||
    parts[0] === 'favorites' ||
    parts[0] === 'creators' ||
    parts[0] === 'downloads' ||
    parts[0] === 'profile' ||
    parts[0] === 'settings'
  ) {
    return { name: parts[0] };
  }
  return { name: 'feed' };
}

function routeHash(route: AppRoute) {
  if (route.name === 'post') {
    const params = new URLSearchParams();
    if (route.initialMedia) params.set('media', route.initialMedia);
    if (route.openViewer) params.set('viewer', '1');
    const qs = params.toString();
    return `#/post/${encode(route.service)}/${encode(route.creatorId)}/${encode(route.postId)}${qs ? `?${qs}` : ''}`;
  }
  if (route.name === 'creator') {
    return `#/creator/${encode(route.service)}/${encode(route.creatorId)}`;
  }
  return `#/${route.name}`;
}

export class NavigationState {
  route = $state<AppRoute>({ name: 'feed' });
  entryKey = $state('initial');
  canGoBack = $state(false);
  canGoForward = $state(false);
  activeRoot = $state<RootRouteName>('feed');
  private scrollPositions = new Map<string, number>();
  private viewStates = new Map<string, Record<string, any>>();
  private backHandlers: Array<() => boolean> = [];
  private initialized = false;
  private index = 0;
  private maxIndex = 0;

  init() {
    if (this.initialized || typeof window === 'undefined') return;
    this.initialized = true;
    history.scrollRestoration = 'manual';
    const existing = history.state as (HistoryEntry & { modal?: boolean }) | null;
    this.entryKey = existing?.pawstash ? existing.key : crypto.randomUUID();
    this.index = existing?.pawstash ? existing.index : 0;
    this.maxIndex = this.index;
    this.canGoBack = this.index > 0;
    this.canGoForward = false;
    this.route = parseRoute(location.hash);
    this.activeRoot = existing?.root ?? rootForRoute(this.route);
    if (!existing?.pawstash) {
      history.replaceState(
        { pawstash: true, key: this.entryKey, index: 0, root: this.activeRoot } satisfies HistoryEntry,
        '',
        routeHash(this.route)
      );
    }

    window.addEventListener('popstate', (event) => {
      if (this.backHandlers.length > 0) {
        const handler = this.backHandlers.pop();
        if (handler && handler()) {
          return;
        }
      }

      const entry = event.state as HistoryEntry | null;
      this.entryKey = entry?.pawstash ? entry.key : crypto.randomUUID();
      this.route = parseRoute(location.hash);
      this.activeRoot = entry?.root ?? rootForRoute(this.route);
      this.index = entry?.pawstash ? entry.index : 0;
      this.canGoBack = this.index > 0;
      this.canGoForward = this.index < this.maxIndex;
    });
  }

  registerBackHandler(handler: () => boolean): () => void {
    let pushedModalState = false;
    if (typeof window !== 'undefined') {
      try {
        history.pushState(
          { pawstash: true, key: this.entryKey, index: this.index, root: this.activeRoot, modal: true },
          '',
          location.hash
        );
        pushedModalState = true;
      } catch (e) {
        logger.warn('Failed to push modal history state', e);
      }
    }

    const wrappedHandler = () => {
      pushedModalState = false;
      return handler();
    };

    this.backHandlers.push(wrappedHandler);

    return () => {
      const idx = this.backHandlers.indexOf(wrappedHandler);
      if (idx !== -1) {
        this.backHandlers.splice(idx, 1);
        if (pushedModalState && typeof window !== 'undefined' && (history.state as any)?.modal) {
          pushedModalState = false;
          history.back();
        }
      }
    };
  }

  navigate(route: AppRoute) {
    const hash = routeHash(route);
    if (hash === location.hash) return;
    const key = crypto.randomUUID();
    const index = this.index + 1;
    const root = rootForRoute(route);
    const activeRoot = route.name === 'post' || route.name === 'creator' ? this.activeRoot : root;
    history.pushState({ pawstash: true, key, index, root: activeRoot } satisfies HistoryEntry, '', hash);
    this.entryKey = key;
    this.route = route;
    this.activeRoot = activeRoot;
    this.index = index;
    this.maxIndex = index;
    this.canGoBack = true;
    this.canGoForward = false;
  }

  navigateRoot(name: RootRouteName) {
    this.navigate({ name });
  }

  openPost(service: string, creatorId: string, postId: string, initialMedia?: string, openViewer?: boolean) {
    this.navigate({ name: 'post', service, creatorId, postId, initialMedia, openViewer });
  }

  openCreator(service: string, creatorId: string) {
    this.navigate({ name: 'creator', service, creatorId });
  }

  back() {
    if (this.backHandlers.length > 0) {
      const handler = this.backHandlers.pop();
      if (handler && handler()) {
        if (typeof window !== 'undefined' && (history.state as any)?.modal) {
          history.back();
        }
        return;
      }
    }

    if (this.canGoBack) history.back();
    else {
      const route: AppRoute = { name: 'feed' };
      const key = crypto.randomUUID();
      history.replaceState(
        { pawstash: true, key, index: 0, root: 'feed' } satisfies HistoryEntry,
        '',
        routeHash(route)
      );
      this.entryKey = key;
      this.route = route;
      this.activeRoot = 'feed';
      this.index = 0;
      this.maxIndex = 0;
      this.canGoBack = false;
      this.canGoForward = false;
    }
  }

  forward() {
    if (this.canGoForward) history.forward();
  }

  rememberScroll(key: string, scrollTop: number) {
    this.scrollPositions.set(key, scrollTop);
  }

  scrollFor(key: string) {
    return this.scrollPositions.get(key) ?? 0;
  }

  saveViewState(key: string, state: Record<string, any>) {
    const existing = this.viewStates.get(key) ?? {};
    this.viewStates.set(key, { ...existing, ...state });
  }

  getViewState<T = Record<string, any>>(key: string): T | undefined {
    return this.viewStates.get(key) as T | undefined;
  }
}

export const navigationState = new NavigationState();
