<script lang="ts">
  import type { ArchiveOpenResult, CloudNode } from '$lib/types/cloud';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import { getMediaThumbnail } from '$lib/utils/mediaThumbnail';
  import { serverPortState } from '$lib/state/serverPort.svelte';
  import { NODE_KIND_ICONS, nodeKindOf, type NodeKind } from '$lib/utils/cloudNodes';
  import { tooltip } from '$lib/motion';
  import IconFolderZip from '~icons/fluent/folder-zip-24-regular';
  import IconLock from '~icons/fluent/lock-closed-24-regular';
  import IconPlay from '~icons/fluent/play-24-filled';

  interface Props {
    name: string;
    label: string;
    load: () => Promise<ArchiveOpenResult>;
    onopen: () => void;
  }

  let { name, label, load, onopen }: Props = $props();

  const TILES = 3;
  const THUMB_WIDTH = 640;
  const MIN_COVER_BYTES = 20 * 1024;

  interface Tile {
    node: CloudNode;
    kind: NodeKind;
    meta: string;
  }

  let status = $state<'waiting' | 'ready' | 'failed'>('waiting');
  let result = $state<ArchiveOpenResult | null>(null);
  let thumbs = $state<Record<string, string>>({});

  let nodes = $derived(result?.folder?.nodes ?? []);
  let files = $derived(nodes.filter((node) => !node.is_folder));
  let locked = $derived(result?.status === 'password_required');
  let covers = $derived(
    files.filter((node) => {
      const kind = nodeKindOf(node);
      return kind === 'video' || (kind === 'image' && (node.size ?? 0) >= MIN_COVER_BYTES);
    })
  );
  let isMedia = $derived(covers.length > 0 && covers.length * 2 >= files.length);
  let folderStats = $derived.by(() => {
    const parents = new Map(nodes.map((node) => [node.id, node.parent_id ?? null]));
    const stats = new Map<string, { files: number; size: number }>();
    for (const file of files) {
      let parent = file.parent_id ?? null;
      while (parent) {
        const entry = stats.get(parent) ?? { files: 0, size: 0 };
        entry.files += 1;
        entry.size += file.size ?? 0;
        stats.set(parent, entry);
        parent = parents.get(parent) ?? null;
      }
    }
    return stats;
  });
  let level = $derived.by(() => {
    let parent: string | null = null;
    for (;;) {
      const children = nodes.filter((node) => (node.parent_id ?? null) === parent);
      if (children.length === 1 && children[0].is_folder) parent = children[0].id;
      else return children;
    }
  });
  let entries = $derived.by((): Tile[] => {
    if (isMedia) {
      const rest = files.filter((node) => !covers.includes(node)).sort((a, b) => (b.size ?? 0) - (a.size ?? 0));
      return [...covers, ...rest].map((node) => ({ node, kind: nodeKindOf(node), meta: node.size ? formatBytes(node.size) : '' }));
    }
    const ordered = [...level].sort((a, b) =>
      a.is_folder !== b.is_folder
        ? Number(b.is_folder) - Number(a.is_folder)
        : (folderStats.get(b.id)?.size ?? b.size ?? 0) - (folderStats.get(a.id)?.size ?? a.size ?? 0)
    );
    return ordered.map((node) => {
      const stats = folderStats.get(node.id);
      const meta = node.is_folder
        ? i18n.t('counts.files', { count: stats?.files ?? 0 })
        : node.size
          ? formatBytes(node.size)
          : '';
      return { node, kind: nodeKindOf(node), meta };
    });
  });
  let tiles = $derived(entries.slice(0, TILES));
  let remaining = $derived(entries.length - tiles.length);
  let kinds = $derived.by(() => {
    const counts = new Map<NodeKind, number>();
    for (const node of files) {
      const kind = nodeKindOf(node);
      counts.set(kind, (counts.get(kind) ?? 0) + 1);
    }
    return [...counts].sort((a, b) => b[1] - a[1]);
  });

  function entryUrl(node: CloudNode): string {
    const url = node.stream_url || node.download_url || '';
    return url.startsWith('/cloud_stream/') ? serverPortState.mediaUrl(url) : url;
  }

  async function loadThumbs(archiveId: string, tiles: Tile[]) {
    for (const { node, kind } of tiles) {
      if (!covers.includes(node)) continue;
      const url = await getMediaThumbnail(`archive:${archiveId}:${node.id}`, entryUrl(node), kind === 'video' ? 'video' : 'image', THUMB_WIDTH);
      if (url) thumbs = { ...thumbs, [node.id]: url };
    }
  }

  function whenVisible(element: HTMLElement) {
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        observer.disconnect();
        load()
          .then((value) => {
            result = value;
            status = value.folder?.nodes.some((entry) => !entry.is_folder) ? 'ready' : 'failed';
            if (status === 'ready' && !locked && isMedia) void loadThumbs(value.archive_id, tiles);
          })
          .catch(() => (status = 'failed'));
      },
      { rootMargin: '200px' }
    );
    observer.observe(element);
    return () => observer.disconnect();
  }
