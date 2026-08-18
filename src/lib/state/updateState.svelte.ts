import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { notify } from '$lib/utils/toast';
import { i18n } from '$lib/i18n';
import { formatBytes } from '$lib/utils/formatters';
import { configState } from '$lib/state/configState.svelte';
import type { UpdateInfo, UpdateProgressPayload } from '$lib/types/update';

export class UpdateState {
  checking = $state(false);
  info = $state<UpdateInfo | null>(null);
  modalOpen = $state(false);
  error = $state<string | null>(null);

  downloading = $state(false);
  downloadProgress = $state(0);
  downloadedBytes = $state(0);
  totalBytes = $state(0);
  speedText = $state('');
  installing = $state(false);

  private unlistenProgress: UnlistenFn | null = null;

  async init() {
    if (this.unlistenProgress) return;
    try {
      this.unlistenProgress = await listen<UpdateProgressPayload>(
        'update-download-progress',
        (event) => {
          this.downloadProgress = Math.round(event.payload.percentage);
          this.downloadedBytes = event.payload.downloaded;
          this.totalBytes = event.payload.total;
          if (event.payload.speed_bytes_per_sec > 0) {
            this.speedText = `${formatBytes(event.payload.speed_bytes_per_sec)}/s`;
          } else {
            this.speedText = '';
          }
        }
      );
    } catch {
      // Ignored if outside Tauri context
    }
  }

  async check(silent: boolean = false) {
    await this.init();
    if (this.checking) return;
    this.checking = true;
    this.error = null;

    try {
      const includePrereleases = !!configState.settings.include_prereleases;
      const result = await invoke<UpdateInfo>('check_for_updates', {
        includePrereleases
      });

      this.info = result;

      if (result.available) {
        if (!silent) {
          this.modalOpen = true;
        } else {
          notify.info(
            i18n.t('update_available_title'),
            i18n.t('update_available_desc', { version: result.latest_version }),
            {
              duration: 9000,
              action: {
                label: i18n.t('update_view_changelog'),
                onClick: () => {
                  this.modalOpen = true;
                }
              }
            }
          );
        }
      } else if (!silent) {
        notify.success(
          i18n.t('update_up_to_date_title'),
          i18n.t('update_up_to_date_desc')
        );
      }
    } catch (err: any) {
      const message = typeof err === 'string' ? err : err?.message || String(err);
      this.error = message;
      if (!silent) {
        notify.error(i18n.t('update_check_failed'), message);
      }
    } finally {
      this.checking = false;
    }
  }

  openModal() {
    this.modalOpen = true;
  }

  closeModal() {
    if (this.downloading) {
      // Don't close if download in progress, or allow minimize
    }
    this.modalOpen = false;
  }

  async openReleasePage() {
    if (!this.info?.release_url) return;
    try {
      await invoke('open_in_browser', { url: this.info.release_url });
    } catch {
      window.open(this.info.release_url, '_blank');
    }
  }

  async startInAppUpdate() {
    await this.init();
    const downloadUrl = this.info?.download_url;
    const assetName = this.info?.asset_name || 'pawstash-update';

    if (!downloadUrl) {
      // If no direct asset matching current platform, fallback to GitHub release in browser
      return this.openReleasePage();
    }

    if (this.downloading) return;

    this.downloading = true;
    this.downloadProgress = 0;
    this.downloadedBytes = 0;
    this.totalBytes = this.info?.asset_size || 0;
    this.speedText = '';
    this.installing = false;

    try {
      await invoke('download_and_install_update', {
        downloadUrl,
        assetName
      });
      this.installing = true;
    } catch (err: any) {
      const message = typeof err === 'string' ? err : err?.message || String(err);
      notify.error(i18n.t('update_install_failed'), message);
      this.downloading = false;
      this.installing = false;
    }
  }
}

export const updateState = new UpdateState();
