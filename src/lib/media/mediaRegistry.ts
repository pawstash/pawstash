import { playbackState } from '$lib/state/playbackState.svelte';

export type MediaElementKind = 'video' | 'audio';

export interface MediaSource {
  kind: MediaElementKind;
  src: string;
}

interface HostRecord {
  node: HTMLElement;
  autoplay: boolean;
  resume: boolean;
  onActive?: (active: boolean) => void;
}

interface MediaEntry {
  key: string;
  element: HTMLMediaElement;
  kind: MediaElementKind;
  propSrc: string;
  src: string;
  hosts: HostRecord[];
  active: HostRecord | null;
  fresh: boolean;
  wasPlayingBeforeDetach: boolean;
  lastActiveAt: number;
  detachListeners: () => void;
}

const MAX_PARKED = 4;

function createElement(kind: MediaElementKind, src: string): HTMLMediaElement {
  const element = document.createElement(kind);
  element.controls = true;
  element.preload = 'auto';
  element.setAttribute('playsinline', '');
  element.src = src;
  return element;
}

// Moving the node keeps its decoder and buffer; recreating it refetches the media.
class MediaElementRegistry {
  private entries = new Map<string, MediaEntry>();
  private park: HTMLElement | null = null;

  acquire(key: string, source: MediaSource): MediaEntry | null {
    if (!key || !source.src) return null;
    let entry = this.entries.get(key);

    if (entry && entry.kind !== source.kind) {
      this.dispose(entry);
      entry = undefined;
    }

    if (!entry) {
      entry = this.create(key, source);
      this.entries.set(key, entry);
      return entry;
    }

    if (source.src !== entry.propSrc) {
      entry.propSrc = source.src;
      this.loadSource(entry, source.src);
    }

    return entry;
  }

  mount(
    key: string,
    node: HTMLElement,
    source: MediaSource,
    options: {
      autoplay?: boolean;
      resume?: boolean;
      poster?: string;
      onActive?: (active: boolean) => void;
    } = {}
  ): () => void {
    const entry = this.acquire(key, source);
    if (!entry) return () => {};

    if (options.poster && entry.element instanceof HTMLVideoElement) entry.element.poster = options.poster;

    const existing = entry.hosts.findIndex((host) => host.node === node);
    if (existing >= 0) entry.hosts.splice(existing, 1);

    const record: HostRecord = {
      node,
      autoplay: options.autoplay ?? false,
      resume: options.resume ?? true,
      onActive: options.onActive
    };
    entry.hosts.push(record);
    this.activate(entry, record);

    return () => this.unmount(key, node);
  }

  unmount(key: string, node: HTMLElement) {
    const entry = this.entries.get(key);
    if (!entry) return;

    const index = entry.hosts.findIndex((host) => host.node === node);
    if (index < 0) return;

    const record = entry.hosts[index];
    entry.hosts.splice(index, 1);
    if (entry.active !== record) return;

    entry.active = null;
    entry.wasPlayingBeforeDetach = !entry.element.paused;

    const next = [...entry.hosts].reverse().find((host) => host.node.isConnected);
    if (next) {
      this.activate(entry, next);
      return;
    }

    entry.element.pause();
    this.parkEntry(entry);
  }

  element(key: string): HTMLMediaElement | undefined {
    return this.entries.get(key)?.element;
  }

  pause(key: string) {
    this.entries.get(key)?.element.pause();
  }

  setPropSource(key: string, src: string) {
    const entry = this.entries.get(key);
    if (!entry || !src || src === entry.propSrc) return;
    entry.propSrc = src;
    this.loadSource(entry, src);
  }

  overrideElementSource(element: HTMLMediaElement, src: string) {
    if (!src) return;
    for (const entry of this.entries.values()) {
      if (entry.element === element) {
        entry.src = src;
        return;
      }
    }
  }

  setPoster(key: string, poster?: string) {
    const entry = this.entries.get(key);
    if (!entry || !poster || !(entry.element instanceof HTMLVideoElement)) return;
    if (entry.element.poster === poster) return;
    entry.element.poster = poster;
  }

  reset(key: string) {
    const entry = this.entries.get(key);
    if (!entry) return;
    this.dispose(entry);
  }

