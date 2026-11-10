<script lang="ts" module>
  import { i18n } from '$lib/i18n';
  import { formatBytes, formatPercent } from '$lib/utils/formatters';

  export function fileSummary(files: number, bytes: number): string {
    const count = i18n.t('counts.files', { count: files });
    return bytes > 0 ? `${count} · ${formatBytes(bytes)}` : count;
  }
</script>

<script lang="ts">
  import { untrack } from 'svelte';
  import type {
    ArchiveAttachment,
    ArchiveOpenResult,
    CloudFolderResult,
    CloudNode
  } from '$lib/types/cloud';
  import type { Post } from '$lib/types/content';
  import { isAutoRetryPending, type DownloadItem } from '$lib/types/download';
  import MediaViewer, {
    type MediaViewerItem,
    type MediaViewerKind
  } from '$lib/components/content/MediaViewer.svelte';
  import { resolveLocalMediaUrl, cloudProviderName } from '$lib/utils/media';
  import { NODE_KIND_ICONS, archiveFolderName, cloudNodePath, nodeKindOf, type NodeKind } from '$lib/utils/cloudNodes';
  import { apiShowInFolder, apiUnlockArchive, openDownloadedFile } from '$lib/utils/ipc';
  import { notify } from '$lib/utils/toast';
  import { logger } from '$lib/utils/logger';
  import { scrollable } from '$lib/actions/scrollable';
  import { createLazyImageLoader } from '$lib/actions/lazyImage';
  import { tooltip } from '$lib/motion';
  import { serverPortState } from '$lib/state/serverPort.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import ProgressRing from '$lib/components/ui/ProgressRing.svelte';
  import SecretUnlockPanel from './SecretUnlockPanel.svelte';
  import IconFolderOpen from '~icons/fluent/folder-open-24-regular';
  import IconZip from '~icons/fluent/folder-zip-24-regular';
  import IconCloud from '~icons/fluent/cloud-24-regular';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconCheck from '~icons/fluent/checkmark-circle-24-regular';
  import IconRetry from '~icons/fluent/arrow-clockwise-24-regular';
  import IconArrowUp from '~icons/fluent/arrow-up-24-regular';
  import IconChevronRight from '~icons/fluent/chevron-right-16-regular';
  import IconGrid from '~icons/fluent/grid-24-regular';
  import IconList from '~icons/fluent/list-24-regular';
  import IconLock from '~icons/fluent/lock-closed-24-regular';
  import IconKey from '~icons/fluent/key-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';

  interface Props {
    folder: CloudFolderResult | null;
    initialFolderId?: string | null;
    post?: Post | null;
    open: boolean;
    onclose: () => void;
    archive?: ArchiveOpenResult | null;
    archiveFile?: ArchiveAttachment | null;
  }

  let {
    folder,
    initialFolderId = null,
    post = null,
    open,
    onclose,
    archive = null,
    archiveFile = null
  }: Props = $props();

  type RowStatus = 'idle' | 'queued' | 'active' | 'paused' | 'done' | 'failed';
  type ViewMode = 'list' | 'grid';

  interface RowState {
    status: RowStatus;
    progress: number | null;
  }

  interface FolderStats {
    files: number;
    size: number;
  }

  interface TreeIndex {
    byId: Map<string, CloudNode>;
    children: Map<string | null, CloudNode[]>;
    stats: Map<string | null, FolderStats>;
    kinds: Map<string, NodeKind>;
  }

  // Pawchive allows five concurrent connections per client and downloads share them.
  const THUMBNAIL_CONCURRENCY = 3;
  const TALL_SHEET_ENTRIES = 12;
  const VIEW_STORAGE_KEY = 'pawstash_cloud_view';

  const PASSWORD_SOURCE_KEYS: Record<NonNullable<ArchiveOpenResult['password_source']>, string> = {
    post: 'cloud.password_source_post',
    creator: 'cloud.password_source_creator',
    user: 'cloud.password_source_user'
  };

  const VIEWER_KINDS = new Set<NodeKind>(['image', 'video', 'audio']);
  const SETTLED = new Set<RowStatus>(['done', 'queued', 'active', 'paused']);
  const IDLE: RowState = { status: 'idle', progress: null };
  const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });
  const lazyImage = createLazyImageLoader(THUMBNAIL_CONCURRENCY);

  let archiveInfo = $derived(archive);
  let source = $derived(archiveInfo?.folder ?? folder);
  let isArchive = $derived(Boolean(archiveInfo) || source?.provider === 'archive');
  let locked = $derived(archiveInfo?.status === 'password_required');
  let unsupported = $derived(archiveInfo?.status === 'unsupported');

  let currentId = $state<string | null>(null);
  let selected = $state<Set<string>>(new Set());
  let previewIndex = $state<number | null>(null);
  let viewport = $state<HTMLElement | null>(null);
  let queueing = $state(false);
  let viewPrefs = $state<Record<string, ViewMode>>(readViewPrefs());

  function compareNodes(a: CloudNode, b: CloudNode): number {
    if (a.is_folder !== b.is_folder) return a.is_folder ? -1 : 1;
    return collator.compare(a.name, b.name);
  }

  function buildIndex(nodes: CloudNode[]): TreeIndex {
    const byId = new Map(nodes.map((node) => [node.id, node]));
    const children = new Map<string | null, CloudNode[]>();
    const kinds = new Map<string, NodeKind>();
    for (const node of nodes) {
      const parent = node.parent_id && byId.has(node.parent_id) ? node.parent_id : null;
      const siblings = children.get(parent);
      if (siblings) siblings.push(node);
      else children.set(parent, [node]);
      kinds.set(node.id, nodeKindOf(node));
    }
    for (const list of children.values()) list.sort(compareNodes);

    const stats = new Map<string | null, FolderStats>();
    const visit = (id: string | null): FolderStats => {
      const known = stats.get(id);
      if (known) return known;
      const total: FolderStats = { files: 0, size: 0 };
      stats.set(id, total);
      for (const child of children.get(id) ?? []) {
        if (child.is_folder) {
          const sub = visit(child.id);
          total.files += sub.files;
          total.size += sub.size;
        } else {
          total.files++;
          total.size += child.size ?? 0;
        }
      }
      return total;
    };
    visit(null);
    for (const node of nodes) if (node.is_folder) visit(node.id);
    return { byId, children, stats, kinds };
  }

  let tree = $derived(buildIndex(source?.nodes ?? []));

  function parentOf(node: CloudNode): string | null {
    return node.parent_id && tree.byId.has(node.parent_id) ? node.parent_id : null;
  }

  function nodeKind(node: CloudNode): NodeKind {
    return tree.kinds.get(node.id) ?? nodeKindOf(node);
  }

  function filesUnder(id: string | null): CloudNode[] {
    const files: CloudNode[] = [];
    const stack = [...(tree.children.get(id) ?? [])].reverse();
    while (stack.length > 0) {
      const node = stack.pop()!;
      if (node.is_folder) stack.push(...[...(tree.children.get(node.id) ?? [])].reverse());
      else files.push(node);
    }
    return files;
  }

  function downloadName(file: CloudNode): string {
    if (file === archiveNode || source?.is_single_file) return file.name;
    const path = cloudNodePath(tree.byId, file);
    const container = source?.title || archiveFile?.name || '';
    if (isArchive) return `${archiveFolderName(container)}/${path}`;
    return path.includes('/') || !container ? path : `${container}/${path}`;
  }

  function startFolder(index: TreeIndex, target: string | null): string | null {
    if (target && index.byId.get(target)?.is_folder) return target;
    let id: string | null = null;
    for (;;) {
      const kids: CloudNode[] = index.children.get(id) ?? [];
      if (kids.length !== 1 || !kids[0].is_folder) return id;
      id = kids[0].id;
    }
  }

  let rootId = $derived(startFolder(tree, null));
  let items = $derived(tree.children.get(currentId) ?? []);

  let trail = $derived.by(() => {
    const path: CloudNode[] = [];
    const seen = new Set<string>();
    let id = currentId;
    while (id && !seen.has(id)) {
      seen.add(id);
      const node = tree.byId.get(id);
      if (!node) break;
      path.unshift(node);
      if (id === rootId) break;
      id = parentOf(node);
    }
    return path[0]?.id === rootId ? path.slice(1) : path;
  });

  let inSubfolder = $derived(currentId !== rootId);

  let openKey = '';
  $effect(() => {
    if (!open) {
      openKey = '';
      return;
    }
    const index = tree;
    const key = `${source?.url ?? archiveFile?.media_id ?? ''}|${initialFolderId ?? ''}`;
    untrack(() => {
      if (key !== openKey) {
        openKey = key;
        currentId = startFolder(index, initialFolderId);
        selected = new Set();
        previewIndex = null;
      } else if (currentId && !index.byId.has(currentId)) {
        currentId = startFolder(index, null);
      }
    });
  });

  $effect(() => {
    if (open) void serverPortState.ensurePort();
  });

  $effect(() => {
    if (!open) return;
    const handleKeydown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || selected.size === 0 || previewIndex !== null) return;
      event.preventDefault();
      event.stopPropagation();
      selected = new Set();
    };
    window.addEventListener('keydown', handleKeydown, true);
    return () => window.removeEventListener('keydown', handleKeydown, true);
  });

  function absoluteUrl(url?: string | null): string {
    if (!url) return '';
    return url.startsWith('/cloud_stream/') ? serverPortState.mediaUrl(url) : url;
  }

  function downloadUrl(node: CloudNode): string {
    return absoluteUrl(node.download_url || node.stream_url);
  }

  let archiveNode = $derived<CloudNode | null>(
    archiveFile
      ? {
          id: archiveFile.media_id,
          name: archiveFile.name,
          size: archiveFile.size ?? null,
          is_folder: false,
          download_url: archiveFile.url
        }
      : null
  );

  let jobs = $derived.by(() => {
    const byMedia = new Map<string, DownloadItem>();
    const byUrl = new Map<string, DownloadItem>();
    for (const job of downloadState.downloads) {
      if (!byMedia.has(job.media_id)) byMedia.set(job.media_id, job);
      if (!byUrl.has(job.url)) byUrl.set(job.url, job);
    }
    const found = new Map<string, DownloadItem>();
    const nodes = archiveNode ? [...(source?.nodes ?? []), archiveNode] : source?.nodes ?? [];
    for (const node of nodes) {
      if (node.is_folder) continue;
      const job =
        byMedia.get(node.id) ??
        (node.stream_url ? byMedia.get(node.stream_url) : undefined) ??
        (node.download_url ? byMedia.get(node.download_url) : undefined) ??
        byUrl.get(downloadUrl(node));
      if (job) found.set(node.id, job);
    }
    return found;
  });

  function byteRatio(job: DownloadItem): number | null {
    return job.total_bytes > 0 ? Math.min(1, job.downloaded_bytes / job.total_bytes) : null;
  }

  function jobState(job?: DownloadItem): RowState {
    if (!job) return IDLE;
    switch (job.status) {
      case 'completed':
        return { status: 'done', progress: 1 };
      case 'queued':
      case 'resolving':
        return { status: 'queued', progress: null };
      case 'downloading':
      case 'verifying':
        return { status: 'active', progress: byteRatio(job) };
      case 'paused':
        return { status: 'paused', progress: byteRatio(job) };
      case 'failed':
        return isAutoRetryPending(job) ? { status: 'queued', progress: null } : { status: 'failed', progress: null };
      default:
        return IDLE;
    }
  }

  function folderState(id: string): RowState {
    let files = 0;
    let done = 0;
    let busy = 0;
    let total = 0;
    let received = 0;
    for (const file of filesUnder(id)) {
      const job = jobs.get(file.id);
      const state = jobState(job);
      const size = file.size || job?.total_bytes || 0;
      files++;
      total += size;
      if (state.status === 'done') {
        done++;
        received += size;
      } else if (state.status === 'queued' || state.status === 'active' || state.status === 'paused') {
        busy++;
        received += job?.downloaded_bytes ?? 0;
      }
    }
    if (files > 0 && done === files) return { status: 'done', progress: 1 };
    if (busy > 0) return { status: 'active', progress: total > 0 ? received / total : null };
    return IDLE;
  }

  let folderStates = $derived(
    new Map(items.filter((node) => node.is_folder).map((node) => [node.id, folderState(node.id)]))
  );

  function rowState(node: CloudNode): RowState {
    return node.is_folder ? folderStates.get(node.id) ?? IDLE : jobState(jobs.get(node.id));
  }

  function statusLabel(state: RowState): string {
    const percent = state.progress !== null ? formatPercent(state.progress * 100) : '';
    switch (state.status) {
      case 'queued':
        return i18n.t('downloads.status_queued');
      case 'active':
        return percent || i18n.t('downloads.status_downloading');
      case 'paused':
        return [i18n.t('downloads.status_paused'), percent].filter(Boolean).join(' · ');
      case 'failed':
        return i18n.t('downloads.status_failed');
      default:
        return '';
    }
  }

  function streamUrl(node: CloudNode): string {
    const job = jobs.get(node.id);
    if (job?.status === 'completed' && job.final_path) {
      return resolveLocalMediaUrl(job.final_path) ?? '';
    }
    return absoluteUrl(node.stream_url || node.download_url);
  }

  function thumbnailUrl(node: CloudNode, grid: boolean): string {
    if (locked || node.is_folder) return '';
    const kind = nodeKind(node);
    const job = jobs.get(node.id);
    if (kind === 'image' && job?.status === 'completed' && job.final_path) {
      return resolveLocalMediaUrl(job.final_path) ?? '';
    }
    if (node.thumbnail_url) return absoluteUrl(node.thumbnail_url);
    return grid && kind === 'image' ? streamUrl(node) : '';
  }

  function secondaryText(node: CloudNode): string {
    if (node.is_folder) {
      const stats = tree.stats.get(node.id);
      return fileSummary(stats?.files ?? 0, stats?.size ?? 0);
    }
    const size = node.size || jobs.get(node.id)?.total_bytes || 0;
    return size > 0 ? formatBytes(size) : '';
  }

  let totals = $derived(tree.stats.get(null) ?? { files: 0, size: 0 });
  let selectable = $derived(!locked && !unsupported && totals.files > 1);

  let selectedUnder = $derived.by(() => {
    const counts = new Map<string | null, number>();
    for (const id of selected) {
      const node = tree.byId.get(id);
      if (!node) continue;
      counts.set(null, (counts.get(null) ?? 0) + 1);
      const seen = new Set<string>();
      let parent = parentOf(node);
      while (parent && !seen.has(parent)) {
        seen.add(parent);
        counts.set(parent, (counts.get(parent) ?? 0) + 1);
        const ancestor = tree.byId.get(parent);
        parent = ancestor ? parentOf(ancestor) : null;
      }
    }
    return counts;
  });

  let selectedFiles = $derived(
    [...selected].map((id) => tree.byId.get(id)).filter((node): node is CloudNode => Boolean(node))
  );
  let selectedBytes = $derived(selectedFiles.reduce((sum, node) => sum + (node.size ?? 0), 0));
  let filesHere = $derived(tree.stats.get(currentId)?.files ?? 0);
  let allHereSelected = $derived(filesHere > 0 && (selectedUnder.get(currentId) ?? 0) === filesHere);

  function isChecked(node: CloudNode): boolean {
    if (!node.is_folder) return selected.has(node.id);
    const files = tree.stats.get(node.id)?.files ?? 0;
    return files > 0 && (selectedUnder.get(node.id) ?? 0) === files;
  }

  function setSelected(files: CloudNode[], value: boolean) {
    const next = new Set(selected);
    for (const file of files) {
      if (value) next.add(file.id);
      else next.delete(file.id);
    }
    selected = next;
  }

  function toggleNode(node: CloudNode) {
    setSelected(node.is_folder ? filesUnder(node.id) : [node], !isChecked(node));
  }

  function toggleAllHere() {
    setSelected(filesUnder(currentId), !allHereSelected);
  }

  let viewerNodes = $derived(locked ? [] : items.filter((node) => VIEWER_KINDS.has(nodeKind(node))));

  let viewerItems = $derived(
    viewerNodes.map((node): MediaViewerItem => {
      const job = jobs.get(node.id);
      return {
        id: node.id,
        url: streamUrl(node),
        name: node.name,
        kind: nodeKind(node) as MediaViewerKind,
        size: node.size ?? undefined,
        poster: node.thumbnail_url ? absoluteUrl(node.thumbnail_url) : undefined,
        downloadStatus: job?.status,
        downloadedBytes: job?.downloaded_bytes,
        totalBytes: job?.total_bytes,
        downloadedPath: job?.status === 'completed' ? job.final_path : undefined
      };
    })
  );

  function canActivate(node: CloudNode): boolean {
    return node.is_folder || (!locked && (selectable || VIEWER_KINDS.has(nodeKind(node))));
  }

  function activate(node: CloudNode) {
    if (node.is_folder) {
      currentId = node.id;
      return;
    }
    if (locked) return;
    const index = viewerNodes.findIndex((candidate) => candidate.id === node.id);
    if (index >= 0) {
      previewIndex = index;
      return;
    }
    if (selectable) toggleNode(node);
  }

  function goUp() {
    const current = currentId ? tree.byId.get(currentId) : null;
    currentId = current ? parentOf(current) : null;
  }

  function readViewPrefs(): Record<string, ViewMode> {
    if (typeof localStorage === 'undefined') return {};
    const raw = localStorage.getItem(VIEW_STORAGE_KEY);
    if (!raw) return {};
    try {
      return JSON.parse(raw) as Record<string, ViewMode>;
    } catch (error) {
      logger.warn('Ignoring unreadable cloud view preference', error);
      return {};
    }
  }

  let viewKey = $derived(isArchive ? 'archive' : source?.provider || 'cloud');
  let hasThumbnails = $derived(items.some((node) => ['image', 'video'].includes(nodeKind(node))));
  let view = $derived<ViewMode>(hasThumbnails ? viewPrefs[viewKey] ?? 'list' : 'list');

  function toggleView() {
    viewPrefs = { ...viewPrefs, [viewKey]: view === 'grid' ? 'list' : 'grid' };
    localStorage.setItem(VIEW_STORAGE_KEY, JSON.stringify(viewPrefs));
  }

  function downloadTarget(): Post {
    if (post) return post;
    const title = source?.title || archiveFile?.name || i18n.t('post.cloud_generic');
    return {
      id: title,
      service: source?.provider || 'cloud',
      user: 'cloud',
      title,
      content: source?.url || ''
    };
  }

  async function queueFiles(files: CloudNode[], from?: string) {
    if (queueing || files.length === 0) return;
    const pending = files.filter((file) => !SETTLED.has(jobState(jobs.get(file.id)).status));
    if (pending.length === 0) {
      const single = files.length === 1;
      const done = single && jobState(jobs.get(files[0].id)).status === 'done';
      notify.info(
        i18n.t(single ? (done ? 'downloads.already_downloaded' : 'downloads.already_in_progress') : 'downloads.all_downloaded')
      );
      return;
    }

    queueing = true;
    let started = 0;
    const failures: unknown[] = [];
    try {
      await serverPortState.ensurePort();
      const target = downloadTarget();
      for (const file of pending) {
        try {
          const url = downloadUrl(file);
          if (!url) throw new Error(i18n.t('post.cloud_file_unavailable'));
          await downloadState.start(target, file.id, url, downloadName(file));
          started++;
        } catch (error) {
          logger.warn(`Failed to queue "${file.name}"`, error);
          failures.push(error);
        }
      }
    } finally {
      queueing = false;
    }

    if (failures.length > 0) {
      notify.error(i18n.t('post.download_all_failed', { count: failures.length }), failures[0]);
    }
    if (started === 0) return;
    const description = from
      ? i18n.t('downloads.files_queued_from', { count: started, name: from })
      : pending.length === 1
        ? pending[0].name
        : i18n.t('downloads.files_queued', { count: started });
    notify.success(i18n.t('feed.download_started'), { description, glyph: 'download' });
  }

  async function downloadSelected() {
    await queueFiles(selectedFiles);
    selected = new Set();
  }

  async function retry(job: DownloadItem) {
    try {
      await downloadState.retry(job.id);
    } catch (error) {
      notify.error(i18n.t('feed.download_failed'), error);
    }
  }

  async function showInFolder(job: DownloadItem) {
    try {
      await apiShowInFolder(job.final_path);
    } catch (error) {
      notify.error(i18n.t('downloads.show_in_folder_failed'), error);
    }
  }

  async function unlockArchive(password: string) {
    if (!archiveInfo) return;
    archiveInfo = await apiUnlockArchive(archiveInfo.archive_id, password);
  }

  let archiveJob = $derived(archiveNode ? jobs.get(archiveNode.id) : undefined);
  let formatName = $derived((archiveInfo?.format && archiveInfo.format !== 'unknown' ? archiveInfo.format : '').toUpperCase());
  let providerLabel = $derived(
    isArchive
      ? formatName
        ? i18n.t('cloud.archive_format', { format: formatName })
        : i18n.t('cloud.type_archive')
      : cloudProviderName(source?.provider) ?? (source?.provider || i18n.t('post.cloud_generic'))
  );
  let ProviderIcon = $derived(isArchive ? IconZip : IconCloud);
  let title = $derived(source?.title || archiveFile?.name || providerLabel);

  let metaText = $derived.by(() => {
    if (unsupported) {
      const size = archiveFile?.size || archiveJob?.total_bytes || 0;
      return size > 0 ? `${providerLabel} · ${formatBytes(size)}` : providerLabel;
    }
    return totals.files > 0 ? `${providerLabel} · ${fileSummary(totals.files, totals.size)}` : providerLabel;
  });

  let passwordSourceKey = $derived(
    archiveInfo?.status === 'ready' && archiveInfo.password_source
      ? PASSWORD_SOURCE_KEYS[archiveInfo.password_source]
      : undefined
  );
  let canExtractAll = $derived(isArchive && !locked && !unsupported && totals.files > 0);
  let showNavRow = $derived(inSubfolder || (selectable && filesHere > 0));
  let compact = $derived(unsupported || (source?.nodes.length ?? 0) <= 1);
  let tallSheet = $derived((source?.nodes.length ?? 0) > TALL_SHEET_ENTRIES);
  let hasSelection = $derived(selected.size > 0);
