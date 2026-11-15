import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { apiClearAccountAvatar, apiGetAccountAvatar, apiSetAccountAvatar } from '$lib/utils/ipc';
import { fileToAvatarDataUrl } from '$lib/utils/avatar';

class AvatarState {
  dataUrl = $state<string | null>(null);
  busy = $state(false);

  private initialized = false;
  private unlisten?: UnlistenFn;

  async init() {
    if (this.initialized) return;
    this.initialized = true;
    try {
      this.unlisten = await listen('account-avatar-updated', () => {
        void this.refresh();
      });
      await this.refresh();
    } catch (error) {
      console.warn('Failed to initialize the profile avatar:', error);
    }
  }

  async refresh() {
    try {
      this.dataUrl = await apiGetAccountAvatar();
    } catch (error) {
      console.warn('Failed to load the profile avatar:', error);
    }
  }

  async setFromFile(file: File) {
    this.busy = true;
    try {
      const image = await fileToAvatarDataUrl(file);
      this.dataUrl = await apiSetAccountAvatar(image);
    } finally {
      this.busy = false;
    }
  }

  async clear() {
    this.busy = true;
    try {
      this.dataUrl = await apiClearAccountAvatar();
    } finally {
      this.busy = false;
    }
  }
}

export const avatarState = new AvatarState();
