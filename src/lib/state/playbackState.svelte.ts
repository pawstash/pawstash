interface PlaybackEntry {
  time: number;
  duration: number;
  updatedAt: number;
}

const STORAGE_KEY = 'pawstash_playback_positions';
const MAX_AGE_MS = 30 * 24 * 60 * 60 * 1000; // 30 days
const MIN_SAVE_TIME = 5; // don't save first 5 seconds
const END_THRESHOLD_RATIO = 0.95; // consider finished if > 95%

class PlaybackState {
  private entries = $state<Record<string, PlaybackEntry>>({});
  private loaded = false;

  constructor() {
    this.load();
  }

  private load() {
    if (typeof localStorage === 'undefined' || this.loaded) return;
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw) as Record<string, PlaybackEntry>;
        const now = Date.now();
        const cleaned: Record<string, PlaybackEntry> = {};
        for (const [key, entry] of Object.entries(parsed)) {
          if (entry && entry.time > 0 && (now - (entry.updatedAt || 0) < MAX_AGE_MS)) {
            cleaned[key] = entry;
          }
        }
        this.entries = cleaned;
      }
    } catch (e) {
      console.warn('Failed to load playback state from localStorage:', e);
    } finally {
      this.loaded = true;
    }
  }

  private persist() {
    if (typeof localStorage === 'undefined') return;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.entries));
    } catch (e) {
      console.warn('Failed to save playback state to localStorage:', e);
    }
  }

  getTime(mediaKey?: string | null): number | undefined {
    if (!mediaKey) return undefined;
    this.load();
    const entry = this.entries[mediaKey];
    if (!entry) return undefined;
    if (entry.duration > 0 && entry.time >= entry.duration * END_THRESHOLD_RATIO) {
      return undefined;
    }
    return entry.time;
  }

  saveTime(mediaKey?: string | null, currentTime?: number, duration?: number) {
    if (!mediaKey || currentTime === undefined || currentTime < MIN_SAVE_TIME) return;
    
    // If video is almost finished (> 95% or within last 5 seconds), clear it
    if (duration && duration > 0 && (currentTime >= duration * END_THRESHOLD_RATIO || (duration - currentTime) <= 5)) {
      this.clearTime(mediaKey);
      return;
    }

    this.entries[mediaKey] = {
      time: Math.floor(currentTime),
      duration: duration ? Math.floor(duration) : 0,
      updatedAt: Date.now()
    };
    this.persist();
  }

  clearTime(mediaKey?: string | null) {
    if (!mediaKey || !this.entries[mediaKey]) return;
    delete this.entries[mediaKey];
    this.persist();
  }
}

export const playbackState = new PlaybackState();
