import { apiFetchCreators } from '$lib/utils/ipc';
import type { Creator } from '$lib/types/pawchive';
import type { FilterMap } from '$lib/types/filter';
import { matchesTriStateFilter } from '$lib/types/filter';

export class CreatorsState {
  creators = $state<Creator[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);
  loaded = $state(false);

  searchQuery = $state('');
  serviceFilters = $state<FilterMap>({});
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

    if (Object.keys(this.serviceFilters).length > 0) {
      result = result.filter((c) => matchesTriStateFilter([c.service], this.serviceFilters));
    }

    const query = this.searchQuery.trim().toLowerCase();
    if (query) {
      result = result.filter(
        (c) =>
          c.name.toLowerCase().includes(query) ||
          c.id.toLowerCase().includes(query)
      );
    }

    result = [...result].sort((a, b) => {
      let comparison = 0;
      if (this.sortBy === 'name') {
        comparison = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
      } else if (this.sortBy === 'updated') {
        const tA = a.updated ?? 0;
        const tB = b.updated ?? 0;
        comparison = tA - tB;
      } else if (this.sortBy === 'indexed') {
        const tA = a.indexed ?? 0;
        const tB = b.indexed ?? 0;
        comparison = tA - tB;
      } else if (this.sortBy === 'favorited') {
        const favA = Number(a.favorited ?? a.kemono_favorited ?? 0);
        const favB = Number(b.favorited ?? b.kemono_favorited ?? 0);
        comparison = favA - favB;
      }

      return this.sortOrder === 'asc' ? comparison : -comparison;
    });

    return result;
  });

  async load(force = false) {
    if (this.loading) return;
    if (this.loaded && !force) return;

    this.loading = true;
    this.error = null;
    try {
      this.creators = await apiFetchCreators();
      this.loaded = true;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  async refresh() {
    await this.load(true);
  }
}

export const creatorsState = new CreatorsState();
