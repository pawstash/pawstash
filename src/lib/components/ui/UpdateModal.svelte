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
</script>

{#if updateState.modalOpen && updateState.info}
  <Modal
    isOpen={updateState.modalOpen}
    title={i18n.t('update_modal_title')}
    onclose={() => updateState.closeModal()}
  >
    <div class="update-modal-content">
      <div class="update-header-card">
        <div class="update-icon-badge">
          <IconSparkle class="w-6 h-6 text-accent" />
        </div>
        <div class="update-header-info">
          <div class="update-title-row">
            <h3 class="update-release-name">
              {updateState.info.release_name || `v${updateState.info.latest_version}`}
            </h3>
            {#if updateState.info.is_prerelease}
              <span class="badge-prerelease">
                {i18n.t('update_badge_prerelease')}
              </span>
            {/if}
          </div>

          <div class="update-meta-row">
            <span class="version-chip">
              <IconTag class="w-3.5 h-3.5 opacity-60" />
              v{updateState.info.current_version} &rarr; <strong>v{updateState.info.latest_version}</strong>
            </span>
            {#if updateState.info.published_at}
              <span class="date-chip">
                <IconCalendar class="w-3.5 h-3.5 opacity-60" />
                {formatPublishedDate(updateState.info.published_at)}
              </span>
            {/if}
            {#if updateState.info.asset_size}
              <span class="size-chip">
                {formatBytes(updateState.info.asset_size)}
              </span>
            {/if}
          </div>
        </div>
      </div>

      <div class="changelog-container">
        <div class="changelog-label">{i18n.t('update_changelog_label')}</div>
        <div class="changelog-scroll">
          {#if updateState.info.release_notes}
            <pre class="changelog-text">{updateState.info.release_notes}</pre>
          {:else}
            <p class="changelog-empty">{i18n.t('update_no_changelog')}</p>
          {/if}
        </div>
      </div>

      {#if updateState.downloading}
        <div class="update-progress-card">
          <div class="progress-info-row">
            <span class="progress-status-text">
              {#if updateState.installing}
                {i18n.t('update_installing')}
              {:else}
                {i18n.t('update_downloading')}
              {/if}
            </span>
            <span class="progress-percent">{updateState.downloadProgress}%</span>
          </div>

          <div class="progress-track">
            <div class="progress-fill" style="width: {updateState.downloadProgress}%"></div>
          </div>

          <div class="progress-meta-row">
            <span>
              {#if updateState.totalBytes > 0}
                {formatBytes(updateState.downloadedBytes)} / {formatBytes(updateState.totalBytes)}
              {:else if updateState.downloadedBytes > 0}
                {formatBytes(updateState.downloadedBytes)}
              {/if}
            </span>
            {#if updateState.speedText}
              <span class="progress-speed">{updateState.speedText}</span>
            {/if}
          </div>
        </div>
      {/if}

      <div class="update-actions">
        <Button
          variant="ghost"
          disabled={updateState.downloading && !updateState.installing}
          onclick={() => updateState.closeModal()}
        >
          {i18n.t('update_action_later')}
        </Button>

        <Button variant="ghost" onclick={() => updateState.openReleasePage()}>
          <IconOpen class="w-4 h-4 mr-1.5" />
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

<style>
  .update-modal-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-4, 16px);
  }

  .update-header-card {
    display: flex;
    align-items: center;
    gap: var(--space-4, 16px);
    padding: var(--space-4, 16px);
    background: var(--color-bg-card, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: var(--radius-lg, 12px);
  }

  .update-icon-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    background: var(--color-accent-subtle, rgba(255, 64, 128, 0.12));
    border-radius: var(--radius-md, 8px);
    color: var(--color-accent);
    flex-shrink: 0;
  }

  .update-header-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    flex: 1;
  }

  .update-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .update-release-name {
    font-size: var(--font-size-base, 15px);
    font-weight: 600;
    color: var(--color-text-primary, #fff);
    margin: 0;
  }

  .badge-prerelease {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 8px;
    background: rgba(234, 179, 8, 0.16);
    color: #facc15;
    border: 1px solid rgba(234, 179, 8, 0.3);
    border-radius: 9999px;
  }

  .update-meta-row {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: var(--font-size-xs, 12px);
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.7));
    flex-wrap: wrap;
  }

  .version-chip,
  .date-chip {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .size-chip {
    font-weight: 500;
    opacity: 0.8;
  }

  .changelog-container {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .changelog-label {
    font-size: var(--font-size-xs, 12px);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.6));
  }

  .changelog-scroll {
    max-height: 200px;
    overflow-y: auto;
    background: var(--color-bg-input, rgba(0, 0, 0, 0.3));
    border: 1px solid var(--color-border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: var(--radius-md, 8px);
    padding: var(--space-3, 12px);
  }

  .changelog-text {
    font-family: inherit;
    font-size: var(--font-size-sm, 13px);
    line-height: 1.5;
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.85));
    white-space: pre-wrap;
    margin: 0;
  }

  .changelog-empty {
    font-size: var(--font-size-sm, 13px);
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.4));
    font-style: italic;
    margin: 0;
  }

  .update-progress-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: var(--space-3, 12px);
    background: var(--color-bg-card, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: var(--radius-md, 8px);
  }

  .progress-info-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: var(--font-size-xs, 12px);
  }

  .progress-status-text {
    font-weight: 500;
    color: var(--color-text-primary, #fff);
  }

  .progress-percent {
    font-weight: 600;
    color: var(--color-accent);
    font-feature-settings: 'tnum';
  }

  .progress-track {
    width: 100%;
    height: 6px;
    background: var(--color-bg-input, rgba(255, 255, 255, 0.08));
    border-radius: 9999px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-accent, #ff4080);
    border-radius: 9999px;
    transition: width 150ms ease-out;
    box-shadow: 0 0 10px var(--color-accent);
  }

  .progress-meta-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11px;
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.6));
    font-feature-settings: 'tnum';
  }

  .progress-speed {
    color: var(--color-accent);
    opacity: 0.9;
  }

  .update-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding-top: var(--space-2, 8px);
  }
</style>