</script>

<button
  type="button"
  class="archive-preview"
  class:is-fallback={status === 'failed'}
  onclick={onopen}
  aria-label={`${i18n.t('cloud.archive_open')}: ${name}`}
  {@attach whenVisible}
>
  {#if status === 'failed'}
    <IconFolderZip class="archive-fallback-icon" />
    <span class="archive-fallback-label">{label}</span>
  {:else}
    {#if status === 'waiting'}
      <span class="archive-body is-waiting" aria-hidden="true"></span>
    {:else}
      <span class="archive-body archive-mosaic" data-count={tiles.length}>
        {#each tiles as { node, kind, meta }, index (node.id)}
          {@const Icon = NODE_KIND_ICONS[kind]}
          {@const thumb = thumbs[node.id]}
          <span class="archive-tile" class:has-thumb={Boolean(thumb)} use:tooltip={node.name}>
            {#if thumb}
              <img src={thumb} alt="" decoding="async" />
              <span class="grid-tile-shade"></span>
              {#if kind === 'video'}<span class="archive-play"><IconPlay /></span>{/if}
            {:else}
              <Icon class="archive-tile-icon" />
            {/if}
            <span class="archive-tile-text">
              <span class="archive-tile-name">{node.name}</span>
              {#if meta}<span class="archive-tile-size">{meta}</span>{/if}
            </span>
            {#if remaining > 0 && index === tiles.length - 1}
              <span class="archive-more">+{remaining}</span>
            {/if}
          </span>
        {/each}
      </span>
    {/if}
    <span class="archive-footer">
      {#if status === 'ready'}
        <span class="archive-summary">
          {#if locked}
            <span class="archive-lock" use:tooltip={i18n.t('cloud.archive_locked')}><IconLock /></span>
          {:else}
            <IconFolderZip />
          {/if}
          {i18n.t('counts.files', { count: files.length })}
        </span>
        {#if kinds.length > 1}
          <span class="archive-kinds">
            {#each kinds.slice(0, 3) as [kind, count] (kind)}
              {@const Icon = NODE_KIND_ICONS[kind]}
              <span class="archive-kind"><Icon />{count}</span>
            {/each}
          </span>
        {/if}
      {/if}
    </span>
  {/if}
</button>

<style>
  .archive-preview {
    display: flex;
    flex-direction: column;
    width: 100%;
    padding: 0;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--surface-container);
    color: inherit;
    font: inherit;
    text-align: start;
    overflow: hidden;
    cursor: pointer;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .archive-preview.is-fallback {
    align-items: center;
    justify-content: center;
    gap: 12px;
    aspect-ratio: 16 / 9;
    padding: 16px;
  }

  .archive-preview :global(.archive-fallback-icon) {
    width: 48px;
    height: 48px;
    color: var(--text-secondary);
  }

  .archive-fallback-label {
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
  }

  @media (hover: hover) {
    .archive-preview:hover {
      background: var(--surface-container-high);
    }
  }

  .archive-preview:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: 2px;
  }

  .archive-body {
    display: block;
    height: 208px;
  }

  .archive-body.is-waiting {
    background: var(--surface-container-high);
    animation: archive-wait 1.4s var(--ease-standard) infinite alternate;
  }

  @keyframes archive-wait {
    to {
      opacity: 0.55;
    }
  }

  .archive-mosaic {
    display: grid;
    grid-template-columns: 2fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 2px;
  }

  .archive-mosaic[data-count='1'] {
    grid-template-columns: 1fr;
    grid-template-rows: 1fr;
  }

  .archive-mosaic[data-count='2'] {
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr;
  }

  .archive-mosaic[data-count='3'] .archive-tile:first-child {
    grid-row: span 2;
  }

  .archive-tile {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-width: 0;
    min-height: 0;
    padding: 12px;
    overflow: hidden;
    background: var(--surface-container-high);
    text-align: center;
  }

  .archive-tile img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: archive-fade-in var(--duration-normal) var(--ease-standard);
  }

  @keyframes archive-fade-in {
    from {
      opacity: 0;
    }
  }

  .archive-tile :global(.archive-tile-icon) {
    flex: none;
    width: 36px;
    height: 36px;
    color: var(--text-secondary);
  }

  .archive-mosaic[data-count='3'] .archive-tile:not(:first-child) :global(.archive-tile-icon) {
    width: 24px;
    height: 24px;
  }

  .archive-tile-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    max-width: 100%;
  }

  .archive-tile-name {
    display: -webkit-box;
    overflow: hidden;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    color: var(--text-primary);
    font-size: var(--type-label-md-size);
    line-height: var(--type-label-md-line);
    overflow-wrap: anywhere;
  }

  .archive-tile-size {
    color: var(--text-secondary);
    font-size: var(--type-label-sm-size);
    line-height: var(--type-label-sm-line);
    font-variant-numeric: tabular-nums;
  }

  .archive-tile.has-thumb {
    justify-content: flex-end;
    align-items: stretch;
    text-align: start;
  }

  .archive-tile.has-thumb .archive-tile-text {
    position: relative;
    z-index: 3;
  }

  .archive-tile.has-thumb .archive-tile-name {
    -webkit-line-clamp: 1;
    line-clamp: 1;
    color: var(--on-media);
  }

  .archive-tile.has-thumb .archive-tile-size {
    color: var(--on-media-secondary);
  }

  .archive-mosaic[data-count='3'] .archive-tile.has-thumb:not(:first-child) .archive-tile-text {
    display: none;
  }

  .archive-play {
    position: absolute;
    top: 50%;
    left: 50%;
    z-index: 3;
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: var(--radius-full);
    background: var(--on-media-scrim-strong);
    color: var(--on-media);
    transform: translate(-50%, -50%);
  }

  .archive-play :global(svg) {
    width: 20px;
    height: 20px;
    margin-inline-start: 2px;
  }

  .archive-more {
    position: absolute;
    inset: 0;
    z-index: 4;
    display: grid;
    place-items: center;
    background: var(--on-media-scrim-strong);
    color: var(--on-media);
    font-size: var(--type-title-lg-size);
    line-height: var(--type-title-lg-line);
    font-variant-numeric: tabular-nums;
  }

  .archive-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 44px;
    padding: 10px 14px;
    color: var(--text-secondary);
    font-size: var(--type-label-lg-size);
    line-height: var(--type-label-lg-line);
    font-variant-numeric: tabular-nums;
  }

  .archive-summary,
  .archive-kinds,
  .archive-kind,
  .archive-lock {
    display: inline-flex;
    align-items: center;
  }

  .archive-summary {
    gap: 8px;
    color: var(--text-primary);
  }

  .archive-kinds {
    gap: 12px;
  }

  .archive-kind {
    gap: 4px;
  }

  .archive-footer :global(svg) {
    flex: none;
    width: 18px;
    height: 18px;
    color: var(--text-secondary);
  }

  @media (prefers-reduced-motion: reduce) {
    .archive-body.is-waiting,
    .archive-tile img {
      animation: none;
    }
  }
</style>
