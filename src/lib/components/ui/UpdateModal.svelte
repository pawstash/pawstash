<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { scrollable } from '$lib/actions/scrollable';
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
    <div class="update-modal-layout">
      <div class="update-version-row">
        <span class="update-version-tag">
          v{updateState.info.latest_version}
        </span>
        {#if updateState.info.asset_size}
          <span class="update-size-tag">
            {formatBytes(updateState.info.asset_size)}
          </span>
        {/if}
      </div>

      {#if updateState.info.release_notes}
        <div class="release-notes-box" use:scrollable>
          <div class="release-notes-content">
            {#each updateState.info.release_notes.split('\n').map(l => l.trim()).filter(Boolean) as line}
              {#if line.startsWith('### ') || line.startsWith('## ')}
                <div class="release-note-header">
                  {line.replace(/^#+\s*/, '')}
                </div>
              {:else}
                <div class="release-note-line">
                  <span class="release-bullet">&bull;</span>
                  <span>{line.replace(/^[-*#]+\s*/, '')}</span>
                </div>
              {/if}
            {/each}
          </div>
        </div>
      {/if}

      {#if updateState.downloading}
        <div class="update-progress-container">
          <div class="update-progress-header">
            <span>{updateState.installing ? i18n.t('update_installing') : i18n.t('update_downloading')}</span>
            <span class="update-progress-pct">{updateState.downloadProgress}%</span>
          </div>
          <div class="update-progress-track">
            <div
              class="update-progress-bar"
              style:width="{updateState.downloadProgress}%"
            ></div>
          </div>
          {#if updateState.speedText || updateState.totalBytes > 0}
            <div class="update-progress-meta">
              <span>{formatBytes(updateState.downloadedBytes)} / {formatBytes(updateState.totalBytes)}</span>
              {#if updateState.speedText}
                <span>{updateState.speedText}</span>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <div class="update-actions-grid">
        <Button
          variant="ghost"
          size="md"
          class="w-full justify-center px-3 whitespace-nowrap"
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

<style>
  .update-modal-layout {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .update-version-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  .update-version-tag {
    font-family: var(--font-mono, monospace);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .update-size-tag {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    color: var(--text-muted);
  }

  .release-notes-box {
    max-height: 200px;
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    border: var(--border-width) solid var(--border-color);
    overflow: hidden;
  }

  .release-notes-content {
    padding: 12px 14px;
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

  .update-progress-pct {
    font-weight: 600;
    color: var(--text-primary);
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

  .update-progress-meta {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    color: var(--text-muted);
  }

  .update-actions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 10px;
    width: 100%;
    padding-top: 4px;
  }
</style>