</script>

{#snippet thumb(node: CloudNode, grid: boolean)}
  {@const Icon = NODE_KIND_ICONS[nodeKind(node)]}
  {@const src = thumbnailUrl(node, grid)}
  <span class="cf-thumb" class:is-folder={node.is_folder} aria-hidden="true">
    <Icon class="cf-thumb-icon" />
    {#if src}
      <img
        alt=""
        decoding="async"
        use:lazyImage={{ src, root: viewport }}
        onload={(event) => event.currentTarget.setAttribute('data-loaded', '')}
      />
    {/if}
  </span>
{/snippet}

{#snippet status(node: CloudNode)}
  {@const state = rowState(node)}
  {@const job = node.is_folder ? undefined : jobs.get(node.id)}
  {#if state.status === 'done'}
    {#if job?.final_path}
      <Button
        variant="ghost"
        class="btn-icon"
        tooltip={i18n.t('downloads.show_in_folder')}
        onclick={() => void showInFolder(job)}
      >
        <IconFolderOpen />
      </Button>
    {/if}
    <span class="cf-status is-done" role="img" aria-label={i18n.t('downloads.status_completed')} use:tooltip={i18n.t('downloads.status_completed')}>
      <IconCheck />
    </span>
  {:else if state.status === 'failed' && job}
    <Button
      variant="ghost"
      class="btn-icon cf-failed"
      tooltip={`${i18n.t('downloads.status_failed')} · ${i18n.t('downloads.retry')}`}
      onclick={() => void retry(job)}
    >
      <IconRetry />
    </Button>
  {:else if state.status !== 'idle'}
    <span class="cf-status" role="img" aria-label={statusLabel(state)} use:tooltip={statusLabel(state)}>
      <ProgressRing value={state.status === 'queued' ? null : state.progress} size={24} />
    </span>
  {:else}
    <Button
      variant="ghost"
      class="btn-icon"
      disabled={locked || queueing || (!node.is_folder && !downloadUrl(node))}
      tooltip={i18n.t('post.download')}
      onclick={() => void queueFiles(node.is_folder ? filesUnder(node.id) : [node], node.is_folder ? node.name : undefined)}
    >
      <IconDownload />
    </Button>
  {/if}
{/snippet}

{#snippet secondary(node: CloudNode)}
  {@const state = rowState(node)}
  {@const label = statusLabel(state)}
  {@const text = secondaryText(node)}
  {#if node.is_encrypted || text || label}
    <span class="cf-secondary">
      {#if node.is_encrypted}
        <span class="cf-lock" role="img" aria-label={i18n.t('cloud.encrypted')} use:tooltip={i18n.t('cloud.encrypted')}><IconLock /></span>
      {/if}
      {#if text}
        <span class="cf-secondary-text">{text}</span>
      {/if}
      {#if label}
        <span class="cf-secondary-status" class:is-failed={state.status === 'failed'}>{text ? `· ${label}` : label}</span>
      {/if}
    </span>
  {/if}
{/snippet}

{#snippet row(node: CloudNode)}
  <li class="cf-row" class:is-selected={selectable && isChecked(node)}>
    {#if selectable}
      <Checkbox
        checked={isChecked(node)}
        ariaLabel={node.name}
        disabled={node.is_folder && (tree.stats.get(node.id)?.files ?? 0) === 0}
        onchange={() => toggleNode(node)}
      />
    {/if}
    <button
      type="button"
      class="cf-row-main"
      disabled={!canActivate(node)}
      onclick={() => activate(node)}
    >
      {@render thumb(node, false)}
      <span class="cf-row-text">
        <span class="cf-name">{node.name}</span>
        {@render secondary(node)}
      </span>
    </button>
    <span class="cf-row-actions">
      {@render status(node)}
    </span>
  </li>
{/snippet}

{#snippet tile(node: CloudNode)}
  <li class="cf-tile" class:is-selected={selectable && isChecked(node)}>
    <button
      type="button"
      class="cf-tile-main"
      disabled={!canActivate(node)}
      aria-label={node.name}
      use:tooltip={node.name}
      onclick={() => activate(node)}
    >
      {@render thumb(node, true)}
    </button>
    {#if selectable}
      <span class="cf-tile-check">
        <Checkbox checked={isChecked(node)} ariaLabel={node.name} onchange={() => toggleNode(node)} />
      </span>
    {/if}
    <span class="cf-tile-footer">
      <span class="cf-row-text">
        <span class="cf-name">{node.name}</span>
        {@render secondary(node)}
      </span>
      <span class="cf-row-actions">
        {@render status(node)}
      </span>
    </span>
  </li>
{/snippet}

{#snippet unsupportedPanel()}
  <div class="cf-panel">
    <p class="cf-panel-text">{i18n.t('cloud.unsupported')}</p>
    {#if archiveNode}
      <ul class="cf-list is-inline">
        {@render row(archiveNode)}
      </ul>
    {/if}
    {#if archiveJob?.status === 'completed' && archiveJob.final_path}
      <div class="dialog-actions">
        <Button variant="ghost" data-tone="accent" onclick={() => void openDownloadedFile(archiveJob.final_path)}>
          <IconOpen />
          {i18n.t('cloud.open_in_system')}
        </Button>
      </div>
    {/if}
  </div>
{/snippet}

{#snippet content()}
  <div
    class="cf"
    class:is-sheet={layoutState.isMobile}
    class:has-footer={hasSelection}
    data-control-size={layoutState.isMobile ? 'md' : 'sm'}
  >
    <div class="cf-meta-row">
      <p class="cf-meta">
        <ProviderIcon class="cf-meta-icon" aria-hidden="true" />
        <span class="cf-meta-text">{metaText}</span>
        {#if passwordSourceKey}
          <span class="cf-meta-key"><IconKey aria-hidden="true" />{i18n.t(passwordSourceKey)}</span>
        {/if}
      </p>
      {#if hasThumbnails || canExtractAll}
        <span class="cf-meta-actions">
          {#if hasThumbnails}
            <Button
              variant="ghost"
              class="btn-icon"
              tooltip={i18n.t(view === 'grid' ? 'cloud.view_list' : 'cloud.view_grid')}
              onclick={toggleView}
            >
              {#if view === 'grid'}<IconList />{:else}<IconGrid />{/if}
            </Button>
          {/if}
          {#if canExtractAll}
            <Button variant="tonal" disabled={queueing} onclick={() => void queueFiles(filesUnder(null), title)}>
              <IconDownload />
              {i18n.t('cloud.extract_all')}
            </Button>
          {/if}
        </span>
      {/if}
    </div>

    {#if unsupported}
      {@render unsupportedPanel()}
    {:else}
      {#if locked && archiveInfo}
        {#key archiveInfo.archive_id}
          <div class="cf-panel">
            <SecretUnlockPanel
              kind="password"
              title={i18n.t('cloud.archive_locked')}
              hints={archiveInfo.password_hints}
              autofocus
              unlock={unlockArchive}
            />
          </div>
        {/key}
      {/if}

      {#if showNavRow}
        <div class="cf-nav">
          {#if selectable && filesHere > 0}
            <label class="cf-select-all">
              <Checkbox
                checked={allHereSelected}
                ariaLabel={i18n.t(allHereSelected ? 'selection.deselect_all' : 'selection.select_all')}
                onchange={toggleAllHere}
              />
              {#if !inSubfolder}
                <span>{i18n.t(allHereSelected ? 'selection.deselect_all' : 'selection.select_all')}</span>
              {/if}
            </label>
          {/if}
          {#if inSubfolder}
            <nav class="cf-path" aria-label={title}>
              <Button variant="ghost" class="btn-icon" tooltip={i18n.t('downloads.cloud_go_up')} onclick={goUp}>
                <IconArrowUp />
              </Button>
              <ol class="cf-crumbs">
                <li>
                  <Button variant="ghost" class="btn-icon" tooltip={title} onclick={() => (currentId = rootId)}>
                    <ProviderIcon />
                  </Button>
                </li>
                {#each trail as crumb, index (crumb.id)}
                  <li class="cf-crumb">
                    <IconChevronRight class="cf-crumb-sep" aria-hidden="true" />
                    {#if index === trail.length - 1}
                      <span class="cf-crumb-current" aria-current="page" use:tooltip={crumb.name}>{crumb.name}</span>
                    {:else}
                      <Button variant="ghost" class="cf-crumb-btn" tooltip={crumb.name} onclick={() => (currentId = crumb.id)}>
                        <span class="cf-crumb-label">{crumb.name}</span>
                      </Button>
                    {/if}
                  </li>
                {/each}
              </ol>
            </nav>
          {/if}
        </div>
      {/if}

      <div class="cf-scroll" use:scrollable={{ overflowX: 'hidden', onReady: (element) => (viewport = element) }}>
        {#if items.length === 0}
          {#if !locked}
            <p class="cf-empty">{i18n.t('downloads.cloud_folder_empty')}</p>
          {/if}
        {:else if view === 'grid'}
          <ul class="cf-grid">
            {#each items as node (node.id)}
              {@render tile(node)}
            {/each}
          </ul>
        {:else}
          <ul class="cf-list">
            {#each items as node (node.id)}
              {@render row(node)}
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
  </div>
{/snippet}

{#snippet selectionFooter()}
  <div class="cf-footer" class:is-sheet={layoutState.isMobile} data-control-size="md">
    <span class="cf-footer-summary">{fileSummary(selected.size, selectedBytes)}</span>
    <div class="dialog-actions">
      <Button variant="ghost" onclick={() => (selected = new Set())}>
        {i18n.t('selection.deselect_all')}
      </Button>
      <Button variant="ghost" data-tone="accent" disabled={queueing} onclick={() => void downloadSelected()}>
        <IconDownload />
        {i18n.t('post.download')}
      </Button>
    </div>
  </div>
{/snippet}

{#if layoutState.isMobile}
  <BottomSheet
    isOpen={open}
    {title}
    {onclose}
    tall={tallSheet}
    flush
    scrollable={false}
    footer={hasSelection ? selectionFooter : undefined}
  >
    {@render content()}
  </BottomSheet>
{:else}
  <Modal
    isOpen={open}
    {title}
    {onclose}
    size={compact ? 'lg' : 'xl'}
    flush
    scrollable={false}
    footer={hasSelection ? selectionFooter : undefined}
  >
    {@render content()}
  </Modal>
{/if}

{#if previewIndex !== null && viewerItems.length > 0}
  <MediaViewer
    items={viewerItems}
    initialIndex={previewIndex}
    onclose={() => (previewIndex = null)}
    ondownload={(item) => {
      const node = tree.byId.get(item.id);
      if (node) void queueFiles([node]);
    }}
  />
{/if}

<style>
  .cf {
    --cf-inset: calc(var(--modal-padding) * var(--ui-scale, 1));
    --cf-gap: calc(var(--space-2) * var(--ui-scale, 1));
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-height: 0;
    font-family: var(--font-sans);
    color: var(--text-primary);
  }

  .cf.is-sheet {
    --cf-inset: calc(var(--bottom-sheet-inset) * var(--ui-scale, 1));
  }

  .cf-meta-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--cf-gap);
    padding: 0 var(--cf-inset) var(--cf-gap);
  }

  .cf-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--space-1) * var(--ui-scale, 1)) var(--cf-gap);
    min-width: 0;
    margin: 0;
    font-size: calc(var(--text-xs) * var(--ui-scale, 1));
    line-height: var(--leading-normal);
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .cf-meta :global(.cf-meta-icon),
  .cf-meta-key :global(svg) {
    flex: none;
    width: calc(var(--control-icon-sm) * var(--ui-scale, 1));
    height: calc(var(--control-icon-sm) * var(--ui-scale, 1));
    color: var(--text-muted);
  }

  .cf-meta-key {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
  }

  .cf-meta-actions {
    display: flex;
    align-items: center;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
    margin-left: auto;
  }

  .cf-panel {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-3) * var(--ui-scale, 1));
    margin: 0 var(--cf-inset) calc(var(--space-3) * var(--ui-scale, 1));
    padding: calc(var(--space-4) * var(--ui-scale, 1));
    border-radius: calc(var(--radius-lg) * var(--ui-scale, 1));
    background: var(--surface-container-highest);
  }

  .cf-panel-text {
    margin: 0;
    font-size: calc(var(--text-sm) * var(--ui-scale, 1));
    line-height: var(--leading-relaxed);
    color: var(--text-secondary);
  }

  .cf-nav {
    display: flex;
    align-items: center;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
    min-width: 0;
    padding: 0 var(--cf-inset) calc(var(--space-1) * var(--ui-scale, 1));
    padding-left: calc(var(--cf-inset) - var(--space-1) * var(--ui-scale, 1));
  }

  .cf-path {
    display: flex;
    align-items: center;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
    min-width: 0;
  }

  .cf-select-all {
    display: inline-flex;
    align-items: center;
    gap: var(--cf-gap);
    padding: 0 calc(var(--space-2) * var(--ui-scale, 1)) 0 calc(var(--space-1) * var(--ui-scale, 1));
    font-size: calc(var(--text-xs) * var(--ui-scale, 1));
    color: var(--text-secondary);
    cursor: pointer;
  }

  .cf-crumbs {
    display: flex;
    align-items: center;
    min-width: 0;
    margin: 0;
    padding: 0;
    list-style: none;
    overflow: hidden;
  }

  .cf-crumbs > li {
    display: flex;
    align-items: center;
    min-width: 0;
  }

  .cf-crumbs > li:first-child {
    flex: none;
  }

  .cf-crumbs :global(.cf-crumb-sep) {
    flex: none;
    width: calc(var(--control-icon-sm) * var(--ui-scale, 1));
    height: calc(var(--control-icon-sm) * var(--ui-scale, 1));
    color: var(--text-muted);
  }

  .cf-crumbs :global(.cf-crumb-btn) {
    flex-shrink: 1;
    min-width: 0;
    padding: 0 calc(var(--space-2) * var(--ui-scale, 1));
  }

  .cf-crumb-label,
  .cf-crumb-current {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cf-crumb-current {
    padding: 0 calc(var(--space-2) * var(--ui-scale, 1));
    font-size: calc(var(--control-font-size) * var(--ui-scale, 1));
    font-weight: var(--font-weight-semibold);
  }

  .cf-scroll {
    flex: 0 1 auto;
    min-height: 0;
    padding-bottom: calc(var(--space-3) * var(--ui-scale, 1));
  }

  .cf.is-sheet:not(.has-footer) .cf-scroll {
    padding-bottom: max(calc(var(--space-3) * var(--ui-scale, 1)), var(--safe-bottom));
  }

  .cf-empty {
    margin: 0;
    padding: calc(var(--space-8) * var(--ui-scale, 1)) var(--cf-inset);
    text-align: center;
    font-size: calc(var(--text-sm) * var(--ui-scale, 1));
    color: var(--text-secondary);
  }

  .cf-list,
  .cf-grid {
    margin: 0;
    padding: 0 calc(var(--cf-inset) - var(--space-2) * var(--ui-scale, 1));
    list-style: none;
  }

  .cf-list.is-inline {
    padding: 0;
  }

  .cf-row {
    display: flex;
    align-items: center;
    gap: var(--cf-gap);
    min-height: calc(var(--file-row-min-height) * var(--ui-scale, 1));
    padding: 0 calc(var(--space-2) * var(--ui-scale, 1));
    border-radius: calc(var(--radius-md) * var(--ui-scale, 1));
    transition: background var(--duration-fast) var(--ease-expo);
  }

  @media (hover: hover) {
    .cf-row:hover {
      background: var(--state-layer-hover);
    }
  }

  .cf-row.is-selected {
    background: var(--accent-container);
    color: var(--accent-on-container);
  }

  .cf-row-main {
    display: flex;
    flex: 1 1 auto;
    align-items: center;
    gap: calc(var(--space-3) * var(--ui-scale, 1));
    min-width: 0;
    align-self: stretch;
    padding: calc(var(--space-1) * var(--ui-scale, 1)) 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    border-radius: inherit;
  }

  .cf-row-main:disabled {
    cursor: default;
  }

  .cf-thumb {
    position: relative;
    display: grid;
    flex: none;
    place-items: center;
    width: calc(var(--file-row-thumb) * var(--ui-scale, 1));
    height: calc(var(--file-row-thumb) * var(--ui-scale, 1));
    overflow: hidden;
    border-radius: calc(var(--radius-sm) * var(--ui-scale, 1));
    background: var(--surface-container-highest);
    color: var(--text-secondary);
  }

  .cf-thumb.is-folder {
    background: var(--accent-container);
    color: var(--accent-on-container);
  }

  .cf-thumb :global(.cf-thumb-icon) {
    width: calc(var(--file-row-icon) * var(--ui-scale, 1));
    height: calc(var(--file-row-icon) * var(--ui-scale, 1));
  }

  .cf-thumb img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity var(--duration-normal) var(--ease-expo);
  }

  .cf-thumb img:global([data-loaded]) {
    opacity: 1;
  }

  .cf-row-text {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-width: 0;
    gap: calc(var(--space-1) * var(--ui-scale, 1) / 2);
  }

  .cf-name {
    display: -webkit-box;
    overflow: hidden;
    font-size: calc(var(--text-sm) * var(--ui-scale, 1));
    line-height: var(--leading-tight);
    overflow-wrap: anywhere;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .cf-secondary {
    display: flex;
    align-items: center;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
    min-width: 0;
    font-size: calc(var(--text-xs) * var(--ui-scale, 1));
    line-height: var(--leading-normal);
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .cf-secondary-text {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cf-secondary-status {
    flex: none;
    color: var(--accent-on-surface);
  }

  .cf-secondary-status.is-failed {
    color: var(--status-error);
  }

  .cf-lock {
    display: inline-flex;
    flex: none;
  }

  .cf-lock :global(svg) {
    width: calc(var(--text-xs) * var(--ui-scale, 1));
    height: calc(var(--text-xs) * var(--ui-scale, 1));
  }

  .cf-row-actions {
    display: flex;
    flex: none;
    align-items: center;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
  }

  .cf-status {
    position: relative;
    display: grid;
    place-items: center;
    width: calc(var(--control-height) * var(--ui-scale, 1));
    height: calc(var(--control-height) * var(--ui-scale, 1));
  }

  .cf-status.is-done {
    color: var(--status-success);
  }

  .cf-status :global(svg:not(.progress-ring)) {
    width: calc(var(--control-icon-size) * var(--ui-scale, 1));
    height: calc(var(--control-icon-size) * var(--ui-scale, 1));
  }

  .cf :global(.btn.cf-failed) {
    color: var(--status-error);
  }

  .cf-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(var(--file-tile-min) * var(--ui-scale, 1)), 1fr));
    gap: calc(var(--space-3) * var(--ui-scale, 1));
    padding: 0 var(--cf-inset);
  }

  .cf-tile {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
    min-width: 0;
  }

  .cf-tile-main {
    display: block;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
    border-radius: calc(var(--radius-md) * var(--ui-scale, 1));
  }

  .cf-tile-main:disabled {
    cursor: default;
  }

  .cf-tile .cf-thumb {
    width: 100%;
    height: auto;
    aspect-ratio: 1;
    border-radius: calc(var(--radius-md) * var(--ui-scale, 1));
  }

  .cf-tile .cf-thumb :global(.cf-thumb-icon) {
    width: calc(var(--file-tile-icon) * var(--ui-scale, 1));
    height: calc(var(--file-tile-icon) * var(--ui-scale, 1));
  }

  .cf-tile.is-selected .cf-thumb {
    outline: 2px solid var(--accent-primary);
    outline-offset: 2px;
  }

  .cf-tile-check {
    position: absolute;
    top: calc(var(--space-1) * var(--ui-scale, 1));
    left: calc(var(--space-1) * var(--ui-scale, 1));
    display: grid;
    place-items: center;
    padding: calc(var(--space-1) * var(--ui-scale, 1));
    border-radius: calc(var(--radius-sm) * var(--ui-scale, 1));
    background: var(--on-media-scrim);
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-expo);
  }

  .cf-tile:hover .cf-tile-check,
  .cf-tile:focus-within .cf-tile-check,
  .cf-tile.is-selected .cf-tile-check,
  .cf.has-footer .cf-tile-check {
    opacity: 1;
  }

  @media (pointer: coarse) {
    .cf-tile-check {
      opacity: 1;
    }
  }

  .cf-tile-footer {
    display: flex;
    align-items: center;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
    min-width: 0;
  }

  .cf-footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--cf-gap, calc(var(--space-2) * var(--ui-scale, 1)));
  }

  .cf-footer .dialog-actions {
    width: auto;
    margin-left: auto;
  }

  .cf-footer-summary {
    font-size: calc(var(--text-sm) * var(--ui-scale, 1));
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .cf-footer.is-sheet {
    flex-direction: column;
    align-items: stretch;
  }

  .cf-footer.is-sheet .dialog-actions {
    width: 100%;
    margin-left: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .cf-thumb img,
    .cf-tile-check {
      transition: none;
    }
  }
</style>
