import { configState } from '$lib/state/configState.svelte';
import { providerState } from '$lib/state/providerState.svelte';
import { contentState } from '$lib/state/contentState.svelte';
import { feedState } from '$lib/state/feedState.svelte';
import { creatorsState } from '$lib/state/creatorsState.svelte';
import { downloadState } from '$lib/state/downloadState.svelte';
import { themeState } from '$lib/theme/themeState.svelte';
import { uiScaleState } from '$lib/state/uiScaleState.svelte';
import { backgroundState } from '$lib/theme/backgroundState.svelte';
import { i18n } from '$lib/i18n';
import { notify } from '$lib/utils/toast';
import { logger } from '$lib/utils/logger';
import {
  apiClearAllContentCache,
  apiClearContentCache,
  apiGetCacheStats,
  apiGetDefaultSettings,
  apiGetNetworkDefaults,
  apiGetSettings,
  apiGetWebviewProxyStatus,
  apiSaveSettings,
  apiUpdatePanicKey,
  apiWipeAllData,
  FALLBACK_NETWORK_DEFAULTS,
  type CacheStats,
  type NetworkDefaults,
  type WebviewProxyStatus
} from '$lib/utils/ipc';
import type { AppSettings } from '$lib/types/config';

type Key = keyof AppSettings;

const SECTION_KEYS: Partial<Record<string, Key[]>> = {
  appearance: ['theme', 'dynamic_accent', 'linux_transparent_window', 'titlebar_style'],
  interface: ['layout_mode', 'toast_position', 'panic_button_enabled', 'panic_button_shortcut'],
  grid: ['grid_aspect_ratio', 'grid_scale', 'card_view_mode', 'disable_blur_placeholders'],
  player: [
    'viewer_close_action',
    'viewer_esc_action',
    'viewer_dock_size',
    'viewer_dock_autohide',
    'viewer_dock_autohide_delay',
    'viewer_dock_peek',
    'viewer_dock_keep_playing',
    'viewer_dock_on_touch',
    'viewer_pause_on_blur',
    'viewer_playback_rate'
  ],
  providers: ['provider_deadline_secs'],
  network: [
    'proxy_mode',
    'proxy_url',
    'proxy_username',
    'proxy_password',
    'proxy_bypass_local',
    'network_api_user_agent',
    'network_browser_user_agent',
    'network_timeout_secs',
    'network_connect_timeout_secs',
    'cloud_scraping_enabled',
    'content_indexing_enabled',
    'cloud_proxy_url',
    'cloud_user_agent',
    'cloud_timeout_secs',
    'cloud_max_redirects'
  ],
  downloads: [
    'download_dir',
    'download_group_by_creator',
    'download_creator_folder_template',
    'download_group_by_post',
    'download_post_folder_template',
    'download_filename_template',
    'download_save_metadata',
    'download_metadata_format',
    'use_aria2c',
    'aria2_connections',
    'download_max_concurrent',
    'download_auto_retry',
    'download_auto_retry_max'
  ],
  notifications: [
    'notifications_enabled',
    'notifications_download_completed',
    'notifications_show_preview',
    'notifications_sound'
  ],
  cache: ['cache_max_mb'],
  about: ['auto_check_updates', 'include_prereleases']
};

export function sectionIsResettable(id: string): boolean {
  return id in SECTION_KEYS || id.startsWith('provider-');
}

class SettingsController {
  settings = $state<AppSettings>({ ...configState.settings });
  defaults = $state<AppSettings>({ ...configState.settings });
  networkDefaults = $state<NetworkDefaults>({ ...FALLBACK_NETWORK_DEFAULTS });
  webviewProxy = $state<WebviewProxyStatus | null>(null);
  cacheStats = $state<CacheStats | null>(null);
  cacheBusy = $state<'images' | 'all' | null>(null);
  resetPending = $state(false);
  wipePending = $state(false);

  private preview: { key: Key; previous: unknown } | null = null;

  get webviewProxyNotice(): string | null {
    const status = this.webviewProxy;
    if (!status?.needed && !status?.active) return null;
    if (!status.supported) return i18n.t('settings.webview_proxy_unsupported');
    if (status.needed !== status.active) return i18n.t('settings.webview_proxy_restart');
    return null;
  }

  async load() {
    this.settings = { ...configState.settings };
    const [defaults, networkDefaults, onDisk] = await Promise.allSettled([
      apiGetDefaultSettings(),
      apiGetNetworkDefaults(),
      apiGetSettings()
    ]);
    if (defaults.status === 'fulfilled') this.defaults = defaults.value;
    else logger.warn('Failed to fetch default settings', defaults.reason);
    if (networkDefaults.status === 'fulfilled') this.networkDefaults = networkDefaults.value;
    else logger.warn('Failed to fetch network defaults', networkDefaults.reason);
    if (onDisk.status === 'fulfilled') {
      configState.hydrate(onDisk.value);
      this.settings = { ...onDisk.value };
    } else {
      logger.warn('Failed to refresh settings from disk', onDisk.reason);
    }
  }

  refreshWebviewProxyStatus() {
    void apiGetWebviewProxyStatus()
      .then((status) => (this.webviewProxy = status))
      .catch((error) => logger.warn('Failed to read WebView proxy status', error));
  }

  save<K extends Key>(key: K, value: AppSettings[K], previous: AppSettings[K] = configState.settings[key]) {
    return this.saveMany({ [key]: value } as Partial<AppSettings>, { [key]: previous } as Partial<AppSettings>);
  }