  private create(key: string, source: MediaSource): MediaEntry {
    const element = createElement(source.kind, source.src);
    const entry: MediaEntry = {
      key,
      element,
      kind: source.kind,
      propSrc: source.src,
      src: source.src,
      hosts: [],
      active: null,
      fresh: true,
      wasPlayingBeforeDetach: false,
      lastActiveAt: Date.now(),
      detachListeners: () => {}
    };

    const onTimeUpdate = () => {
      if (element.duration > 0) playbackState.saveTime(key, element.currentTime, element.duration);
    };
    const onLoadedMetadata = () => {
      playbackState.saveDuration(key, element.duration);
    };
    const onEnded = () => {
      playbackState.clearTime(key);
    };
    const onPlay = () => {
      this.pauseOthers(entry);
    };

    element.addEventListener('timeupdate', onTimeUpdate);
    element.addEventListener('loadedmetadata', onLoadedMetadata);
    element.addEventListener('ended', onEnded);
    element.addEventListener('play', onPlay);
    entry.detachListeners = () => {
      element.removeEventListener('timeupdate', onTimeUpdate);
      element.removeEventListener('loadedmetadata', onLoadedMetadata);
      element.removeEventListener('ended', onEnded);
      element.removeEventListener('play', onPlay);
    };

    return entry;
  }

  private activate(entry: MediaEntry, next: HostRecord) {
    const previous = entry.active;
    if (previous && previous !== next) previous.onActive?.(false);

    const element = entry.element;
    const wasPlaying = !element.paused;
    const wasParked = !element.isConnected || element.parentElement === this.park;
    const time = element.currentTime;

    entry.active = next;
    element.preload = 'auto';
    if (element.parentElement !== next.node) next.node.appendChild(element);

    if (time > 0 && Math.abs(element.currentTime - time) > 1) {
      element.currentTime = time;
    }

    if (wasPlaying) {
      if (element.paused) void element.play().catch(() => {});
    } else {
      const shouldResume = wasParked && next.resume && entry.wasPlayingBeforeDetach;
      if (entry.fresh ? next.autoplay : shouldResume) {
        void element.play().catch(() => {});
      }
    }

    entry.fresh = false;
    if (wasParked) entry.wasPlayingBeforeDetach = false;
    entry.lastActiveAt = Date.now();
    next.onActive?.(true);
  }

  private loadSource(entry: MediaEntry, src: string) {
    const element = entry.element;
    const wasPlaying = !element.paused;
    entry.src = src;
    element.src = src;
    element.load();
    if (!wasPlaying) return;
    element.addEventListener(
      'loadedmetadata',
      () => {
        void element.play().catch(() => {});
      },
      { once: true }
    );
  }

  private pauseOthers(entry: MediaEntry) {
    for (const other of this.entries.values()) {
      if (other === entry) continue;
      if (!other.element.paused) other.element.pause();
    }
  }

  private parkEntry(entry: MediaEntry) {
    entry.element.preload = 'none';
    this.ensurePark().appendChild(entry.element);
    entry.lastActiveAt = Date.now();
    this.evictParked(entry.key);
  }

  private evictParked(keepKey: string) {
    if (!this.park) return;
    const parked = [...this.entries.values()].filter(
      (entry) => entry.key !== keepKey && !entry.active && entry.element.parentElement === this.park
    );
    if (parked.length <= MAX_PARKED) return;

    parked.sort((a, b) => a.lastActiveAt - b.lastActiveAt);
    for (const entry of parked.slice(0, parked.length - MAX_PARKED)) this.dispose(entry);
  }

  private dispose(entry: MediaEntry) {
    entry.detachListeners();
    entry.element.pause();
    entry.element.removeAttribute('src');
    entry.element.load();
    entry.element.remove();
    this.entries.delete(entry.key);
  }

  private ensurePark(): HTMLElement {
    if (this.park?.isConnected) return this.park;

    const existing = document.getElementById('pawstash-media-park');
    if (existing) {
      this.park = existing;
      return existing;
    }

    const park = document.createElement('div');
    park.id = 'pawstash-media-park';
    park.setAttribute('aria-hidden', 'true');
    park.style.position = 'fixed';
    park.style.top = '0';
    park.style.left = '0';
    park.style.width = '1px';
    park.style.height = '1px';
    park.style.overflow = 'hidden';
    park.style.opacity = '0';
    park.style.pointerEvents = 'none';
    park.style.zIndex = '-1';
    document.body.appendChild(park);
    this.park = park;
    return park;
  }
}

export const mediaRegistry = new MediaElementRegistry();
