import { configState } from '$lib/state/configState.svelte';

const STORAGE_KEY = 'pawstash_media_dock';
export const DOCK_MIN_WIDTH = 260;
export const DOCK_MAX_WIDTH = 720;

export type DockSide = 'left' | 'right';

interface StoredDock {
  side?: DockSide;
  /** Fractions of the free height, so the dock keeps its vertical spot when the window resizes. */
  ny?: number;
  width?: number;
  preset?: string;
  /** Legacy corner-era field, used only to choose a side on migration. */
  nx?: number;
}

class MediaDockState {
  peeked = $state(false);
  dragging = $state(false);
  private side = $state<DockSide>('right');
  private ny = $state<number | null>(null);
  private customWidth = $state<number | null>(null);
  private customPreset = $state<string>('medium');

  constructor() {
    if (typeof localStorage === 'undefined') return;
    try {
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}') as StoredDock;
      if (stored.side === 'left' || stored.side === 'right') this.side = stored.side;
      else if (typeof stored.nx === 'number') this.side = stored.nx < 0.5 ? 'left' : 'right';
      if (typeof stored.ny === 'number') this.ny = Math.max(0, Math.min(1, stored.ny));
      if (typeof stored.width === 'number' && Number.isFinite(stored.width)) {
        this.customWidth = Math.max(DOCK_MIN_WIDTH, Math.min(DOCK_MAX_WIDTH, stored.width));
        this.customPreset = stored.preset || 'medium';
      }
    } catch {}
  }

  get preset(): string {
    return configState.settings.viewer_dock_size || 'medium';
  }

  /**
   * A resized dock keeps its width until the user picks a different preset; then the
   * preset wins again, so the Settings choice is never silently dead.
   */
  get sizePx(): number {
    if (this.customWidth !== null && this.customPreset === this.preset) return this.customWidth;
    return this.preset === 'small' ? 320 : this.preset === 'large' ? 560 : 420;
  }

  get autohide(): boolean {
    return configState.settings.viewer_dock_autohide ?? true;
  }

  get autohideDelayMs(): number {
    const seconds = configState.settings.viewer_dock_autohide_delay ?? 1;
    return Math.max(0, Math.min(15, seconds)) * 1000;
  }

  get peekPx(): number {
    const peek = configState.settings.viewer_dock_peek ?? 12;
    return Math.max(8, Math.min(40, peek));
  }

  get keepPlaying(): boolean {
    return configState.settings.viewer_dock_keep_playing ?? true;
  }

  /** The card is always attached to one side; only the vertical position is free. */
  positionFor(viewportW: number, viewportH: number, width: number, height: number, bottomInset: number, margin: number) {
    const usableWidth = Math.max(200, Math.min(width, viewportW - margin * 2));
    const x = this.side === 'left' ? margin : Math.max(margin, viewportW - usableWidth - margin);
    const freeY = Math.max(0, viewportH - height - margin * 2);
    const y = this.ny !== null ? margin + this.ny * freeY : viewportH - height - bottomInset;
    return { x: Math.round(x), y: Math.round(y) };
  }

  setPosition(x: number, y: number, viewportW: number, viewportH: number, width: number, height: number, _bottomInset: number, margin: number) {
    this.side = x + width / 2 < viewportW / 2 ? 'left' : 'right';
    const freeY = Math.max(1, viewportH - height - margin * 2);
    this.ny = Math.max(0, Math.min(1, (y - margin) / freeY));
    this.persist();
  }

  setWidth(width: number) {
    this.customWidth = Math.max(DOCK_MIN_WIDTH, Math.min(DOCK_MAX_WIDTH, Math.round(width)));
    this.customPreset = this.preset;
    this.persist();
  }

  /** Settings changed the preset: drop the manual width. */
  clearCustomWidth() {
    if (this.customWidth === null) return;
    this.customWidth = null;
    this.persist();
  }

  setPeeked(peeked: boolean) {
    this.peeked = Boolean(peeked) && this.autohide && this.peekPx > 0;
  }

  private persist() {
    if (typeof localStorage === 'undefined') return;
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          side: this.side,
          ny: this.ny ?? undefined,
          width: this.customWidth ?? undefined,
          preset: this.customPreset
        } satisfies StoredDock)
      );
    } catch {}
  }
}

export const mediaDockState = new MediaDockState();
