<script lang="ts">
  import type { CloudFolderResult } from '$lib/types/cloud';
  import { apiResolveCloudLink } from '$lib/utils/ipc';
  import { formatBytes } from '$lib/utils/formatters';
  import CloudFolderModal from './CloudFolderModal.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import CountBadge from '$lib/components/ui/CountBadge.svelte';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconFolder from '~icons/fluent/folder-open-24-regular';
  import IconSpinner from '~icons/fluent/spinner-ios-20-regular';
  import IconCloud from '~icons/fluent/cloud-24-regular';

  let {
    url
  }: {
    url: string;
  } = $props();

  let loading = $state(false);
  let resolvedData = $state<CloudFolderResult | null>(null);
  let error = $state<string | null>(null);
  let modalOpen = $state(false);

  let providerName = $derived.by(() => {
    const u = (url || '').toLowerCase();
    if (u.includes('mega.nz') || u.includes('mega.co.nz')) return 'MEGA';
    if (u.includes('pixeldrain.com')) return 'Pixeldrain';
    if (u.includes('dropbox.com')) return 'Dropbox';
    if (u.includes('drive.google.com')) return 'Google Drive';
    if (u.includes('iframely.net') || u.includes('iframe.ly')) return 'Cloud Embed';
    return 'Cloud';
  });

  let providerBadgeClass = $derived.by(() => {
    const p = providerName.toLowerCase();
    if (p === 'mega') return 'bg-red-500/15 text-red-400';
    if (p === 'dropbox') return 'bg-blue-500/15 text-blue-400';
    if (p === 'pixeldrain') return 'bg-purple-500/15 text-purple-400';
    return 'bg-[var(--accent)]/15 text-[var(--accent)]';
  });

  async function resolveLink(openModalAfter = false) {
    if (loading) return;
    if (resolvedData) {
      if (openModalAfter) modalOpen = true;
      return;
    }
    loading = true;
    error = null;
    try {
      const res = await apiResolveCloudLink(url);
      resolvedData = res;
      if (openModalAfter) modalOpen = true;
    } catch (err: any) {
      error = typeof err === 'string' ? err : err?.message || 'Failed to resolve cloud link';
      console.warn('Cloud link resolution failed:', err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (url && !resolvedData && !loading && !error) {
      void resolveLink(false);
    }
  });
</script>

<div class="cloud-link-card my-3 flex items-center justify-between gap-3 p-3.5 rounded-2xl bg-[var(--bg-card)] hover:bg-[var(--bg-card-hover)] transition-all shadow-sm">
  <div class="flex items-center gap-3 min-w-0">
    <div class="provider-pill flex items-center justify-center px-3 py-1 rounded-full text-xs font-semibold uppercase tracking-wider {providerBadgeClass} shrink-0">
      <IconCloud class="w-3.5 h-3.5 mr-1.5" />
      <span>{providerName}</span>
    </div>

    <div class="min-w-0 flex flex-col justify-center">
      {#if resolvedData}
        <div class="text-[13px] font-medium text-[var(--fg-default)] truncate">
          {resolvedData.title}
        </div>
        <div class="text-[11.5px] text-[var(--fg-muted)] flex items-center gap-1.5 mt-0.5">
          <CountBadge count={resolvedData.total_files} variant="pill" />
          {#if resolvedData.total_size > 0}
            <span>•</span>
            <span>{formatBytes(resolvedData.total_size)}</span>
          {/if}
        </div>
      {:else}
        <div class="text-[13px] font-medium text-[var(--fg-default)] truncate max-w-[320px]">
          {url}
        </div>
        {#if error}
          <div class="text-[11px] text-[var(--danger,red)] truncate mt-0.5">
            {error}
          </div>
        {:else}
          <div class="text-[11.5px] text-[var(--fg-muted)] mt-0.5">
            {loading ? 'Inspecting files...' : 'Click to inspect files'}
          </div>
        {/if}
      {/if}
    </div>
  </div>

  <div class="actions flex items-center gap-2 shrink-0">
    <Button
      variant={resolvedData ? 'accent' : 'primary'}
      size="sm"
      onclick={() => resolveLink(true)}
      disabled={loading}
    >
      {#if loading}
        <IconSpinner class="w-3.5 h-3.5 mr-1.5 animate-spin" />
        <span>Loading...</span>
      {:else}
        <IconFolder class="w-3.5 h-3.5 mr-1.5" />
        <span>{resolvedData ? 'Browse' : 'Inspect'}</span>
      {/if}
    </Button>

    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      class="flex items-center justify-center w-8 h-8 rounded-full text-[var(--fg-muted)] hover:text-[var(--fg-default)] hover:bg-white/10 transition-colors"
      title="Open in browser"
    >
      <IconOpen class="w-4 h-4" />
    </a>
  </div>
</div>

<CloudFolderModal
  folder={resolvedData}
  open={modalOpen}
  onclose={() => modalOpen = false}
/>
