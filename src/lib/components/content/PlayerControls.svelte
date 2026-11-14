<script lang="ts">
  import type { Snippet } from 'svelte';
  import PlayerSettings from './PlayerSettings.svelte';
  import { playerPrefs } from '$lib/state/playerPrefs.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { formatDuration } from '$lib/utils/formatters';
  import { tooltip } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import IconPlay from '~icons/fluent/play-24-filled';
  import IconPause from '~icons/fluent/pause-24-filled';
  import IconVolume from '~icons/fluent/speaker-2-24-regular';
  import IconMuted from '~icons/fluent/speaker-mute-24-regular';

  interface Props {
    media: HTMLMediaElement | null;
    playable?: boolean;
    /** `meta` puts the clock above the seek (viewer/dock); `row` keeps it in the control row (inline). */
    timePosition?: 'meta' | 'row';
    compact?: boolean;
    actions?: Snippet;
    onactivity?: () => void;
    onplaychange?: (playing: boolean) => void;
  }

  let {
    media,
    playable = true,
    timePosition = 'meta',
    compact = false,
    actions,
    onactivity,
    onplaychange
  }: Props = $props();

  let playing = $state(false);
  let muted = $state(false);
  let volume = $state(1);
  let currentTime = $state(0);
  let duration = $state(0);
  let bufferedEnd = $state(0);

  let playbackRate = $derived(playerPrefs.rate);
  let loopEnabled = $derived(playerPrefs.loop);
  let progress = $derived(duration > 0 ? Math.min(100, (currentTime / duration) * 100) : 0);
  let bufferedPct = $derived(duration > 0 ? Math.min(100, (bufferedEnd / duration) * 100) : 0);
  let volumePct = $derived(muted ? 0 : Math.round(volume * 100));
  let timeLabel = $derived(
    duration > 0
      ? `${formatDuration(currentTime) || '0:00'} / ${formatDuration(duration)}`
      : playable
        ? formatDuration(currentTime) || '0:00'
        : ''
  );

  let seekEl = $state<HTMLDivElement>();
  let volumeEl = $state<HTMLDivElement>();
  let seeking = false;
  let volumeDragging = false;
  let seekHoverPct = $state<number | null>(null);

  $effect(() => {
    const element = media;
    if (!element) return;

    playerPrefs.initFromSettings(configState.settings.viewer_playback_rate);
    playerPrefs.apply(element);

    const sync = () => {
      currentTime = element.currentTime;
      if (Number.isFinite(element.duration) && element.duration > 0) duration = element.duration;
      playing = !element.paused;
      muted = element.muted;
      volume = element.volume;
      const ranges = element.buffered;
      bufferedEnd = ranges.length > 0 ? ranges.end(ranges.length - 1) : 0;
    };
    const syncState = () => {
      sync();
      onplaychange?.(!element.paused);
    };

    sync();
    const passive = ['timeupdate', 'loadedmetadata', 'durationchange', 'progress', 'seeking', 'seeked', 'volumechange', 'ended'] as const;
    for (const type of passive) element.addEventListener(type, sync);
    element.addEventListener('play', syncState);
    element.addEventListener('pause', syncState);

    return () => {
      for (const type of passive) element.removeEventListener(type, sync);
      element.removeEventListener('play', syncState);
      element.removeEventListener('pause', syncState);
    };
  });

  function togglePlay() {
    if (!media) return;
    if (media.paused) void media.play().catch(() => {});
    else media.pause();
    onactivity?.();
  }

  function toggleMute() {
    if (!media) return;
    media.muted = !media.muted;
    if (!media.muted && media.volume === 0) media.volume = 0.5;
    onactivity?.();
  }

  function setPlaybackRate(rate: number) {
    playerPrefs.setRate(rate);
    if (media) media.playbackRate = playerPrefs.rate;
  }

  function toggleLoop() {
    playerPrefs.setLoop(!playerPrefs.loop);
    if (media) media.loop = playerPrefs.loop;
  }

  function seekTo(event: PointerEvent) {
    if (!media || !seekEl || duration <= 0) return;
    const rect = seekEl.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    currentTime = ratio * duration;
    media.currentTime = currentTime;
    seekHoverPct = ratio * 100;
  }

  function updateSeekHover(event: PointerEvent) {
    if (!seekEl || duration <= 0) return;
    const rect = seekEl.getBoundingClientRect();
    seekHoverPct = Math.max(0, Math.min(100, ((event.clientX - rect.left) / rect.width) * 100));
  }

  function onSeekDown(event: PointerEvent) {
    if (event.button !== 0 || duration <= 0) return;
    event.stopPropagation();
    event.preventDefault();
    seeking = true;
    seekEl?.setPointerCapture(event.pointerId);
    seekTo(event);
    onactivity?.();
  }

  function onSeekMove(event: PointerEvent) {
    updateSeekHover(event);
    if (!seeking) return;
    event.stopPropagation();
    seekTo(event);
    onactivity?.();
  }

  function onSeekUp(event: PointerEvent) {
    if (!seeking) return;
    seeking = false;
    try {
      seekEl?.releasePointerCapture(event.pointerId);
    } catch {}
  }

  function onSeekKeydown(event: KeyboardEvent) {
    if (!media) return;
    const max = Number.isFinite(media.duration) ? media.duration : duration;
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();
      event.stopPropagation();
      media.currentTime = Math.max(0, Math.min(max, media.currentTime + (event.key === 'ArrowLeft' ? -5 : 5)));
    } else if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault();
      event.stopPropagation();
      media.currentTime = event.key === 'Home' ? 0 : max;
    } else {
      return;
    }
    onactivity?.();
  }

  function seekVolumeTo(event: PointerEvent) {
    if (!media || !volumeEl) return;
    const rect = volumeEl.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    media.volume = ratio;
    media.muted = ratio === 0;
  }

  function onVolumeDown(event: PointerEvent) {
    if (event.button !== 0) return;
    event.stopPropagation();
    event.preventDefault();
    volumeDragging = true;
    volumeEl?.setPointerCapture(event.pointerId);
    seekVolumeTo(event);
    onactivity?.();
  }

  function onVolumeMove(event: PointerEvent) {
    if (!volumeDragging) return;
    event.stopPropagation();
    seekVolumeTo(event);
    onactivity?.();
  }

  function onVolumeUp(event: PointerEvent) {
    if (!volumeDragging) return;
    volumeDragging = false;
    try {
      volumeEl?.releasePointerCapture(event.pointerId);
    } catch {}
  }

  function onVolumeKeydown(event: KeyboardEvent) {
    if (!media) return;
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();
      event.stopPropagation();
      media.volume = Math.max(0, Math.min(1, media.volume + (event.key === 'ArrowLeft' ? -0.05 : 0.05)));
      media.muted = media.volume === 0;
    } else if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault();
      event.stopPropagation();
      media.volume = event.key === 'Home' ? 0 : 1;
      media.muted = media.volume === 0;
    } else {
      return;
    }
    onactivity?.();
  }

  /** Wheeling over the volume rail nudges the level instead of zooming the stage or peeking the dock. */
  function volumeWheel(node: HTMLElement) {
    const handler = (event: WheelEvent) => {
      if (!media) return;
      event.preventDefault();
      event.stopPropagation();
      const next = Math.max(0, Math.min(1, media.volume + (event.deltaY > 0 ? -0.05 : 0.05)));
      media.volume = next;
      media.muted = next === 0;
      onactivity?.();
    };
    node.addEventListener('wheel', handler, { passive: false });
    return { destroy: () => node.removeEventListener('wheel', handler) };
  }
