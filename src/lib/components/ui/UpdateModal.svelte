<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { updateState } from '$lib/state/updateState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
</script>

{#if updateState.modalOpen && updateState.info}
  <Modal
    isOpen={updateState.modalOpen}
    title={i18n.t('update_modal_title')}
    size="md"
    onclose={() => updateState.closeModal()}
  >
    <div class="flex flex-col gap-4">
      <div class="flex items-baseline justify-between gap-2">
        <span class="text-sm font-semibold text-white font-mono">
          v{updateState.info.latest_version}
        </span>
        {#if updateState.info.asset_size}
          <span class="text-xs font-mono text-white/40">
            {formatBytes(updateState.info.asset_size)}
          </span>
        {/if}
      </div>

      {#if updateState.info.release_notes}
        <div class="max-h-48 overflow-y-auto custom-scrollbar rounded-xl bg-black/40 border border-white/[0.06] p-3 text-xs text-white/70 leading-relaxed space-y-1.5">
          {#each updateState.info.release_notes.split('\n').map(l => l.trim()).filter(Boolean) as line}
            <div class="flex items-start gap-2">
              <span class="text-white/30 shrink-0 select-none">&bull;</span>
              <span>{line.replace(/^[-*#]+\s*/, '')}</span>
            </div>
          {/each}
        </div>
      {/if}

      {#if updateState.downloading}
        <div class="flex flex-col gap-1.5">
          <div class="flex justify-between text-xs font-mono text-white/60">
            <span>{updateState.installing ? i18n.t('update_installing') : i18n.t('update_downloading')}</span>
            <span class="text-white font-semibold">{updateState.downloadProgress}%</span>
          </div>
          <div class="w-full h-1.5 rounded-full bg-white/10 overflow-hidden">
            <div
              class="h-full bg-[var(--accent)] transition-all duration-150 rounded-full"
              style="width: {updateState.downloadProgress}%"
            ></div>
          </div>
          {#if updateState.speedText || updateState.totalBytes > 0}
            <div class="flex justify-between text-[11px] font-mono text-white/40">
              <span>{formatBytes(updateState.downloadedBytes)} / {formatBytes(updateState.totalBytes)}</span>
              {#if updateState.speedText}
                <span>{updateState.speedText}</span>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5 w-full pt-1">
        <Button
          variant="ghost"
          size="md"
          class="w-full justify-center px-3 border border-white/8 hover:border-white/16 whitespace-nowrap"
          onclick={() => updateState.openReleasePage()}
        >
          <IconOpen class="w-4 h-4 mr-2 opacity-60 shrink-0" />
          <span>{i18n.t('update_action_github')}</span>
        </Button>

        <Button
          variant="accent"
          size="md"
          class="w-full justify-center px-3 whitespace-nowrap"
          disabled={updateState.downloading}
          onclick={() => updateState.startInAppUpdate()}
        >
          {#if updateState.downloading}
            <IconLoading class="mr-2 shrink-0" />
            <span>{updateState.installing ? i18n.t('update_installing') : `${updateState.downloadProgress}%`}</span>
          {:else}
            <IconArrowDownload class="mr-2 shrink-0" />
            <span>{i18n.t('update_action_download')}</span>
          {/if}
        </Button>
      </div>
    </div>
  </Modal>
{/if}
