<script lang="ts">
  import { i18n } from '$lib/i18n';
  import type { CacheStats } from '$lib/utils/ipc';

  interface Props {
    stats: CacheStats | null;
    limitMb: number;
    formatBytes: (bytes: number) => string;
  }

  let { stats, limitMb, formatBytes }: Props = $props();

  let totalUsed = $derived((stats?.total_bytes ?? 0) + (stats?.metadata_bytes ?? 0));
  let maxBytes = $derived(Math.max(1, limitMb * 1024 * 1024));
  let usedPercent = $derived(Math.min(100, Math.max(0, (totalUsed / maxBytes) * 100)));

  let categories = $derived.by(() => {
    if (!stats || totalUsed === 0) return [];

    const items = [
      {
        id: 'previews',
        label: i18n.t('settings.cache_previews'),
        bytes: stats.preview_bytes,
        color: '#a855f7',
        bgClass: 'bg-purple-500'
      },
      {
        id: 'avatars',
        label: i18n.t('settings.cache_avatars'),
        bytes: stats.avatar_bytes,
        color: '#38bdf8',
        bgClass: 'bg-sky-400'
      },
      {
        id: 'banners',
        label: i18n.t('settings.cache_banners'),
        bytes: stats.banner_bytes,
        color: '#34d399',
        bgClass: 'bg-emerald-400'
      },
      {
        id: 'metadata',
        label: i18n.t('settings.cache_metadata'),
        bytes: stats.metadata_bytes,
        color: '#818cf8',
        bgClass: 'bg-indigo-400'
      },
      {
        id: 'other',
        label: i18n.t('settings.cache_other'),
        bytes: stats.other_bytes,
        color: '#94a3b8',
        bgClass: 'bg-slate-400'
      }
    ];

    return items
      .filter((it) => it.bytes > 0)
      .map((it) => ({
        ...it,
        shareOfUsed: (it.bytes / totalUsed) * 100,
        shareOfTotal: (it.bytes / maxBytes) * 100
      }));
  });
</script>

<div class="flex flex-col gap-3 w-full">
  <div class="flex items-baseline justify-between gap-2">
    <div class="flex items-baseline gap-2">
      <span class="text-base font-semibold font-outfit text-white tracking-tight">
        {formatBytes(totalUsed)}
      </span>
      <span class="text-xs text-white/40 font-mono">
        / {limitMb} MB
      </span>
      {#if stats?.file_count}
        <span class="text-[11px] text-white/30 font-medium">
          ({stats.file_count} {i18n.t('settings.cache_files').toLowerCase()})
        </span>
      {/if}
    </div>

    <span class="text-xs font-mono font-medium {usedPercent > 85 ? 'text-amber-400' : 'text-white/50'}">
      {usedPercent.toFixed(1)}%
    </span>
  </div>

  <div class="storage-bar-track" title="{formatBytes(totalUsed)} / {limitMb} MB">
    {#if categories.length === 0}
      <div class="h-full w-full bg-white/[0.04]"></div>
    {:else}
      <div class="flex h-full" style="width: {usedPercent}%;">
        {#each categories as cat (cat.id)}
          <div
            class="storage-bar-segment"
            style="width: {cat.shareOfUsed}%; background-color: {cat.color};"
            title="{cat.label}: {formatBytes(cat.bytes)} ({cat.shareOfUsed.toFixed(1)}%)"
          ></div>
        {/each}
      </div>
    {/if}
  </div>

  <div class="flex flex-wrap items-center gap-x-4 gap-y-1.5 pt-0.5">
    {#each categories as cat (cat.id)}
      <div class="flex items-center gap-1.5 text-[11.5px] text-white/70">
        <span class="w-2 h-2 rounded-full shrink-0" style="background-color: {cat.color};"></span>
        <span class="text-white/40">{cat.label}:</span>
        <span class="font-mono text-white/90">{formatBytes(cat.bytes)}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .storage-bar-track {
    width: 100%;
    height: 9px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.07);
    overflow: hidden;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.4);
    display: flex;
  }

  .storage-bar-segment {
    height: 100%;
    transition: width 350ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .storage-bar-segment:first-child {
    border-top-left-radius: 9999px;
    border-bottom-left-radius: 9999px;
  }

  .storage-bar-segment:last-child {
    border-top-right-radius: 9999px;
    border-bottom-right-radius: 9999px;
  }
</style>
