import type { Post } from '$lib/types/content';
import { i18n } from '$lib/i18n';
import { libraryState } from '$lib/state/libraryState.svelte';
import { navigationState } from '$lib/state/navigationState.svelte';
import { logger } from '$lib/utils/logger';
import { notify } from '$lib/utils/toast';

function stash(stashId: string) {
  return libraryState.collections.find((collection) => collection.id === stashId) ?? null;
}

function describe(stashId: string) {
  const collection = stash(stashId);
  return {
    name: collection ? libraryState.getStashDisplayName(collection) : undefined,
    swatch: collection?.color || undefined
  };
}

export function notifyAddedToStash(stashId: string, description?: string) {
  const { name, swatch } = describe(stashId);
  notify.success(i18n.t('library.added_to_stash'), {
    description: description ?? name,
    swatch,
    action: {
      label: i18n.t('library.open_stash_action'),
      onclick: () => {
        void libraryState.selectCollection(stashId);
        navigationState.navigateRoot('library');
      }
    }
  });
}

export function notifyRemovedFromStash(stashId: string, posts: Post[]) {
  const { name, swatch } = describe(stashId);
  notify.success(i18n.t('library.removed_from_stash'), {
    description: name,
    swatch,
    glyph: 'removed',
    action: {
      label: i18n.t('library.undo'),
      onclick: () => {
        void (async () => {
          try {
            for (const post of posts) {
              await libraryState.save(post, stashId);
            }
          } catch (error) {
            logger.error('Failed to undo stash removal', error);
            notify.error(i18n.t('library.save_error'), error);
          }
        })();
      }
    }
  });
}
