<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { updateState } from '$lib/state/updateState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  interface ParsedNoteLine {
    isHeader: boolean;
    text: string;
  }

  let parsedNotes = $derived.by<ParsedNoteLine[]>(() => {
    if (!updateState.info?.release_notes) return [];

    const lines = updateState.info.release_notes
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean);

    const result: ParsedNoteLine[] = [];

    for (const line of lines) {
      if (line.toLowerCase().startsWith('**full changelog**:') || line.toLowerCase().startsWith('full changelog:')) {
        continue;
      }

      const isHeader = line.startsWith('#');
      if (isHeader) {
        const cleanText = line.replace(/^#+\s*/, '').trim();
        if (/^(changes(\s+in)?|what'?s\s+(new|changed))/i.test(cleanText)) {
          continue;
        }
        result.push({ isHeader: true, text: cleanText });
      } else {
        const cleanText = line.replace(/^[-*•]\s*/, '').trim();
        if (cleanText) {
          result.push({ isHeader: false, text: cleanText });
        }
      }
    }

    return result;
  });
</script>

{#snippet updateBody()}
  {#if parsedNotes.length > 0}
    <div class="release-notes-box">
      <div class="release-notes-content">
        {#each parsedNotes as item}
          {#if item.isHeader}
            <div class="release-note-header">
              {item.text}
            </div>
          {:else}
            <div class="release-note-line">
              <span class="release-bullet">&bull;</span>
              <span>{item.text}</span>
            </div>
          {/if}
        {/each}
      </div>
    </div>
  {/if}
{/snippet}

{#snippet updateActions()}
  <div class="update-actions-wrapper">
    {#if updateState.downloading}
      <div class="update-progress-container">
        <div class="update-progress-header">
          <span class="update-progress-title">
            {updateState.installing ? i18n.t('update_installing') : i18n.t('update_downloading')}
          </span>
          <span class="update-progress-speed font-mono">
            {#if updateState.speedText && !updateState.installing}
              {updateState.speedText}
            {/if}
          </span>
        </div>
        <div class="update-progress-track">
          <div
            class="update-progress-bar"
            style:width="{updateState.downloadProgress}%"
          ></div>
        </div>
      </div>
    {/if}

    <div class="update-actions-grid">
      <Button
        variant="ghost"
        size="md"
        class="w-full justify-center px-3 whitespace-nowrap border border-[var(--border-color)]"
        disabled={updateState.downloading}
        onclick={() => updateState.closeModal()}
      >
        <span>{i18n.t('common.cancel')}</span>
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
          <span>{updateState.info?.asset_size ? formatBytes(updateState.info.asset_size) : i18n.t('update_action_download')}</span>
        {/if}
      </Button>
    </div>
  </div>
{/snippet}

{#if updateState.modalOpen && updateState.info}
  {#if layoutState.isMobile}
    <BottomSheet
      isOpen={updateState.modalOpen}
      title={i18n.t('update_modal_title')}
      closeLabel={i18n.t('common.close')}
      onclose={() => updateState.closeModal()}
    >
      {#snippet children()}
        <div class="update-modal-body">
          {@render updateBody()}
        </div>
      {/snippet}

      {#snippet footer()}
        {@render updateActions()}
      {/snippet}
    </BottomSheet>
  {:else}
    <Modal
      isOpen={updateState.modalOpen}
      title={i18n.t('update_modal_title')}
      size="lg"
      onclose={() => updateState.closeModal()}
    >
      {#snippet children()}
        <div class="update-modal-layout">
          {@render updateBody()}
        </div>
      {/snippet}

      {#snippet footer()}
        {@render updateActions()}
      {/snippet}
    </Modal>
  {/if}
{/if}

<style>
  .update-modal-layout {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 0;
  }

  .update-modal-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 0;
  }

  :global(.bottom-sheet) .update-modal-body {
    padding: 0;
  }

  .update-actions-wrapper {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
    box-sizing: border-box;
  }


  .release-notes-content {
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  .release-note-header {
    font-family: var(--font-mono, monospace);
    font-size: 12.5px;
    font-weight: 700;
    color: var(--accent-primary);
    padding-top: 8px;
    margin-top: 4px;
    border-top: var(--border-width) solid var(--border-color);
  }

  .release-note-header:first-child {
    padding-top: 0;
    margin-top: 0;
    border-top: none;
  }

  .release-note-line {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .release-bullet {
    color: var(--text-muted);
    flex-shrink: 0;
    user-select: none;
  }

  .update-progress-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .update-progress-header {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    color: var(--text-secondary);
  }

  .update-progress-track {
    width: 100%;
    height: 6px;
    border-radius: var(--radius-full);
    background: var(--bg-card);
    overflow: hidden;
  }

  .update-progress-bar {
    height: 100%;
    border-radius: var(--radius-full);
    background: var(--accent-primary);
    transition: width var(--duration-fast, 150ms) ease;
  }

  .update-actions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 10px;
    width: 100%;
    padding-top: 4px;
  }
</style>
