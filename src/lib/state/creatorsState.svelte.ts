import { apiFetchCreators } from '$lib/utils/ipc';
import { logger } from '$lib/utils/logger';
import type { Creator } from '$lib/types/content';
import type { FilterMap, TriStateFilter } from '$lib/types/filter';
import { matchesTriStateFilter } from '$lib/types/filter';
import { configState } from './configState.svelte';
import { providerState } from './providerState.svelte';

const collator = new Intl.Collator(undefined, { sensitivity: 'base', numeric: true });

function parseTimestamp(value: unknown): number {
  if (!value) return 0;
  const numeric = Number(value);
  if (Number.isFinite(numeric) && numeric > 0) {
    return numeric > 10_000_000_000 ? Math.round(numeric / 1000) : numeric;
  }
  const ms = new Date(String(value)).getTime();
  return Number.isNaN(ms) ? 0 : Math.round(ms / 1000);
}

function parseFavoriteCount(c: Creator): number {
  const val = c.favorited ?? (c.extra as any)?.favorited ?? (c as any).kemono_favorited ?? (c.extra as any)?.kemono_favorited ?? (c as any).favorite_count ?? 0;
  const num = Number(val);
  return Number.isFinite(num) ? num : 0;
}

export class CreatorsState {
  creators = $state<Creator[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);
  loaded = $state(false);

  searchQuery = $state('');
  providerFilters = $state<FilterMap>({});
  serviceFilters = $state<FilterMap>({});
  aiFilter = $state<TriStateFilter>('neutral');
  sortBy = $state<'name' | 'updated' | 'indexed' | 'favorited'>('favorited');
  sortOrder = $state<'asc' | 'desc'>('desc');
  activeTab = $state<'all' | 'subscribed'>('all');

  services = $derived.by(() => {
    const list = new Set(this.creators.map((c) => c.service));
    return [...list].sort();
  });

  creatorsMap = $derived.by(() => {
    const map = new Map<string, string>();
    for (const c of this.creators) {
      map.set(`${c.service.toLowerCase()}:${c.id.toLowerCase()}`, c.name);
    }
    return map;
  });

  filteredCreators = $derived.by(() => {
    let result = this.creators;

    if (Object.keys(this.providerFilters).length > 0) {
      result = result.filter((c) => {
        const cProvider = (c.extra as any)?.provider_id || providerState.getProviderIdForService(c.service);
        return matchesTriStateFilter([cProvider], this.providerFilters);
      });
    }

    if (Object.keys(this.serviceFilters).length > 0) {
      result = result.filter((c) => matchesTriStateFilter([c.service], this.serviceFilters));
    }

    if (configState.settings.pawchive_hide_ai || this.aiFilter !== 'neutral') {
      result = result.filter((c) => {
        const isAi = Boolean(
          (c.extra as any)?.tags?.some((t: string) => t.toLowerCase() === 'ai' || t.toLowerCase().includes('ai generated')) ||
          c.name.toLowerCase().includes('[ai]') ||
          c.name.toLowerCase().includes('(ai)')
        );
        if (configState.settings.pawchive_hide_ai || this.aiFilter === 'exclude') {
          return !isAi;
        } else if (this.aiFilter === 'include') {
          return isAi;
        }
        return true;
      });
    }

    const query = this.searchQuery.trim().toLowerCase();
    if (query) {
      result = result.filter(
        (c) =>
          c.name.toLowerCase().includes(query) ||
          c.id.toLowerCase().includes(query)
      );
    }

    const sortBy = this.sortBy;
    const sortOrder = this.sortOrder;

    result = [...result].sort((a, b) => {
      let comparison = 0;
      if (sortBy === 'name') {
        comparison = collator.compare(a.name || '', b.name || '');
      } else if (sortBy === 'updated') {
        const tA = parseTimestamp(a.updated || a.indexed);
        const tB = parseTimestamp(b.updated || b.indexed);
        comparison = tA - tB;
        if (comparison === 0) {
          comparison = collator.compare(a.name || '', b.name || '');
        }
      } else if (sortBy === 'indexed') {
        const tA = parseTimestamp(a.indexed || a.updated);
        const tB = parseTimestamp(b.indexed || b.updated);
        comparison = tA - tB;
        if (comparison === 0) {
          comparison = collator.compare(a.name || '', b.name || '');
        }
      } else if (sortBy === 'favorited') {
        const favA = parseFavoriteCount(a);
        const favB = parseFavoriteCount(b);
        comparison = favA - favB;
        if (comparison === 0) {
          comparison = parseTimestamp(a.updated || a.indexed) - parseTimestamp(b.updated || b.indexed);
          if (comparison === 0) {
            comparison = collator.compare(a.name || '', b.name || '');
          }
        }
      }

      return sortOrder === 'asc' ? comparison : -comparison;
    });

    return result;
  });

  private _loadId = 0;

  async load(force = false) {
    if (this.loading && !force) return;
    if (this.loaded && !force) return;

    const currentId = ++this._loadId;
    this.loading = true;
    this.error = null;
    try {
      const list = await apiFetchCreators();
      if (currentId !== this._loadId) return;
      this.creators = list;
      this.loaded = true;
      logger.info(`[Creators] Loaded ${this.creators.length} creators`);
    } catch (e) {
      if (currentId !== this._loadId) return;
      this.error = e instanceof Error ? e.message : String(e);
      logger.error('[Creators] Failed to load creators', e);
    } finally {
      if (currentId === this._loadId) {
        this.loading = false;
      }
    }
  }

  async refresh() {
    await this.load(true);
  }
}

export const creatorsState = new CreatorsState();