  async saveMany(patch: Partial<AppSettings>, previous?: Partial<AppSettings>): Promise<boolean> {
    const keys = Object.keys(patch) as Key[];
    const rollback = previous ?? Object.fromEntries(keys.map((key) => [key, configState.settings[key]]));
    Object.assign(this.settings, patch);
    const next = { ...this.settings };
    configState.updateSettings(next);
    try {
      await configState.save(next);
      if (keys.includes('cache_max_mb')) await this.loadCacheStats();
      this.refreshWebviewProxyStatus();
      return true;
    } catch (error) {
      Object.assign(this.settings, rollback);
      configState.updateSettings({ ...this.settings });
      notify.error(i18n.t('settings.save_failed'), error);
      return false;
    }
  }

  previewValue<K extends Key>(key: K, value: AppSettings[K]) {
    if (this.preview?.key !== key) this.preview = { key, previous: this.settings[key] };
    this.settings[key] = value;
    configState.updateSettings({ ...this.settings });
  }

  commit<K extends Key>(key: K, value: AppSettings[K]) {
    const previous = (this.preview?.key === key ? this.preview.previous : this.settings[key]) as AppSettings[K];
    this.preview = null;
    void this.save(key, value, previous);
  }

  setPanicKey(shortcut: string, enabled: boolean) {
    void this.saveMany({ panic_button_shortcut: shortcut, panic_button_enabled: enabled });
    void apiUpdatePanicKey(shortcut, enabled).catch((error) =>
      notify.error(i18n.t('settings.save_failed'), error)
    );
  }

  async loadCacheStats() {
    try {
      this.cacheStats = await apiGetCacheStats();
    } catch (error) {
      notify.error(i18n.t('settings.cache_stats_failed'), error);
    }
  }

  async clearCache(scope: 'images' | 'all') {
    if (this.cacheBusy) return;
    this.cacheBusy = scope;
    try {
      this.cacheStats = scope === 'all' ? await apiClearAllContentCache() : await apiClearContentCache();
      notify.success(
        i18n.t(scope === 'all' ? 'settings.cache_all_cleared' : 'settings.cache_cleared'),
        i18n.t('settings.cache_usage_desc')
      );
    } catch (error) {
      notify.error(i18n.t('settings.cache_clear_failed'), error);
    } finally {
      this.cacheBusy = null;
    }
  }

  async resetSection(id: string, label: string) {
    try {
      if (id.startsWith('provider-')) {
        await this.resetProvider(id.slice('provider-'.length));
      } else {
        const defaults = await apiGetDefaultSettings();
        const next = { ...this.settings };
        for (const key of SECTION_KEYS[id] ?? []) (next as Record<Key, unknown>)[key] = defaults[key];
        if (id === 'appearance') {
          const { fontFamily } = themeState.tokens;
          themeState.reset();
          themeState.setFontFamily(fontFamily ?? '');
          backgroundState.reset();
        }
        if (id === 'interface') {
          themeState.setFontFamily('');
          uiScaleState.set(0);
        }
        this.settings = next;
        configState.updateSettings(next);
        await configState.save(next);
        if (id === 'interface') {
          await apiUpdatePanicKey(next.panic_button_shortcut || 'H', next.panic_button_enabled ?? true);
        }
        this.refreshWebviewProxyStatus();
      }
      notify.success(i18n.t('settings.reset_section_success'), label);
    } catch (error) {
      notify.error(i18n.t('settings.reset_failed'), error);
    }
  }

  private async resetProvider(providerId: string) {
    const provider = providerState.providers.find((p) => p.id === providerId);
    if (!provider) return;
    const defaults = await apiGetDefaultSettings();
    const original = defaults.providers?.find((p) => p.id === providerId);
    await providerState.updateProvider({
      ...provider,
      enabled: original?.enabled ?? true,
      api_url: original?.api_url ?? provider.api_url,
      advanced_network: false,
      user_agent: '',
      timeout_secs: 0,
      proxy_url: '',
      max_retries: 0,
      min_interval_ms: 0
    });
    if (providerId === 'pawchive') await this.save('pawchive_hide_ai', defaults.pawchive_hide_ai ?? false);
    this.reloadContent();
  }

  reloadContent() {
    contentState.clearAllCache();
    void feedState.refresh();
    void creatorsState.refresh();
  }

  async resetAll() {
    if (this.resetPending) return false;
    this.resetPending = true;
    try {
      const defaults = await apiGetDefaultSettings();
      await apiSaveSettings(defaults);
      configState.hydrate(defaults);
      this.settings = { ...defaults };
      themeState.reset();
      backgroundState.reset();
      await this.loadCacheStats();
      notify.success(i18n.t('settings.reset_success'), i18n.t('settings.reset_all_desc'));
      return true;
    } catch (error) {
      notify.error(i18n.t('settings.reset_failed'), error);
      return false;
    } finally {
      this.resetPending = false;
    }
  }

  async wipeAll() {
    if (this.wipePending) return false;
    this.wipePending = true;
    try {
      this.cacheStats = await apiWipeAllData();
      notify.success(i18n.t('settings.wipe_all_data_success'), i18n.t('settings.wipe_all_data_desc'));
      await this.loadCacheStats();
      await downloadState.refresh();
      return true;
    } catch (error) {
      notify.error(i18n.t('settings.wipe_all_data_failed'), error);
      return false;
    } finally {
      this.wipePending = false;
    }
  }
}

export const settingsController = new SettingsController();
