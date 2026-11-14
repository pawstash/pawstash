import { configState } from '$lib/state/configState.svelte';

export const PLAYBACK_RATES = [0.5, 0.75, 1, 1.25, 1.5, 2] as const;

/**
 * Playback preferences outlive both media surfaces: docking or expanding must not
 * reset the speed or the loop the user picked in the other one.
 */
class PlayerPrefs {
  rate = $state(1);
  loop = $state(false);
  private touched = false;

  initFromSettings(rate: number | undefined) {
    if (this.touched) return;
    const next = Number(rate);
    if (Number.isFinite(next) && next > 0) this.rate = next;
  }

  setRate(rate: number) {
    this.touched = true;
    this.rate = Math.max(0.25, Math.min(3, rate));
    if (Number(configState.settings.viewer_playback_rate) !== this.rate) {
      configState.settings.viewer_playback_rate = this.rate;
      void configState.save(configState.settings).catch(() => {});
    }
  }

  setLoop(loop: boolean) {
    this.loop = loop;
  }

  apply(element: HTMLMediaElement) {
    element.playbackRate = this.rate;
    element.loop = this.loop;
  }
}

export const playerPrefs = new PlayerPrefs();

export function defaultPlaybackRate(): number {
  return Number(configState.settings.viewer_playback_rate) || 1;
}
