<script lang="ts">
  import { untrack } from 'svelte';
  import { mediaRegistry, type MediaElementKind } from '$lib/media/mediaRegistry';

  interface Props {
    mediaKey: string;
    kind: MediaElementKind;
    src: string;
    poster?: string;
    autoplay?: boolean;
    resume?: boolean;
    onmedia?: (element: HTMLMediaElement) => void;
    onloadedmetadata?: (event: Event) => void;
    ontimeupdate?: (event: Event) => void;
    onplaying?: (event: Event) => void;
    onplay?: (event: Event) => void;
    onpause?: (event: Event) => void;
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
    onmedia,
    onloadedmetadata,
    ontimeupdate,
    onplaying,
    onplay,
    onpause,
    onended,
    onerror
  }: Props = $props();

  let host = $state<HTMLDivElement>();
  let active = $state(false);
  let element = $state<HTMLMediaElement | null>(null);

  let ready = $derived(Boolean(mediaKey && src));

  $effect(() => {
    if (!ready || !host) return;

    const key = mediaKey;
    const node = host;
    const cleanup = mediaRegistry.mount(
      key,
      node,
      { kind, src: untrack(() => src) },
      {
        autoplay: untrack(() => autoplay),
        resume: untrack(() => resume),
        poster: untrack(() => poster),
        onActive: (value) => {
          active = value;
          element = value ? mediaRegistry.element(key) ?? null : null;
        }
      }
    );

    return () => {
      active = false;
      element = null;
      cleanup();
    };
  });

  $effect(() => {
    mediaRegistry.setPropSource(mediaKey, src);
  });

  $effect(() => {
    mediaRegistry.setPoster(mediaKey, poster);
  });

  $effect(() => {
    const current = element;
    if (!active || !current) return;
    untrack(() => onmedia?.(current));
  });

  $effect(() => {
    const current = element;
    if (!active || !current) return;

    const handlers: Array<[string, ((event: Event) => void) | undefined]> = [
      ['loadedmetadata', onloadedmetadata],
      ['timeupdate', ontimeupdate],
      ['playing', onplaying],
      ['play', onplay],
      ['pause', onpause],
      ['ended', onended],
      ['error', onerror]
    ];

    for (const [type, handler] of handlers) {
      if (handler) current.addEventListener(type, handler);
    }

    return () => {
      for (const [type, handler] of handlers) {
        if (handler) current.removeEventListener(type, handler);
      }
    };
  });
</script>

<div bind:this={host} class="media-surface"></div>

<style>
  .media-surface {
    display: contents;
  }
</style>
