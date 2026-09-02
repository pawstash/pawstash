import { listen } from '@tauri-apps/api/event';
import { apiResolveDeepLink } from '$lib/utils/ipc';
import { navigationState } from '$lib/state/navigationState.svelte';
import { feedState } from '$lib/state/feedState.svelte';
import { notify } from '$lib/utils/toast';
import { logger } from '$lib/utils/logger';

export async function handleDeepLinkUrl(rawUrl: string): Promise<boolean> {
  const url = rawUrl.trim();
  if (!url) return false;

  try {
    const target = await apiResolveDeepLink(url);
    logger.info('[DeepLink] Resolved target:', target);

    if (target.type === 'post') {
      const { service, creator_id, post_id } = target.payload;
      navigationState.openPost(service, creator_id, post_id);
      return true;
    } else if (target.type === 'creator') {
      const { service, creator_id } = target.payload;
      navigationState.openCreator(service, creator_id);
      return true;
    } else if (target.type === 'search') {
      const { query } = target.payload;
      if (query) {
        navigationState.navigateRoot('feed');
        feedState.searchQuery = query;
      }
      return true;
    }
  } catch (err: any) {
    logger.warn(`[DeepLink] Failed to resolve URL ${url}:`, err);
    notify.error(typeof err === 'string' ? err : err?.message || 'Failed to open link');
  }

  return false;
}

export function initDeepLinkListener(): () => void {
  const unlistenPromise = listen<string>('deep-link:opened', (event) => {
    if (event.payload) {
      void handleDeepLinkUrl(event.payload);
    }
  });

  return () => {
    void unlistenPromise.then((unlisten) => unlisten());
  };
}
