import { invoke } from '@tauri-apps/api/core';
import type { ProviderConfig, ProviderHealth, PostRevisionData, ProviderCapabilities } from '$lib/types/provider';
import { apiGetProviderCapabilities, apiGetActiveCapabilities } from '$lib/utils/ipc';
import { logger } from '$lib/utils/logger';

class ProviderState {
  providers = $state<ProviderConfig[]>([]);
  healths = $state<Record<string, ProviderHealth>>({});
  postRevisions = $state<Record<string, PostRevisionData[]>>({});
  selectedProvider = $state<Record<string, string>>({}); // postKey -> providerId or 'auto'
  selectedRevision = $state<Record<string, number>>({}); // postKey -> revision_id
  capabilities = $state<Record<string, ProviderCapabilities>>({});
  activeCapabilities = $state<ProviderCapabilities | null>(null);
  loading = $state(false);

  async loadCapabilities(): Promise<void> {
    try {
      const [allCaps, active] = await Promise.all([
        apiGetProviderCapabilities(),
        apiGetActiveCapabilities()
      ]);
      this.capabilities = allCaps;
      this.activeCapabilities = active;
    } catch (e) {
      logger.error('Failed to load provider capabilities', e);
    }
  }

  async loadProviders(): Promise<ProviderConfig[]> {
    this.loading = true;
    try {
      const list = await invoke<ProviderConfig[]>('list_providers');
      this.providers = list;
      await this.loadCapabilities();
      return list;
    } catch (e) {
      logger.error('Failed to load providers', e);
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
      await this.loadCapabilities();
    } catch (e) {
      logger.error('Failed to save providers', e);
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

  getProvidersForService(service?: string): ProviderConfig[] {
    if (!service) return this.providers.filter((p) => p.enabled);
    const s = service.toLowerCase();
    return this.providers
      .filter((p) => p.enabled && (p.services.length === 0 || p.services.some((srv) => srv.toLowerCase() === s)))
      .sort((a, b) => a.priority - b.priority);
  }

  getProviderForService(service?: string): ProviderConfig | undefined {
    return this.getProvidersForService(service)[0];
  }

  isServiceEnabled(service?: string): boolean {
    return this.getProvidersForService(service).length > 0;
  }

  getProviderIdForService(service?: string): string {
    return this.getProviderForService(service)?.id || '';
  }

  getProviderById(id: string): ProviderConfig | undefined {
    return this.providers.find((p) => p.id === id);
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
      logger.warn('Failed to load post revisions', e);
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