</script>

<div
  class="player-controls"
  class:is-compact={compact}
>
  {#if playable && timePosition === 'meta'}
    <div class="player-controls-meta">
      <span class="player-controls-clock">{timeLabel}</span>
    </div>
  {/if}

  {#if playable}
    <div
      bind:this={seekEl}
      class="player-controls-seek"
      role="slider"
      aria-label={i18n.t('post.viewer_seek')}
      aria-valuemin="0"
      aria-valuemax={Math.round(duration)}
      aria-valuenow={Math.round(currentTime)}
      aria-valuetext={timeLabel}
      tabindex="0"
      onpointerdown={onSeekDown}
      onpointermove={onSeekMove}
      onpointerup={onSeekUp}
      onpointercancel={onSeekUp}
      onpointerleave={() => (seekHoverPct = null)}
      onkeydown={onSeekKeydown}
    >
      <span class="player-controls-buffer" style:width={`${bufferedPct}%`}></span>
      <span class="player-controls-played" style:width={`${progress}%`}></span>
      <span class="player-controls-thumb" style:left={`${progress}%`}></span>
      {#if seekHoverPct !== null && duration > 0}
        <span class="player-controls-bubble" style:left={`${seekHoverPct}%`}>{formatDuration((seekHoverPct / 100) * duration)}</span>
      {/if}
    </div>
  {/if}

  <div class="player-controls-row">
    {#if playable}
      <div class="player-controls-group">
        <button
          type="button"
          class="player-controls-btn"
          onclick={togglePlay}
          use:tooltip={i18n.t(playing ? 'post.pause' : 'post.play')}
          aria-label={i18n.t(playing ? 'post.pause' : 'post.play')}
        >
          {#if playing}<IconPause />{:else}<IconPlay />{/if}
        </button>
        <button
          type="button"
          class="player-controls-btn"
          onclick={toggleMute}
          use:tooltip={i18n.t(muted ? 'post.unmute' : 'post.mute')}
          aria-label={i18n.t(muted ? 'post.unmute' : 'post.mute')}
        >
          {#if muted}<IconMuted />{:else}<IconVolume />{/if}
        </button>
        <div
          bind:this={volumeEl}
          class="player-controls-volume"
          role="slider"
          aria-label={i18n.t('post.viewer_volume')}
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={volumePct}
          aria-valuetext={`${volumePct}%`}
          tabindex="0"
          onpointerdown={onVolumeDown}
          onpointermove={onVolumeMove}
          onpointerup={onVolumeUp}
          onpointercancel={onVolumeUp}
          onkeydown={onVolumeKeydown}
          use:volumeWheel
        >
          <span style:width={`${volumePct}%`}></span>
        </div>
      </div>
    {/if}

    {#if playable && timePosition === 'row'}
      <span class="player-controls-time">{timeLabel}</span>
    {/if}

    <span class="player-controls-spacer"></span>

    <div class="player-controls-group">
      {#if playable}
        <PlayerSettings {playbackRate} {loopEnabled} onrate={setPlaybackRate} onloop={toggleLoop} />
      {/if}
      {@render actions?.()}
    </div>
  </div>
</div>

<style>
  .player-controls {
    --pc-control-height: var(--control-height);
    --pc-icon-size: var(--control-icon-size);
    display: flex;
    flex-direction: column;
    gap: var(--pc-gap, var(--space-2));
    width: 100%;
    min-width: 0;
    container-type: inline-size;
  }

  .player-controls.is-compact {
    --pc-control-height: var(--control-height-sm);
    --pc-icon-size: var(--control-icon-base);
    --pc-gap: var(--space-1);
  }

  @media (pointer: coarse) {
    .player-controls.is-compact {
      --pc-control-height: var(--control-height-base);
      --pc-icon-size: var(--control-icon-md);
    }
  }

  .player-controls-meta {
    display: flex;
    align-items: baseline;
    justify-content: flex-end;
    min-width: 0;
  }

  .player-controls-clock {
    color: var(--player-ink);
    font-family: var(--font-sans);
    font-size: var(--type-title-md-size);
    line-height: var(--type-title-md-line);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
  }

  .player-controls-seek {
    position: relative;
    height: var(--space-4);
    cursor: pointer;
    touch-action: none;
  }

  .player-controls-seek::before {
    content: '';
    position: absolute;
    inset: 50% 0 auto;
    height: var(--player-seek-height);
    border-radius: var(--radius-full);
    background: var(--player-track);
    transform: translateY(-50%);
    transition: height var(--duration-fast) var(--ease-standard);
  }

  .player-controls-buffer,
  .player-controls-played {
    position: absolute;
    top: 50%;
    left: 0;
    height: var(--player-seek-height);
    border-radius: var(--radius-full);
    transform: translateY(-50%);
    transition: height var(--duration-fast) var(--ease-standard);
    pointer-events: none;
  }

  .player-controls-seek:hover::before,
  .player-controls-seek:focus-visible::before,
  .player-controls-seek:hover .player-controls-buffer,
  .player-controls-seek:focus-visible .player-controls-buffer,
  .player-controls-seek:hover .player-controls-played,
  .player-controls-seek:focus-visible .player-controls-played {
    height: var(--player-seek-height-active);
  }

  .player-controls-buffer {
    background: var(--player-track-buffer);
  }

  .player-controls-played {
    background: var(--accent-primary);
  }

  .player-controls-thumb {
    position: absolute;
    top: 50%;
    width: var(--space-3);
    height: var(--space-3);
    margin-left: calc(var(--space-3) / -2);
    border-radius: var(--radius-full);
    background: var(--accent-primary);
    transform: translateY(-50%);
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-standard);
    pointer-events: none;
  }

  .player-controls-seek:hover .player-controls-thumb,
  .player-controls-seek:focus-visible .player-controls-thumb {
    opacity: 1;
  }

  .player-controls-bubble {
    position: absolute;
    bottom: var(--space-4);
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--player-scrim-strong);
    color: var(--player-ink);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--type-body-sm-size);
    font-variant-numeric: tabular-nums;
    transform: translateX(-50%);
    pointer-events: none;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  }

  .player-controls-row {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
    min-height: var(--pc-control-height);
  }

  .player-controls-group {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
  }

  :global(.player-controls-btn) {
    display: grid;
    place-items: center;
    flex: none;
    width: var(--pc-control-height);
    height: var(--pc-control-height);
    padding: 0;
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--player-ink);
    cursor: default;
    transition: background-color var(--duration-fast) var(--ease-standard), color var(--duration-fast) var(--ease-standard);
  }

  :global(.player-controls-btn:hover) {
    background: var(--player-state-hover);
    color: var(--player-ink);
  }

  :global(.player-controls-btn:active) {
    background: var(--player-state-pressed);
  }

  :global(.player-controls-btn svg) {
    width: var(--pc-icon-size);
    height: var(--pc-icon-size);
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.6));
  }

  .player-controls :global(.btn.player-icon-btn) {
    width: var(--pc-control-height);
    height: var(--pc-control-height);
  }

  .player-controls :global(.btn.player-icon-btn svg) {
    width: var(--pc-icon-size);
    height: var(--pc-icon-size);
  }

  .player-controls-volume {
    position: relative;
    display: flex;
    align-items: center;
    flex: none;
    width: clamp(56px, 14vw, 96px);
    height: var(--pc-control-height);
    cursor: pointer;
    touch-action: none;
  }

  .player-controls.is-compact .player-controls-volume {
    width: clamp(48px, 12vw, 84px);
  }

  .player-controls-volume::before {
    content: '';
    width: 100%;
    height: var(--player-volume-height);
    border-radius: var(--radius-full);
    background: var(--player-track);
    transition: height var(--duration-fast) var(--ease-standard);
  }

  .player-controls-volume > span {
    position: absolute;
    top: 50%;
    left: 0;
    height: var(--player-volume-height);
    border-radius: var(--radius-full);
    background: var(--player-ink);
    transform: translateY(-50%);
    transition: height var(--duration-fast) var(--ease-standard);
    pointer-events: none;
  }

  .player-controls-volume:hover::before,
  .player-controls-volume:focus-visible::before,
  .player-controls-volume:hover > span,
  .player-controls-volume:focus-visible > span {
    height: var(--player-volume-height-active);
  }

  .player-controls-time {
    flex: none;
    margin-inline-start: var(--space-1);
    color: var(--player-ink);
    font-family: var(--font-sans);
    font-size: var(--type-body-sm-size);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
  }

  .player-controls-spacer {
    flex: 1 1 auto;
  }

  @container (max-width: 380px) {
    .player-controls-volume {
      display: none;
    }
  }

  @container (max-width: 340px) {
    .player-controls.is-compact .player-controls-time {
      display: none;
    }
  }
</style>
