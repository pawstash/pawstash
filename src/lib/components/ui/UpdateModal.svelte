<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { updateState } from '$lib/state/updateState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconTag from '~icons/fluent/tag-24-regular';
  import IconCalendar from '~icons/fluent/calendar-ltr-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';

  function formatPublishedDate(dateStr: string): string {
    if (!dateStr) return '';
    try {
      const d = new Date(dateStr);
      return d.toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric'
      });
    } catch {
      return dateStr;
    }
  }

  const changelogLines = $derived.by(() => {
    const raw = updateState.info?.release_notes?.trim();
    if (!raw) return [];
    return raw
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean);
  });
</script>

{#if updateState.modalOpen && updateState.info}
  <Modal
    isOpen={updateState.modalOpen}
    title={i18n.t('update_modal_title')}
    onclose={() => updateState.closeModal()}
  >
    <div class="flex flex-col gap-4 w-full text-white">
      <!-- Release Info Card -->
      <div class="relative overflow-hidden flex items-start gap-3.5 p-4 rounded-xl bg-white/[0.03] border border-white/[0.08] shadow-sm">
        <div class="flex items-center justify-center w-10 h-10 rounded-lg bg-[var(--accent-glow)]/20 text-[var(--accent)] shrink-0 border border-white/[0.08]">
          <IconSparkle class="w-5 h-5" />
        </div>

        <div class="flex flex-col gap-1.5 min-w-0 flex-1">
          <div class="flex items-center gap-2 flex-wrap">
            <h3 class="text-base font-semibold font-outfit text-white tracking-tight">
              {updateState.info.release_name || `v${updateState.info.latest_version}`}
            </h3>
            {#if updateState.info.is_prerelease}
              <span class="text-[10px] font-semibold uppercase tracking-wider px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-300 border border-amber-500/30">
                {i18n.t('update_badge_prerelease')}
              </span>
            {/if}
          </div>

          <div class="flex items-center gap-3 text-xs text-white/60 flex-wrap">
            <div class="flex items-center gap-1.5 font-mono text-[11px] text-white/80 bg-white/[0.04] px-2 py-0.5 rounded-md border border-white/[0.06]">
              <IconTag class="w-3 h-3 text-white/40" />
              <span>v{updateState.info.current_version}</span>
              <span class="text-white/40">&rarr;</span>
              <span class="text-[var(--accent)] font-semibold">v{updateState.info.latest_version}</span>
            </div>

            {#if updateState.info.published_at}
              <div class="flex items-center gap-1 text-[11px] text-white/50">
                <IconCalendar class="w-3 h-3 text-white/40" />
                <span>{formatPublishedDate(updateState.info.published_at)}</span>
              </div>
            {/if}

            {#if updateState.info.asset_size}
              <span class="font-mono text-[11px] text-white/50">
                {formatBytes(updateState.info.asset_size)}
              </span>
            {/if}
          </div>
        </div>
      </div>

      <!-- Changelog Section -->
      <div class="flex flex-col gap-2">
        <span class="text-[11px] font-semibold uppercase tracking-wider text-white/50 font-outfit">
          {i18n.t('update_changelog_label')}
        </span>

        <div class="max-h-48 overflow-y-auto rounded-xl bg-black/40 border border-white/[0.06] p-3.5 custom-scrollbar">
          {#if changelogLines.length > 0}
            <ul class="flex flex-col gap-2 text-xs leading-relaxed text-white/80">
              {#each changelogLines as line}
                {#if line.startsWith('###') || line.startsWith('##')}
                  <li class="font-semibold text-white/90 pt-1 font-outfit text-xs border-b border-white/[0.06] pb-1">
                    {line.replace(/^#+\s*/, '')}
                  </li>
                {:else if line.startsWith('-') || line.startsWith('*')}
                  <li class="flex items-start gap-2 pl-1">
                    <span class="w-1.5 h-1.5 rounded-full bg-[var(--accent)] shrink-0 mt-1.5"></span>
                    <span class="flex-1 font-sans">{line.replace(/^[-*]\s*/, '')}</span>
                  </li>
                {:else}
                  <li class="text-white/70 font-sans">{line}</li>
                {/if}
              {/each}
            </ul>
          {:else}
            <p class="text-xs text-white/40 italic py-2 text-center">
              {i18n.t('update_no_changelog')}
            </p>
          {/if}
        </div>
      </div>

      <!-- Download Progress Bar (When downloading) -->
      {#if updateState.downloading}
        <div class="flex flex-col gap-2 p-3.5 rounded-xl bg-white/[0.03] border border-white/[0.08]">
          <div class="flex items-baseline justify-between gap-2">
            <span class="text-xs font-medium text-white/90">
              {#if updateState.installing}
                {i18n.t('update_installing')}
              {:else}
                {i18n.t('update_downloading')}
              {/if}
            </span>
            <span class="font-mono text-xs font-semibold text-[var(--accent)]">
              {updateState.downloadProgress}%
            </span>
          </div>

          <div class="w-full h-2 rounded-full bg-white/[0.07] overflow-hidden">
            <div
              class="h-full bg-[var(--accent)] transition-all duration-200 rounded-full"
              style="width: {updateState.downloadProgress}%; box-shadow: 0 0 10px var(--accent-glow);"
            ></div>
          </div>

          <div class="flex items-center justify-between text-[11px] font-mono text-white/50">
            <span>
              {#if updateState.totalBytes > 0}
                {formatBytes(updateState.downloadedBytes)} / {formatBytes(updateState.totalBytes)}
              {:else if updateState.downloadedBytes > 0}
                {formatBytes(updateState.downloadedBytes)}
              {/if}
            </span>
            {#if updateState.speedText}
              <span>{updateState.speedText}</span>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Modal Actions -->
      <div class="flex items-center justify-end gap-2.5 pt-2 border-t border-white/[0.06]">
        <Button
          variant="ghost"
          disabled={updateState.downloading && !updateState.installing}
          onclick={() => updateState.closeModal()}
        >
          {i18n.t('update_action_later')}
        </Button>

        <Button
          variant="secondary"
          onclick={() => updateState.openReleasePage()}
        >
          <IconOpen class="w-4 h-4 mr-1.5 text-white/60" />
          {i18n.t('update_action_github')}
        </Button>

        <Button
          variant="accent"
          disabled={updateState.downloading}
          onclick={() => updateState.startInAppUpdate()}
        >
          {#if updateState.downloading}
            <IconLoading class="w-4 h-4 mr-1.5" />
            {updateState.installing ? i18n.t('update_installing') : `${updateState.downloadProgress}%`}
          {:else}
            <IconArrowDownload class="w-4 h-4 mr-1.5" />
            {i18n.t('update_action_download')}
          {/if}
        </Button>
      </div>
    </div>
  </Modal>
{/if}
