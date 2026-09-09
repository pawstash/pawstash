import {
  apiListCreatorsPage,
  apiListCreatorServices,
  apiListCreatorNames,
  apiGetCreatorName,
  apiSyncCreators,
  type CreatorsQueryParams
} from '$lib/utils/ipc';
import { logger } from '$lib/utils/logger';
import type { Creator } from '$lib/types/content';
import type { FilterMap, TriStateFilter } from '$lib/types/filter';
import { configState } from './configState.svelte';
import { providerState } from './providerState.svelte';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export class CreatorsState {
  creators = $state.raw<Creator[]>([]);
  loading = $state(false);
  loadingMore = $state(false);
  syncing = $state(false);
  error = $state<string | null>(null);
  loaded = $state(false);
  totalCount = $state(0);
  hasMore = $state(false);

  private _searchQuery = $state('');
  providerFilters = $state<FilterMap>({});
  serviceFilters = $state<FilterMap>({});
  aiFilter = $state<TriStateFilter>('neutral');
  sortBy = $state<'name' | 'updated' | 'indexed' | 'favorited'>('favorited');
  sortOrder = $state<'asc' | 'desc'>('desc');
  activeTab = $state<'all' | 'subscribed'>('all');

  services = $state<string[]>([]);

  get enabledServices(): string[] {
    const list = this.services.filter((s) => providerState.isServiceEnabled(s));
    if (list.length > 0) return list;
    if (this.services.length > 0) return this.services;
    const defaults: string[] = [];
    for (const p of providerState.providers) {
      if (p.enabled) {
        defaults.push(...p.services);
      }
    }
    return [...new Set(defaults)].sort();
  }

  // Lookup index "service:id" -> name. Kept outside $state to avoid reactive proxy overhead.
  creatorsMap = new Map<string, string>();

  get searchQuery(): string {
    return this._searchQuery;
  }

  set searchQuery(val: string) {
    if (this._searchQuery === val) return;
    this._searchQuery = val;
    this.scheduleSearch(250);
  }

  get filteredCreators(): Creator[] {
    return this.creators;
  }

  private _unlisten: UnlistenFn | null = null;
  private _queryId = 0;
  private _debounceTimer: ReturnType<typeof setTimeout> | undefined;
  private _initialized = false;

  async init() {
    if (this._initialized) return;
    this._initialized = true;

    void listen<number>('creators-updated', (event) => {
      logger.info(`[Creators] Background sync updated ${event.payload} creators`);
      void this.loadServices();
      void this.loadNamesMap();
      void this.loadPage(false);
    }).then((un) => {
      this._unlisten = un;
    });

    if (providerState.providers.length === 0) {
      await providerState.loadProviders();
    }
    await this.loadServices();
    void this.loadNamesMap();
    await this.loadPage(false);
  }

  destroy() {
    this._unlisten?.();
    this._unlisten = null;
    if (this._debounceTimer) clearTimeout(this._debounceTimer);
  }

  async loadServices() {
    try {
      this.services = await apiListCreatorServices();
    } catch (e) {
      logger.warn('[Creators] Failed to load creator services', e);
    }
  }

  async loadNamesMap() {
    try {
      const names = await apiListCreatorNames();
      for (const [key, name] of Object.entries(names)) {
        this.creatorsMap.set(key, name);
      }
    } catch (e) {
      logger.warn('[Creators] Failed to load creator names map', e);
    }
  }

  getName(service?: string, creatorId?: string): string | undefined {
    if (!service || !creatorId) return undefined;
    const key = `${service.toLowerCase()}:${creatorId.toLowerCase()}`;
    const cached = this.creatorsMap.get(key);
    if (cached) return cached;

    void apiGetCreatorName(service, creatorId).then((name) => {
      if (name) {
        this.creatorsMap.set(key, name);
      }
    });
    return undefined;
  }

  async resolveName(service?: string, creatorId?: string): Promise<string | undefined> {
    if (!service || !creatorId) return undefined;
    const key = `${service.toLowerCase()}:${creatorId.toLowerCase()}`;
    const cached = this.creatorsMap.get(key);
    if (cached) return cached;

    try {
      const name = await apiGetCreatorName(service, creatorId);
      if (name) {
        this.creatorsMap.set(key, name);
        return name;
      }
    } catch {}
    return undefined;
  }

  setName(service: string, creatorId: string, name: string) {
    if (!service || !creatorId || !name) return;
    this.creatorsMap.set(`${service.toLowerCase()}:${creatorId.toLowerCase()}`, name);
  }

  private buildQueryParams(offset = 0, limit = 80): CreatorsQueryParams {
    const enabledConfigs = providerState.providers.filter((p) => p.enabled);
    const enabledIds = enabledConfigs.map((p) => p.id);

    const activeProviderEntries = Object.entries(this.providerFilters).filter(([, state]) => state !== 'neutral');
    let candidateProviders = [...enabledIds];
    if (activeProviderEntries.length > 0) {
      const included = activeProviderEntries.filter(([, state]) => state === 'include').map(([id]) => id);
      const excluded = activeProviderEntries.filter(([, state]) => state === 'exclude').map(([id]) => id);

      if (included.length > 0) {
        candidateProviders = candidateProviders.filter((id) => included.includes(id));
      }
      if (excluded.length > 0) {
        candidateProviders = candidateProviders.filter((id) => !excluded.includes(id));
      }
    }
    const effectiveProviders = candidateProviders.length === 0 ? ['__none__'] : candidateProviders;

    const allServices = this.services.length > 0 ? this.services : this.enabledServices;
    let candidateServices = allServices.filter((s) => {
      const provs = providerState.getProvidersForService(s);
      if (provs.length === 0) return true;
      return provs.some((p) => effectiveProviders.includes(p.id));
    });

    const includedServices: string[] = [];
    const excludedServices: string[] = [];
    for (const [srv, state] of Object.entries(this.serviceFilters)) {
      if (state === 'include') includedServices.push(srv);
      else if (state === 'exclude') excludedServices.push(srv);
    }

    let finalServices: string[];
    if (includedServices.length > 0) {
      finalServices = candidateServices.filter((s) => includedServices.includes(s));
    } else if (excludedServices.length > 0) {
      finalServices = candidateServices.filter((s) => !excludedServices.includes(s));
    } else {
      finalServices = candidateServices;
    }

    const effectiveServices = finalServices.length === 0
      ? ['__none__']
      : finalServices;

    return {
      query: this.searchQuery.trim() || undefined,
      services: effectiveServices,
      providers: effectiveProviders,
      sort_by: this.sortBy,
      sort_order: this.sortOrder,
      subscribed_only: this.activeTab === 'subscribed',
      hide_ai: configState.settings.pawchive_hide_ai || this.aiFilter === 'exclude',
      limit,
      offset
    };
  }

  async loadPage(reset = false) {
    if (providerState.providers.length === 0) {
      await providerState.loadProviders();
    }
    if (this.services.length === 0) {
      await this.loadServices();
    }

    const currentId = ++this._queryId;
    if (reset) {
      this.creators = [];
    }
    this.loading = true;
    this.error = null;

    try {
      const params = this.buildQueryParams(0, 80);
      const res = await apiListCreatorsPage(params);
      if (currentId !== this._queryId) return;

      this.creators = res.items;
      this.totalCount = res.total;
      this.hasMore = res.has_more;
      this.loaded = true;

      for (const c of res.items) {
        this.creatorsMap.set(`${c.service.toLowerCase()}:${c.id.toLowerCase()}`, c.name);
      }
    } catch (e) {
      if (currentId !== this._queryId) return;
      this.error = e instanceof Error ? e.message : String(e);
      logger.error('[Creators] Failed to load creators page', e);
    } finally {
      if (currentId === this._queryId) {
        this.loading = false;
      }
    }
  }

  scheduleSearch(delay = 250) {
    if (this._debounceTimer) clearTimeout(this._debounceTimer);
    this._debounceTimer = setTimeout(() => {
      void this.loadPage(true);
    }, delay);
  }

  async loadMore() {
    if (this.loadingMore || !this.hasMore || this.loading) return;
    this.loadingMore = true;

    try {
      if (providerState.providers.length === 0) {
        await providerState.loadProviders();
      }
      if (this.services.length === 0) {
        await this.loadServices();
      }
      const params = this.buildQueryParams(this.creators.length, 80);
      const res = await apiListCreatorsPage(params);

      this.creators = [...this.creators, ...res.items];
      this.totalCount = res.total;
      this.hasMore = res.has_more;

      for (const c of res.items) {
        this.creatorsMap.set(`${c.service.toLowerCase()}:${c.id.toLowerCase()}`, c.name);
      }
    } catch (e) {
      logger.error('[Creators] Failed to load more creators', e);
    } finally {
      this.loadingMore = false;
    }
  }

  async load(force = false) {
    if (!this._initialized) {
      await this.init();
      return;
    }
    if (force) {
      await this.refresh();
    } else if (!this.loaded) {
      await this.loadPage(false);
    }
  }

  async refresh() {
    this.syncing = true;
    try {
      const count = await apiSyncCreators();
      logger.info(`[Creators] Synced ${count} creators`);
      await this.loadServices();
      await this.loadNamesMap();
      await this.loadPage(true);
    } catch (e) {
      logger.error('[Creators] Failed to sync creators', e);
    } finally {
      this.syncing = false;
    }
  }
}

export const creatorsState = new CreatorsState();
