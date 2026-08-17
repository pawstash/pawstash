import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CreatorSubscription, SubscriptionInput } from '$lib/types/subscription';
import {
  apiDeleteSubscription,
  apiListSubscriptions,
  apiRefreshSubscription,
  apiSetSubscriptionEnabled,
  apiUpsertSubscription
} from '$lib/utils/ipc';

class SubscriptionState {
  items = $state<CreatorSubscription[]>([]);
  loading = $state(false);
  private initialized = false;
  private unlisten?: UnlistenFn;

  async init() {
    if (this.initialized) return;
    this.initialized = true;
    this.unlisten = await listen<CreatorSubscription>('subscription-updated', ({ payload }) => this.upsert(payload));
    await this.reload();
  }

  async reload() {
    this.loading = true;
    try { this.items = await apiListSubscriptions(); }
    finally { this.loading = false; }
  }

  forCreator(service: string, creatorId: string) {
    return this.items.find((item) => item.service === service && item.creator_id === creatorId);
  }

  async save(input: SubscriptionInput) {
    const item = await apiUpsertSubscription(input);
    this.upsert(item);
    return item;
  }

  async setEnabled(id: string, enabled: boolean) { this.upsert(await apiSetSubscriptionEnabled(id, enabled)); }
  async refresh(id: string) { this.upsert(await apiRefreshSubscription(id)); }
  async remove(id: string) {
    if (await apiDeleteSubscription(id)) this.items = this.items.filter((item) => item.id !== id);
  }

  private upsert(item: CreatorSubscription) {
    const index = this.items.findIndex((current) => current.id === item.id);
    if (index < 0) this.items = [...this.items, item];
    else this.items = this.items.map((current, i) => i === index ? item : current);
  }
}

export const subscriptionState = new SubscriptionState();
