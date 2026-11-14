<script lang="ts">
  import MediaSurface from '$lib/components/ui/MediaSurface.svelte';
  import PlayerControls from './PlayerControls.svelte';
  import { tooltip } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import IconFullscreen from '~icons/fluent/full-screen-maximize-24-regular';

  interface Props {
    mediaKey: string;
    kind: 'video' | 'audio';
    src: string;
    poster?: string;
    autoplay?: boolean;
    resume?: boolean;
    priority?: boolean;
    /** `overlay` fills a tile and floats the bar over the media; `block` stacks the bar under it. */
    variant?: 'overlay' | 'block';
    onopenviewer?: () => void;
    onmedia?: (element: HTMLMediaElement) => void;
    onloadedmetadata?: (event: Event) => void;
    onplaying?: (event: Event) => void;
    onplay?: (event: Event) => void;
    onpause?: (event: Event) => void;
    onwaiting?: (event: Event) => void;
    onstalled?: (event: Event) => void;
    oncanplay?: (event: Event) => void;
    onended?: (event: Event) => void;
    onerror?: (event: Event) => void;
  }

  let {
    mediaKey,
    kind,
    src,
    poster,
    autoplay = false,
    resume = true,
    priority = false,
    variant = 'overlay',
    onopenviewer,
    onmedia,
    onloadedmetadata,
    onplaying,
    onplay,
    onpause,
    onwaiting,
    onstalled,
    oncanplay,
    onended,
    onerror
  }: Props = $props();

  let media = $state<HTMLMediaElement | null>(null);

  function attach(element: HTMLMediaElement) {
    media = element;
    onmedia?.(element);
  }
</script>

<div class="inline-player" class:is-block={variant === 'block'} class:is-audio={kind === 'audio'}>
  <div class="inline-media" aria-hidden={kind === 'audio'}>
    <MediaSurface
      {mediaKey}
      {kind}
      {src}
      {poster}
      {autoplay}
      {resume}
      {priority}
      controls={false}
      onmedia={attach}
      onloadedmetadata={(event) => onloadedmetadata?.(event)}
      onplaying={(event) => onplaying?.(event)}
      onplay={(event) => onplay?.(event)}
      onpause={(event) => onpause?.(event)}
      onwaiting={(event) => onwaiting?.(event)}
      onstalled={(event) => onstalled?.(event)}
      oncanplay={(event) => oncanplay?.(event)}
      onended={(event) => onended?.(event)}
      onerror={(event) => onerror?.(event)}
    />
  </div>

  <div class="inline-controls">
    {#snippet inlineActions()}
      {#if onopenviewer}
        <button
          type="button"
          class="player-controls-btn"
          onclick={onopenviewer}
          use:tooltip={i18n.t('post.viewer_open')}
          aria-label={i18n.t('post.viewer_open')}
        >
          <IconFullscreen />
        </button>
      {/if}
    {/snippet}

    <PlayerControls {media} playable timePosition="row" compact actions={inlineActions} />
  </div>
</div>

<style>
  .inline-player {
    position: absolute;
    inset: 0;
    z-index: 3;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    min-width: 0;
    container-type: inline-size;
    pointer-events: none;
  }

  .inline-player.is-block {
    position: relative;
    inset: auto;
    display: block;
    width: 100%;
    pointer-events: auto;
  }

  .inline-media {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: #000;
  }

  .inline-player.is-block .inline-media {
    position: relative;
    inset: auto;
    width: 100%;
    aspect-ratio: 16 / 9;
    max-height: 520px;
  }

  .inline-player.is-audio .inline-media {
    display: none;
  }

  .inline-media :global(video) {
    width: 100%;
    height: 100%;
    min-height: 0;
    max-height: none;
    object-fit: contain;
    background: #000;
  }

  .inline-controls {
    position: relative;
    z-index: 4;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-4) var(--space-2) var(--space-2);
    background: linear-gradient(
      to top,
      color-mix(in srgb, var(--player-fade-color) 86%, transparent) 0%,
      color-mix(in srgb, var(--player-fade-color) 36%, transparent) 58%,
      transparent 100%
    );
    pointer-events: auto;
  }

  .inline-player.is-block .inline-controls {
    padding: var(--space-2);
    background: var(--surface-container);
  }

  .inline-player.is-audio .inline-controls {
    border-radius: var(--radius-md);
    background: var(--surface-container);
  }
</style>
