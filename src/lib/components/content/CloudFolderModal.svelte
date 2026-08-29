<script lang="ts">
  import { onMount } from 'svelte';
  import type { CloudFolderResult, CloudNode } from '$lib/types/cloud';
  import type { Post } from '$lib/types/content';
  import { formatBytes } from '$lib/utils/formatters';
  import { i18n } from '$lib/i18n/i18nState.svelte';
  import { apiStartDownload } from '$lib/utils/ipc';
  import { serverPortState } from '$lib/state/serverPort.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { toast } from 'svelte-sonner';
  import { scrollable } from '$lib/actions/scrollable';
  import { ripple, tooltip } from '$lib/motion';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import MediaViewer from '$lib/components/content/MediaViewer.svelte';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconFolder from '~icons/fluent/folder-24-filled';
  import IconChevronRight from '~icons/fluent/chevron-right-20-regular';
  import IconChevronLeft from '~icons/fluent/chevron-left-20-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-clip-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconZip from '~icons/fluent/folder-zip-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconEye from '~icons/fluent/eye-24-regular';
  import IconSelectAll from '~icons/fluent/select-all-on-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';

  let {
    folder,
    initialFolderId = null,
    post = null,
    open,
    onclose
  }: {
    folder: CloudFolderResult | null;
    initialFolderId?: string | null;
    post?: Post | null;
    open: boolean;
    onclose: () => void;
  } = $props();

  let selectedIds = $state<Set<string>>(new Set());

  // Folder navigation history
  let currentFolderId = $state<string | null>(null);
  let history = $state<{ id: string | null; name: string }[]>([]);

  // Media previewer state
  let previewIndex = $state<number | null>(null);

  onMount(() => {
    void serverPortState.ensurePort();
  });

  function getEffectiveStartFolder(nodes: CloudNode[], targetId?: string | null): { id: string | null; name: string } {
    if (targetId) {
      const targetNode = nodes.find((n) => n.id === targetId);
      if (targetNode) {
        return { id: targetNode.id, name: targetNode.name };
      }
    }

    // Find top-level root container (nodes without a parent in the dataset)
    const roots = nodes.filter((n) => !n.parent_id || !nodes.some((p) => p.id === n.parent_id));
    if (roots.length === 1 && roots[0].is_folder) {
      const root = roots[0];
      const children = nodes.filter((n) => n.parent_id === root.id);
      // If root container only contains 1 folder child, start directly inside that folder
      if (children.length === 1 && children[0].is_folder) {
        return { id: children[0].id, name: children[0].name };
      }
      return { id: root.id, name: root.name };
    }

    return { id: null, name: folder?.title || 'Root' };
  }

  $effect(() => {
    if (folder && open) {
      const start = getEffectiveStartFolder(folder.nodes, initialFolderId);
      currentFolderId = start.id;
      history = [{ id: start.id, name: start.name }];
    }
  });

  function isMedia(node: CloudNode): boolean {
    const ext = node.name.split('.').pop()?.toLowerCase() || '';
    const mime = (node.mime_type || '').toLowerCase();
    return (
      mime.startsWith('image/') ||
      mime.startsWith('video/') ||
      ['jpg', 'jpeg', 'png', 'gif', 'webp', 'mp4', 'mkv', 'webm', 'mov', 'avi'].includes(ext)
    );
  }

  function getNodeTypeLabel(node: CloudNode): string {
    if (node.is_folder) return 'Folder';
    const ext = node.name.split('.').pop()?.toLowerCase() || '';
    const mime = (node.mime_type || '').toLowerCase();

    if (mime.startsWith('video/') || ['mp4', 'mkv', 'webm', 'mov', 'avi'].includes(ext)) {
      return 'Video';
    }
    if (mime.startsWith('image/') || ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp'].includes(ext)) {
      return 'Image';
    }
    if (['zip', 'rar', '7z', 'tar', 'gz', 'bz2'].includes(ext) || mime.includes('zip') || mime.includes('compressed')) {
      return 'ZIP compressed';
    }
    return ext ? `${ext.toUpperCase()} file` : 'File';
  }

  // Collect all file nodes recursively inside a folder
  function getAllDescendantFiles(folderId: string): CloudNode[] {
    if (!folder?.nodes) return [];
    const result: CloudNode[] = [];
    const queue = [folderId];
    while (queue.length > 0) {
      const current = queue.shift()!;
      for (const n of folder.nodes) {
        if (n.parent_id === current) {
          if (n.is_folder) {
            queue.push(n.id);
          } else {
            result.push(n);
          }
        }
      }
    }
    return result;
  }

  // Compute folder total size from all descendant files
  function getFolderTotalSize(folderId: string): number {
    const descendantFiles = getAllDescendantFiles(folderId);
    return descendantFiles.reduce((sum, f) => sum + (f.size || 0), 0);
  }

  function getNodeDownloadJob(node: CloudNode) {
    return downloadState.downloads.find((item) =>
      item.media_id === node.id ||
      (node.name && item.filename === node.name) ||
      (node.download_url && item.url === node.download_url)
    );
  }

  function getNodeProgress(node: CloudNode): {
    status: 'none' | 'completed' | 'downloading' | 'queued' | 'paused' | 'failed';
    percent: number;
    downloaded: number;
    total: number;
  } {
    if (node.is_folder) {
      const descendants = getAllDescendantFiles(node.id);
      if (descendants.length === 0) return { status: 'none', percent: 0, downloaded: 0, total: 0 };

      let completedCount = 0;
      let activeCount = 0;
      let totalBytes = 0;
      let downloadedBytes = 0;

      for (const f of descendants) {
        const job = getNodeDownloadJob(f);
        const fSize = f.size || (job?.total_bytes) || 0;
        totalBytes += fSize;

        if (job?.status === 'completed') {
          downloadedBytes += fSize;
          completedCount++;
        } else if (job && ['downloading', 'resolving', 'verifying', 'queued'].includes(job.status)) {
          activeCount++;
          downloadedBytes += job.downloaded_bytes || 0;
        }
      }

      if (completedCount === descendants.length) {
        return { status: 'completed', percent: 100, downloaded: totalBytes, total: totalBytes };
      }
      if (activeCount > 0) {
        const percent = totalBytes > 0 ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)) : 0;
        return { status: 'downloading', percent, downloaded: downloadedBytes, total: totalBytes };
      }
      if (completedCount > 0) {
        const percent = totalBytes > 0 ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)) : 0;
        return { status: 'paused', percent, downloaded: downloadedBytes, total: totalBytes };
      }
      return { status: 'none', percent: 0, downloaded: 0, total: totalBytes };
    }

    const job = getNodeDownloadJob(node);
    if (!job) return { status: 'none', percent: 0, downloaded: 0, total: node.size || 0 };

    if (job.status === 'completed') {
      const size = job.total_bytes || node.size || 0;
      return { status: 'completed', percent: 100, downloaded: size, total: size };
    }

    if (['downloading', 'resolving', 'verifying'].includes(job.status)) {
      const total = (job.total_bytes && job.total_bytes > 0) ? job.total_bytes : (node.size || 0);
      const downloaded = job.downloaded_bytes || 0;
      const percent = total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : 0;
      return { status: 'downloading', percent, downloaded, total };
    }

    if (job.status === 'queued') {
      return { status: 'queued', percent: 0, downloaded: 0, total: node.size || 0 };
    }

    if (job.status === 'paused') {
      const total = (job.total_bytes && job.total_bytes > 0) ? job.total_bytes : (node.size || 0);
      const downloaded = job.downloaded_bytes || 0;
      const percent = total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : 0;
      return { status: 'paused', percent, downloaded, total };
    }

    if (job.status === 'failed') {
      return { status: 'failed', percent: 0, downloaded: 0, total: node.size || 0 };
    }

    return { status: 'none', percent: 0, downloaded: 0, total: node.size || 0 };
  }

  function resolveStreamUrl(node: CloudNode): string {
    const localJob = getNodeDownloadJob(node);
    const port = serverPortState.port || 0;
    if (localJob?.status === 'completed' && localJob.final_path) {
      if (port > 0) {
        const encoded = localJob.final_path.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
        return `http://127.0.0.1:${port}/media/${encoded}`;
      }
      return convertFileSrc(localJob.final_path);
    }

    if (node.stream_url?.startsWith('/cloud_stream/') && port > 0) {
      return `http://127.0.0.1:${port}${node.stream_url}`;
    }
    if (node.download_url?.startsWith('/cloud_stream/') && port > 0) {
      return `http://127.0.0.1:${port}${node.download_url}`;
    }
    const rawUrl = node.stream_url || node.download_url || '';
    if (
      port > 0 &&
      rawUrl &&
      (rawUrl.includes('dropbox.com') ||
        rawUrl.includes('pixeldrain.com') ||
        rawUrl.includes('drive.google.com') ||
        rawUrl.includes('dropboxusercontent.com'))
    ) {
      return `http://127.0.0.1:${port}/cloud_stream/proxy?url=${encodeURIComponent(rawUrl)}&name=${encodeURIComponent(node.name)}`;
    }
    return rawUrl;
  }

  function resolveDownloadUrl(node: CloudNode): string {
    const raw = node.download_url || node.stream_url || '';
    const port = serverPortState.port || 0;
    if (raw.startsWith('/cloud_stream/') && port > 0) {
      return `http://127.0.0.1:${port}${raw}`;
    }
    return raw;
  }

  let currentItems = $derived.by(() => {
    if (!folder?.nodes || folder.nodes.length === 0) return [];
    const list = folder.nodes;

    let itemsInDir = list.filter((n) => {
      if (currentFolderId === null) {
        return !n.parent_id || !list.some((p) => p.id === n.parent_id);
      }
      return n.parent_id === currentFolderId;
    });

    if (itemsInDir.length === 0 && currentFolderId === null) {
      itemsInDir = list;
    }

    const folders = itemsInDir.filter((n) => n.is_folder).sort((a, b) => a.name.localeCompare(b.name));
    const files = itemsInDir.filter((n) => !n.is_folder).sort((a, b) => a.name.localeCompare(b.name));

    return [...folders, ...files];
  });

  let mediaNodes = $derived.by(() => {
    if (!folder?.nodes) return [];
    return folder.nodes.filter((n) => !n.is_folder && isMedia(n));
  });

  let mediaViewerItems = $derived.by(() => {
    return mediaNodes.map((n) => {
      const ext = n.name.split('.').pop()?.toLowerCase() || '';
      const mime = (n.mime_type || '').toLowerCase();
      const kind: 'image' | 'video' | 'audio' | 'file' =
        mime.startsWith('video/') || ['mp4', 'mkv', 'webm', 'mov'].includes(ext)
          ? 'video'
          : mime.startsWith('audio/') || ['mp3', 'wav', 'flac', 'ogg', 'm4a'].includes(ext)
          ? 'audio'
          : 'image';
      return {
        id: n.id,
        url: resolveStreamUrl(n),
        name: n.name,
        kind,
        size: n.size || undefined
      };
    });
  });

  function getNodeIcon(node: CloudNode) {
    if (node.is_folder) return IconFolder;
    const ext = node.name.split('.').pop()?.toLowerCase() || '';
    const mime = (node.mime_type || '').toLowerCase();

    if (mime.startsWith('image/') || ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg'].includes(ext)) {
      return IconImage;
    }
    if (mime.startsWith('video/') || ['mp4', 'mkv', 'webm', 'mov', 'avi', 'wmv'].includes(ext)) {
      return IconVideo;
    }
    if (['zip', 'rar', '7z', 'tar', 'gz', 'bz2'].includes(ext) || mime.includes('zip') || mime.includes('compressed')) {
      return IconZip;
    }
    return IconDocument;
  }

  function navigateIntoFolder(node: CloudNode) {
    currentFolderId = node.id;
    history = [...history, { id: node.id, name: node.name }];
  }

  function navigateUp() {
    if (history.length <= 1) return;
    const newHistory = history.slice(0, -1);
    const parent = newHistory[newHistory.length - 1];
    history = newHistory;
    currentFolderId = parent.id;
  }

  function navigateToBreadcrumb(index: number) {
    if (index < 0 || index >= history.length) return;
    const target = history[index];
    history = history.slice(0, index + 1);
    currentFolderId = target.id;
  }

  let allCurrentFiles = $derived.by(() => {
    return currentItems.filter((n) => !n.is_folder);
  });

  let selectedFiles = $derived.by(() => {
    if (!folder?.nodes || selectedIds.size === 0) return [];
    return folder.nodes.filter((n) => !n.is_folder && selectedIds.has(n.id));
  });

  let selectedTotalBytes = $derived.by(() => {
    return selectedFiles.reduce((sum, f) => sum + (f.size || 0), 0);
  });

  let isAllCurrentSelected = $derived.by(() => {
    if (allCurrentFiles.length === 0) return false;
    return allCurrentFiles.every((n) => selectedIds.has(n.id));
  });

  function clearSelection() {
    selectedIds = new Set();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && selectedIds.size > 0) {
      e.stopPropagation();
      e.preventDefault();
      clearSelection();
    }
  }

  function toggleSelect(node: CloudNode) {
    const next = new Set(selectedIds);
    if (node.is_folder) {
      const descendants = getAllDescendantFiles(node.id);
      const allSelected = descendants.length > 0 && descendants.every((d) => next.has(d.id));
      if (allSelected) {
        descendants.forEach((d) => next.delete(d.id));
      } else {
        descendants.forEach((d) => next.add(d.id));
      }
    } else {
      if (next.has(node.id)) next.delete(node.id);
      else next.add(node.id);
    }
    selectedIds = next;
  }

  function toggleSelectAll() {
    if (!folder?.nodes) return;
    if (isAllCurrentSelected) {
      const next = new Set(selectedIds);
      allCurrentFiles.forEach((n) => next.delete(n.id));
      selectedIds = next;
    } else {
      const next = new Set(selectedIds);
      allCurrentFiles.forEach((n) => next.add(n.id));
      selectedIds = next;
    }
  }

  function getDownloadPost(): Post {
    if (post) return post;
    return {
      id: folder?.title || 'cloud_download',
      service: folder?.provider || 'cloud',
      user: 'cloud',
      title: folder?.title || 'Cloud Download',
      content: folder?.url || ''
    };
  }

  async function downloadSelected() {
    if (!folder || selectedFiles.length === 0) return;
    const targetPost = getDownloadPost();

    // Filter out files that are already completed or currently downloading
    const filesToDownload = selectedFiles.filter((f) => {
      const job = getNodeDownloadJob(f);
      if (job?.status === 'completed') return false;
      if (job && ['downloading', 'resolving', 'verifying', 'queued'].includes(job.status)) return false;
      return true;
    });

    if (filesToDownload.length === 0) {
      toast.info(i18n.t('downloads.all_downloaded') || 'All selected files are already downloaded');
      clearSelection();
      return;
    }

    let started = 0;
    for (const f of filesToDownload) {
      const dlUrl = resolveDownloadUrl(f);
      if (!dlUrl) continue;
      try {
        await apiStartDownload(targetPost, f.id, dlUrl, f.name);
        started++;
      } catch {
        // ignore
      }
    }

    if (started > 0) {
      toast.success(
        i18n.t('feed.download_started') || 'Download started',
        { description: `${started} ${i18n.t('selection.items_count') || 'files added to queue'}` }
      );
    }
    clearSelection();
  }

  async function downloadSingle(node: CloudNode) {
    if (!folder) return;
    const targetPost = getDownloadPost();

    if (node.is_folder) {
      const files = getAllDescendantFiles(node.id);
      if (files.length === 0) {
        toast.error('Folder is empty');
        return;
      }

      // Filter out files that are already completed or currently downloading
      const filesToDownload = files.filter((f) => {
        const job = getNodeDownloadJob(f);
        if (job?.status === 'completed') return false;
        if (job && ['downloading', 'resolving', 'verifying', 'queued'].includes(job.status)) return false;
        return true;
      });

      if (filesToDownload.length === 0) {
        toast.info(i18n.t('downloads.all_downloaded') || 'All files in folder are already downloaded');
        return;
      }

      let started = 0;
      for (const f of filesToDownload) {
        const dlUrl = resolveDownloadUrl(f);
        if (!dlUrl) continue;
        try {
          await apiStartDownload(targetPost, f.id, dlUrl, f.name);
          started++;
        } catch {
          // ignore
        }
      }

      if (started > 0) {
        toast.success(
          i18n.t('feed.download_started') || 'Download started',
          { description: `${started} files from "${node.name}" added to queue` }
        );
      }
      return;
    }

    const job = getNodeDownloadJob(node);
    if (job?.status === 'completed') {
      toast.info(i18n.t('downloads.already_downloaded') || 'File is already downloaded');
      return;
    }
    if (job && ['downloading', 'resolving', 'verifying', 'queued'].includes(job.status)) {
      toast.info(i18n.t('downloads.already_in_progress') || 'Download is already in progress');
      return;
    }

    const dlUrl = resolveDownloadUrl(node);
    if (!dlUrl) return;

    try {
      await apiStartDownload(targetPost, node.id, dlUrl, node.name);
      toast.success(i18n.t('feed.download_started') || 'Download started', { description: node.name });
    } catch {
      // ignore
    }
  }

  function openMediaPreview(node: CloudNode) {
    const idx = mediaNodes.findIndex((n) => n.id === node.id);
    if (idx >= 0) {
      previewIndex = idx;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<Modal
  isOpen={open}
  title={folder?.title || 'Cloud Folder'}
  {onclose}
  size="2xl"
  position="top"
  fixedHeight={true}
  flush={true}
  borderlessHeader={true}
  scrollable={false}
>
  {#if folder}
    <div class="mega-explorer flex flex-col w-full h-full text-[var(--fg-default)] select-none">
      
      <!-- Breadcrumbs Bar (Fixed 44px height to prevent any layout shifts) -->
      <div class="mega-breadcrumbs h-11 min-h-[44px] max-h-[44px] flex items-center gap-1 px-4 text-[13px] text-[var(--fg-muted)] bg-transparent overflow-x-auto whitespace-nowrap shrink-0 border-b border-white/[0.04]">
        {#if history.length > 1}
          <button
            type="button"
            class="w-7 h-7 flex items-center justify-center rounded-lg hover:bg-white/10 text-[var(--fg-default)] transition-colors mr-1 shrink-0"
            onclick={navigateUp}
            title="Go up one folder"
          >
            <IconChevronLeft class="w-[18px] h-[18px]" />
          </button>
        {/if}

        {#each history as step, idx}
          {@const isLast = idx === history.length - 1}
          <button
            type="button"
            class="breadcrumb-link inline-flex items-center h-7 px-1.5 rounded transition-colors {isLast ? 'font-semibold text-[var(--fg-default)] cursor-default' : 'font-normal hover:text-[var(--fg-default)] hover:underline'}"
            onclick={() => !isLast && navigateToBreadcrumb(idx)}
            disabled={isLast}
          >
            {step.name}
          </button>
          {#if !isLast}
            <IconChevronRight class="w-3.5 h-3.5 text-[var(--fg-muted)]/40 shrink-0 mx-0.5" />
          {/if}
        {/each}
      </div>

      <!-- MEGA Style Full-Width Table (flex-1 with fixed scroll container) -->
      <div class="mega-table-container flex-1 min-h-0 relative overflow-hidden w-full bg-transparent" use:scrollable>
        <table class="mega-table w-full min-w-[500px] border-collapse text-left bg-transparent">
          <thead class="bg-transparent">
            <tr class="h-9 border-b border-white/[0.06] border-t-0 text-[11px] font-semibold uppercase tracking-wider text-[var(--fg-muted)]/80 sticky top-0 z-10 select-none bg-transparent backdrop-blur-sm">
              <th class="py-2 px-4 w-10 text-center font-normal bg-transparent">
                <Checkbox
                  checked={isAllCurrentSelected}
                  onchange={toggleSelectAll}
                />
              </th>
              <th class="py-2 px-3 font-semibold bg-transparent">{i18n.t('downloads.name')}</th>
              <th class="py-2 px-3 font-semibold w-28 bg-transparent">{i18n.t('downloads.type')}</th>
              <th class="py-2 px-4 font-semibold w-28 text-right bg-transparent">{i18n.t('downloads.size')}</th>
              <th class="py-2 px-4 w-[124px] text-right bg-transparent font-normal"></th>
            </tr>
          </thead>
          <tbody class="divide-y divide-white/[0.04] bg-transparent text-xs">
            {#if currentItems.length === 0}
              <tr>
                <td colspan="5" class="py-20 text-center text-xs text-[var(--fg-muted)]">
                  This folder is empty
                </td>
              </tr>
            {:else}
              {#each currentItems as node (node.id)}
                {@const IconComp = getNodeIcon(node)}
                {@const isFolder = node.is_folder}
                {@const isSelected = isFolder ? (getAllDescendantFiles(node.id).length > 0 && getAllDescendantFiles(node.id).every((d) => selectedIds.has(d.id))) : selectedIds.has(node.id)}
                {@const typeLabel = getNodeTypeLabel(node)}
                {@const displaySize = isFolder ? getFolderTotalSize(node.id) : (node.size || 0)}
                {@const media = !isFolder && isMedia(node)}
                {@const progress = getNodeProgress(node)}
                {@const isDownloading = progress.status === 'downloading' || progress.status === 'queued'}
                {@const isCompleted = progress.status === 'completed'}

                <tr
                  class="mega-row h-[42px] transition-colors cursor-pointer select-none group even:bg-white/[0.015] hover:bg-white/[0.04] {isSelected ? 'bg-[var(--accent)]/10' : ''} {isCompleted ? 'is-completed' : ''} {isDownloading ? 'is-downloading' : ''}"
                  style={isDownloading && progress.percent > 0 ? `--row-progress: ${progress.percent}%;` : ''}
                  onclick={() => isFolder ? navigateIntoFolder(node) : toggleSelect(node)}
                >
                  <!-- Checkbox Column -->
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <td class="py-2 px-4 text-center relative z-[1]" onclick={(e) => e.stopPropagation()}>
                    <Checkbox
                      checked={isSelected}
                      onchange={() => toggleSelect(node)}
                    />
                  </td>

                  <!-- Name Column with Icon -->
                  <td class="py-2 px-3 min-w-0 relative z-[1]">
                    <div class="flex items-center gap-3 min-w-0">
                      <div class="flex items-center justify-center w-6 h-6 shrink-0">
                        <IconComp class="w-5 h-5 {isFolder ? 'text-[var(--accent)]' : (isCompleted ? 'text-emerald-400' : 'text-[var(--fg-muted)]')}" />
                      </div>
                      <span
                        class="text-[13.5px] truncate {isFolder ? 'font-medium text-[var(--fg-default)]' : 'font-normal text-[var(--fg-default)]/90'}"
                        title={node.name}
                      >
                        {node.name}
                      </span>
                    </div>
                  </td>

                  <!-- Type Column -->
                  <td class="py-2 px-3 text-[var(--fg-muted)] text-[12px] font-normal relative z-[1]">
                    {typeLabel}
                  </td>

                  <!-- Size Column (for both Files and Folders) -->
                  <td class="py-2 px-4 text-right text-[12px] font-mono whitespace-nowrap relative z-[1]">
                    {#if isDownloading}
                      <span class="text-[var(--accent)] font-medium">
                        {progress.percent > 0 ? `${progress.percent}%` : (i18n.t('downloads.status_queued') || 'Queued')}
                      </span>
                    {:else if isCompleted}
                      <span class="text-emerald-400 font-medium inline-flex items-center gap-1 justify-end">
                        {displaySize > 0 ? formatBytes(displaySize) : '—'}
                      </span>
                    {:else}
                      <span class="text-[var(--fg-muted)]">
                        {displaySize > 0 ? formatBytes(displaySize) : '—'}
                      </span>
                    {/if}
                  </td>

                  <!-- Actions Column (Fixed 3-icon slot width to prevent layout shifts) -->
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <td class="py-1 px-4 text-right whitespace-nowrap w-[124px] relative z-[1]" onclick={(e) => e.stopPropagation()}>
                    <div class="inline-flex items-center justify-end gap-1.5 w-[104px]">
                      {#if media}
                        <button
                          type="button"
                          class="action-btn w-8 h-8 flex items-center justify-center rounded-lg text-[var(--fg-muted)] hover:text-[var(--fg-default)] hover:bg-white/10 transition-all shrink-0"
                          onclick={() => openMediaPreview(node)}
                          title={i18n.t('post.viewer_open') || 'Preview'}
                        >
                          <IconEye class="w-[18px] h-[18px]" />
                        </button>
                      {/if}

                      {#if !isFolder && node.download_url}
                        <a
                          href={resolveStreamUrl(node)}
                          target="_blank"
                          rel="noopener noreferrer"
                          class="action-btn w-8 h-8 flex items-center justify-center rounded-lg text-[var(--fg-muted)] hover:text-[var(--fg-default)] hover:bg-white/10 transition-all inline-flex shrink-0"
                          title={i18n.t('post.open_link') || 'Open direct stream'}
                        >
                          <IconOpen class="w-[18px] h-[18px]" />
                        </a>
                      {/if}

                      {#if isCompleted}
                        <button
                          type="button"
                          class="action-btn w-8 h-8 flex items-center justify-center rounded-lg text-emerald-400 hover:bg-emerald-400/10 transition-all shrink-0"
                          onclick={() => isFolder ? navigateIntoFolder(node) : (media ? openMediaPreview(node) : downloadSingle(node))}
                          title={i18n.t('downloads.completed') || 'Downloaded'}
                        >
                          <IconCheckmark class="w-[18px] h-[18px]" />
                        </button>
                      {:else if isDownloading}
                        <div
                          class="w-8 h-8 flex items-center justify-center text-[var(--accent)] shrink-0"
                          title={`${i18n.t('downloads.status_downloading') || 'Downloading'}: ${progress.percent}%`}
                        >
                          <IconLoading class="w-[18px] h-[18px]" />
                        </div>
                      {:else}
                        <button
                          type="button"
                          class="action-btn w-8 h-8 flex items-center justify-center rounded-lg text-[var(--fg-muted)] hover:text-[var(--accent)] hover:bg-[var(--accent)]/10 transition-all shrink-0"
                          onclick={() => downloadSingle(node)}
                          title={i18n.t('post.download') || 'Download'}
                        >
                          <IconDownload class="w-[18px] h-[18px]" />
                        </button>
                      {/if}
                    </div>
                  </td>
                </tr>
              {/each}

              {#if selectedIds.size > 0}
                <tr class="h-20 pointer-events-none border-none">
                  <td colspan="5" class="h-20 p-0 border-none bg-transparent"></td>
                </tr>
              {/if}
            {/if}
          </tbody>
        </table>
      </div>

    </div>
  {/if}

  {#snippet floating()}
    {#if selectedIds.size > 0}
      <aside class="modal-selection-dock-wrapper" aria-label="Selection actions">
        <div class="selection-dock" role="toolbar">
          <div class="selection-counter">
            <span class="selection-count-badge active">
              {selectedFiles.length}
            </span>
            <span class="selection-count-label">
              {selectedTotalBytes > 0 ? formatBytes(selectedTotalBytes) : (i18n.t('selection.items_count') || 'selected')}
            </span>
          </div>

          <div class="selection-dock-divider"></div>

          {#if allCurrentFiles.length > 0}
            <button
              type="button"
              class="selection-dock-btn"
              use:ripple
              onclick={toggleSelectAll}
              aria-label={i18n.t(isAllCurrentSelected ? 'selection.deselect_all' : 'selection.select_all')}
            >
              <IconSelectAll class="w-[17px] h-[17px]" />
              <span>{i18n.t(isAllCurrentSelected ? 'selection.deselect_all' : 'selection.select_all') || (isAllCurrentSelected ? 'Deselect all' : 'Select all')}</span>
            </button>

            <div class="selection-dock-divider"></div>
          {/if}

          <button
            type="button"
            class="selection-dock-btn btn-accent"
            use:ripple
            onclick={downloadSelected}
          >
            <IconDownload class="w-[17px] h-[17px]" />
            <span>{i18n.t('post.download') || 'Download'} ({selectedFiles.length})</span>
          </button>

          <div class="selection-dock-divider"></div>

          <button
            type="button"
            class="selection-dock-close-btn"
            use:ripple
            onclick={clearSelection}
            use:tooltip={`${i18n.t('selection.cancel') || 'Cancel'} (Esc)`}
            aria-label="Cancel selection"
          >
            <IconDismiss class="w-[18px] h-[18px]" />
          </button>
        </div>
      </aside>
    {/if}
  {/snippet}
</Modal>

{#if previewIndex !== null && mediaViewerItems.length > 0}
  <MediaViewer
    items={mediaViewerItems}
    initialIndex={previewIndex}
    onclose={() => previewIndex = null}
  />
{/if}

<style>
  .mega-explorer {
    font-family: var(--font-sans);
  }

  .mega-breadcrumbs {
    scrollbar-width: none;
    -ms-overflow-style: none;
  }

  .mega-breadcrumbs::-webkit-scrollbar {
    display: none;
  }

  .mega-table {
    border-radius: 0;
  }

  .mega-row {
    position: relative;
  }

  .mega-row.is-completed {
    background: rgba(16, 185, 129, 0.02);
  }

  .mega-row.is-completed:hover {
    background: rgba(16, 185, 129, 0.05);
  }

  .mega-row.is-downloading {
    background-image: linear-gradient(
      to right,
      rgba(255, 255, 255, 0.08) 0%,
      rgba(255, 255, 255, 0.08) var(--row-progress, 0%),
      transparent var(--row-progress, 0%),
      transparent 100%
    );
  }

  .action-btn {
    cursor: pointer;
  }

  .modal-selection-dock-wrapper {
    position: absolute;
    bottom: 16px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 40;
    pointer-events: none;
    display: flex;
    justify-content: center;
    max-width: calc(100% - 32px);
  }

  .selection-dock {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 52px;
    padding: 0 8px;
    border-radius: 20px;
    background: rgba(16, 17, 22, 0.92);
    border: 1px solid var(--border-color);
    box-shadow: 0 16px 44px rgba(0, 0, 0, 0.55), 0 0 0 1px rgba(255, 255, 255, 0.05);
    backdrop-filter: blur(24px) saturate(1.6);
    -webkit-backdrop-filter: blur(24px) saturate(1.6);
    animation: floatingDockIn 200ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
    white-space: nowrap;
    overflow-x: auto;
    scrollbar-width: none;
    max-width: 100%;
  }

  .selection-dock::-webkit-scrollbar {
    display: none;
  }

  @keyframes floatingDockIn {
    0% {
      opacity: 0;
      transform: translateY(16px) scale(0.96);
    }
    100% {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .selection-counter {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 10px 0 6px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.06);
    flex-shrink: 0;
  }

  .selection-count-badge {
    min-width: 22px;
    height: 22px;
    padding: 0 6px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.12);
    color: rgba(255, 255, 255, 0.7);
    font-size: 11.5px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .selection-count-badge.active {
    background: var(--accent-primary);
    color: var(--accent-text, #ffffff);
  }

  .selection-count-label {
    font-size: 12.5px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    letter-spacing: -0.01em;
  }

  .selection-dock-divider {
    width: 1px;
    height: 20px;
    background: rgba(255, 255, 255, 0.08);
    margin: 0 2px;
    flex-shrink: 0;
  }

  .selection-dock-btn {
    height: 36px;
    padding: 0 12px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.05);
    border: none;
    color: rgba(255, 255, 255, 0.8);
    font-size: 12.5px;
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo),
                transform var(--duration-fast) var(--ease-expo);
    flex-shrink: 0;
  }

  .selection-dock-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #ffffff;
  }

  .selection-dock-btn:active {
    transform: scale(0.97);
  }

  .selection-dock-btn.btn-accent {
    background: var(--accent-primary);
    color: var(--accent-text, #ffffff);
  }

  .selection-dock-btn.btn-accent:hover {
    background: var(--accent-hover);
    color: #ffffff;
  }

  .selection-dock-close-btn {
    width: 36px;
    height: 36px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.05);
    border: none;
    color: rgba(255, 255, 255, 0.6);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo),
                transform var(--duration-fast) var(--ease-expo);
    flex-shrink: 0;
  }

  .selection-dock-close-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #ffffff;
  }

  .selection-dock-close-btn:active {
    transform: scale(0.95);
  }
</style>
