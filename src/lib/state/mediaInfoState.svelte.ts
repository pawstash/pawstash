import type { MediaInfo } from '$lib/types/media';
import { apiProbeMediaInfo } from '$lib/utils/ipc';

const MAX_MEDIA_INFO_ENTRIES = 1000;

class MediaInfoState {
  private entries = $state<Record<string, MediaInfo | null>>({});
  private inflight = new Map<string, Promise<MediaInfo | null>>();

  private remember(key: string, value: MediaInfo | null) {
    if (!(key in this.entries) && Object.keys(this.entries).length >= MAX_MEDIA_INFO_ENTRIES) {
      const oldest = Object.keys(this.entries)[0];
      if (oldest) {
        const next = { ...this.entries };
        delete next[oldest];
        this.entries = next;
      }
    }
    this.entries = { ...this.entries, [key]: value };
  }

  info(key?: string | null): MediaInfo | null | undefined {
    if (!key) return undefined;
    return this.entries[key];
  }

  ensure(key?: string | null, url?: string | null) {
    if (!key || !url || key in this.entries || this.inflight.has(key)) return;
    if (!/^https?:\/\//i.test(url)) return;

    const pending = apiProbeMediaInfo(url)
      .then((info) => {
        const value = info ?? null;
        this.remember(key, value);
        return value;
      })
      .catch(() => {
        this.remember(key, null);
        return null;
      })
      .finally(() => {
        this.inflight.delete(key);
      });
    this.inflight.set(key, pending);
  }
}

export const mediaInfoState = new MediaInfoState();
