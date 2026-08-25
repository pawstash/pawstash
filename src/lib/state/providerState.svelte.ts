import { invoke } from '@tauri-apps/api/core';
import type { ProviderConfig, ProviderHealth, PostRevisionData } from '$lib/types/provider';

class ProviderState {
  providers = $state<ProviderConfig[]>([]);
  healths = $state<Record<string, ProviderHealth>>({});
  postRevisions = $state<Record<string, PostRevisionData[]>>({});
  selectedProvider = $state<Record<string, string>>({}); // postKey -> providerId or 'auto'
  selectedRevision = $state<Record<string, number>>({}); // postKey -> revision_id
  loading = $state(false);

  async loadProviders(): Promise<ProviderConfig[]> {
    this.loading = true;
    try {
      const list = await invoke<ProviderConfig[]>('list_providers');
      let changed = false;
      const sanitized = list.map((p) => {
        if (p.id === 'coomer' || p.id === 'onlyhaven') {
          const cleanFallbacks = p.fallback_urls.filter((u) => !u.includes('coomer'));
          if (cleanFallbacks.length !== p.fallback_urls.length || p.name !== 'OnlyHaven') {
            changed = true;
            return { ...p, name: 'OnlyHaven', fallback_urls: cleanFallbacks };
          }
        }
        return p;
      });
      this.providers = sanitized;
      if (changed) {
        void this.saveProviders(sanitized);
      }
      return sanitized;
    } catch (e) {
      console.error('Failed to load providers:', e);
      return [];
    } finally {
      this.loading = false;
    }
  }

  async saveProviders(list: ProviderConfig[]): Promise<void> {
    this.loading = true;
    try {
      await invoke('save_providers', { providers: list });
      this.providers = list;
    } catch (e) {
      console.error('Failed to save providers:', e);
      throw e;
    } finally {
      this.loading = false;
    }
  }

  async testProvider(providerId: string): Promise<ProviderHealth> {
    try {
      const health = await invoke<ProviderHealth>('test_provider_connection', { providerId });
      this.healths[providerId] = health;
      return health;
    } catch (e) {
      const fallback: ProviderHealth = {
        provider_id: providerId,
        active_endpoint: '',
        is_healthy: false,
        latency_ms: 0,
        error: String(e),
        last_checked_at: new Date().toISOString(),
      };
      this.healths[providerId] = fallback;
      return fallback;
    }
  }

  async testAllProviders(): Promise<void> {
    await Promise.all(
      this.providers
        .filter((p) => p.enabled)
        .map((p) => this.testProvider(p.id))
    );
  }

  async addProvider(config: ProviderConfig): Promise<void> {
    const list = [...this.providers, config];
    await this.saveProviders(list);
  }

  async updateProvider(config: ProviderConfig): Promise<void> {
    const list = this.providers.map((p) => (p.id === config.id ? config : p));
    await this.saveProviders(list);
  }

  async removeProvider(id: string): Promise<void> {
    const list = this.providers.filter((p) => p.id !== id);
    await this.saveProviders(list);
  }

  async movePriority(id: string, direction: 'up' | 'down'): Promise<void> {
    const idx = this.providers.findIndex((p) => p.id === id);
    if (idx === -1) return;
    const targetIdx = direction === 'up' ? idx - 1 : idx + 1;
    if (targetIdx < 0 || targetIdx >= this.providers.length) return;

    const list = [...this.providers];
    const [moved] = list.splice(idx, 1);
    list.splice(targetIdx, 0, moved);

    // Reassign priorities
    list.forEach((p, i) => {
      p.priority = i + 1;
    });

    await this.saveProviders(list);
  }

  getProvidersForService(service: string): ProviderConfig[] {
    const s = service.toLowerCase();
    const matches = this.providers
      .filter((p) => p.enabled && (p.services.length === 0 || p.services.some((srv) => srv.toLowerCase() === s)))
      .sort((a, b) => a.priority - b.priority);
    if (matches.length > 0) return matches;
    return this.providers.filter((p) => p.enabled).sort((a, b) => a.priority - b.priority);
  }

  async loadPostRevisions(service: string, creatorId: string, postId: string): Promise<PostRevisionData[]> {
    const key = this.getPostKey(service, creatorId, postId);
    try {
      const revs = await invoke<PostRevisionData[]>('fetch_post_revisions', {
        service,
        creatorId,
        postId,
      });
      this.postRevisions = { ...this.postRevisions, [key]: revs || [] };
      return revs || [];
    } catch (e) {
      console.warn('Failed to load post revisions:', e);
      return [];
    }
  }

  getPostKey(service: string, creatorId: string, postId: string): string {
    return `${service}:${creatorId}:${postId}`;
  }

  getSelectedProvider(service: string, creatorId: string, postId: string): string {
    const key = this.getPostKey(service, creatorId, postId);
    return this.selectedProvider[key] || 'auto';
  }

  setSelectedProvider(service: string, creatorId: string, postId: string, providerId: string): void {
    const key = this.getPostKey(service, creatorId, postId);
    this.selectedProvider = { ...this.selectedProvider, [key]: providerId };
  }

  getSelectedRevision(service: string, creatorId: string, postId: string): number | null {
    const key = this.getPostKey(service, creatorId, postId);
    return this.selectedRevision[key] ?? null;
  }

  setSelectedRevision(service: string, creatorId: string, postId: string, revId: number | null): void {
    const key = this.getPostKey(service, creatorId, postId);
    const copy = { ...this.selectedRevision };
    if (revId === null) {
      delete copy[key];
    } else {
      copy[key] = revId;
    }
    this.selectedRevision = copy;
  }
}

export const providerState = new ProviderState();
