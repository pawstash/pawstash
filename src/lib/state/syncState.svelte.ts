import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { SyncDevice, SyncPhase, SyncProgress, SyncStatus } from '$lib/types/sync';
import { libraryState } from './libraryState.svelte';
import { subscriptionState } from './subscriptionState.svelte';
import { accountState } from './accountState.svelte';
import { notify } from '$lib/utils/toast';
import { i18n } from '$lib/i18n';
import {
  apiConnectSyncAccount, apiCreateSyncAccount, apiDisconnectSync, apiGetSyncStatus,
  apiChangeSyncPassword, apiCopySyncRecoveryKit, apiGetSyncRecoveryKit, apiListSyncDevices, apiLockSync,
  apiRecoverSyncAccount, apiResolveSyncConflict, apiRevokeSyncDevice, apiRunSync,
  apiSetSyncEnabled, apiUnlockSync
} from '$lib/utils/ipc';

class SyncState {
  status = $state<SyncStatus>({ configured: false, enabled: false, unlocked: false, syncing: false, revision: 0, cursor: 0, conflict: false });
  busy = $state(false);
  phase = $state<SyncPhase>('idle');
  progress = $state<number | null>(null);
  progressMessage = $state<string | null>(null);
  currentCount = $state<number | null>(null);
  totalCount = $state<number | null>(null);
  devices = $state<SyncDevice[]>([]);

  get isSyncing(): boolean {
    if (this.phase !== 'idle') return true;
    if (this.busy && this.status.configured && this.status.unlocked) return true;
    return false;
  }

  private initialized = false;
  private unlistenStatus?: UnlistenFn;
  private unlistenProgress?: UnlistenFn;

  async init() {
    if (this.initialized) return;
    this.initialized = true;

    try {
      this.unlistenStatus = await listen<SyncStatus>('sync-status-updated', ({ payload }) => {
        const prevError = this.status.last_error;
        this.status = payload;
        if (!payload.syncing) {
          this.phase = 'idle';
          this.progress = null;
          this.progressMessage = null;
        }
        if (payload.last_error && payload.last_error !== prevError) {
          notify.error(i18n.t('sync.sync_failed') || 'Sync failed', payload.last_error, {
            action: {
              label: i18n.t('common.copy') || 'Copy',
              onclick: () => {
                void navigator.clipboard?.writeText(payload.last_error || '');
                notify.success(i18n.t('common.copied') || 'Copied');
              }
            }
          });
        }
        void this.refreshLibrary();
      });

      this.unlistenProgress = await listen<SyncProgress>('sync-progress', ({ payload }) => {
        this.phase = payload.phase;
        this.progress = payload.progress ?? null;
        this.progressMessage = payload.message ?? null;
        this.currentCount = payload.current ?? null;
        this.totalCount = payload.total ?? null;

        if (payload.phase === 'idle') {
          this.progress = null;
          this.progressMessage = null;
          this.currentCount = null;
          this.totalCount = null;
          this.status.syncing = false;
        }
      });

      await this.refresh();
    } catch (e) {
      console.warn('Failed to initialize sync state:', e);
    }
  }

  async refresh() {
    try {
      const st = await apiGetSyncStatus();
      this.status = st;
      if (!st.syncing) {
        this.phase = 'idle';
        this.progress = null;
      }
    } catch (e) {
      console.warn('Failed to refresh sync status:', e);
    }
  }

  async setEnabled(enabled: boolean) {
    return this.work(() => apiSetSyncEnabled(enabled));
  }

  async create(server: string, account: string, password: string, device: string) {
    return this.work(() => apiCreateSyncAccount(server, account, password, device), true);
  }

  async connect(server: string, account: string, password: string, device: string) {
    return this.work(() => apiConnectSyncAccount(server, account, password, device), true);
  }

  async unlock(password: string) {
    return this.work(() => apiUnlockSync(password));
  }

  async lock() {
    return this.work(apiLockSync);
  }

  async disconnect() {
    return this.work(apiDisconnectSync, true);
  }

  async changePassword(currentPassword: string, newPassword: string) {
    return this.work(() => apiChangeSyncPassword(currentPassword, newPassword));
  }

  async getRecoveryKit() {
    this.busy = true;
    try {
      return await apiGetSyncRecoveryKit();
    } finally {
      this.busy = false;
    }
  }

  async copyRecoveryKit() {
    this.busy = true;
    try {
      return await apiCopySyncRecoveryKit();
    } finally {
      this.busy = false;
    }
  }

  async recover(recoveryKit: string, newPassword: string, device: string) {
    return this.work(() => apiRecoverSyncAccount(recoveryKit, newPassword, device), true);
  }

  async loadDevices() {
    this.busy = true;
    try {
      this.devices = await apiListSyncDevices();
      return this.devices;
    } finally {
      this.busy = false;
    }
  }

  async revokeDevice(deviceId: string) {
    this.busy = true;
    try {
      this.devices = await apiRevokeSyncDevice(deviceId);
      return this.devices;
    } finally {
      this.busy = false;
    }
  }

  async sync() {
    this.phase = 'connecting';
    this.progress = null;
    return this.work(apiRunSync, true);
  }

  async resolve(resolution: 'local' | 'remote') {
    return this.work(() => apiResolveSyncConflict(resolution), true);
  }

  private async work(call: () => Promise<SyncStatus>, refreshLibrary = false) {
    this.busy = true;
    try {
      this.status = await call();
      if (refreshLibrary) await this.refreshLibrary();
      return this.status;
    } finally {
      this.busy = false;
      this.phase = 'idle';
      this.progress = null;
      this.status.syncing = false;
    }
  }

  private async refreshLibrary() {
    try {
      await Promise.allSettled([
        libraryState.refreshCollections(),
        libraryState.refreshSavedKeys(),
        libraryState.refresh(),
        subscriptionState.reload(),
        accountState.refresh(),
        accountState.fetchFavorites('post', true),
        accountState.fetchFavorites('creator', true)
      ]);
    } catch (e) {
      console.warn('Failed to refresh library after sync:', e);
    }
  }
}

export const syncState = new SyncState();
