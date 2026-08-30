import type { AppSettings } from '$lib/types/config';
import { providerState } from '$lib/state/providerState.svelte';

export class ConfigState {
  settings = $state<AppSettings>({
    download_dir: 'Downloads/Pawstash',
    cache_max_mb: 128,
    api_domain: 'pawchive.pw',
    file_domain: 'file.pawchive.pw',
    image_domain: 'img.pawchive.pw',
    session_cookie: '',
    pawchive_username: '',
    theme: 'glass',
    use_aria2c: true,
    aria2_connections: 16,
    proxy_mode: 'system',
    proxy_url: '',
    proxy_username: '',
    proxy_password: '',
    proxy_bypass_local: true,
    grid_scale: 85,
    grid_aspect_ratio: 'square',
    dynamic_accent: true,
    sticky_header: true,
    layout_mode: 'auto',
    sync_enabled: true,
    sync_auto: true,
    sync_on_change: true,
    sync_pawchive_session: false,
    sync_pull_interval_seconds: 300,
    sync_push_interval_seconds: 60,
    toast_position: 'auto',
    auto_check_updates: true,
    include_prereleases: false,
    scroll_edge_mask: true,
    titlebar_style: 'auto',
    download_group_by_creator: true,
    download_creator_folder_template: '{creator}',
    download_group_by_post: false,
    download_post_folder_template: '{post_title}',
    download_filename_template: '{post_title} - {filename}',
    download_save_metadata: false,
    download_metadata_format: 'txt',
    panic_button_enabled: true,
    panic_button_shortcut: 'H',
    persist_in_app_favorites_locally: true
  });

  updateSettings(newSettings: AppSettings) {
    const sc = newSettings.panic_button_shortcut || newSettings.boss_key_shortcut;
    if (!sc || sc === 'Alt+X') {
      newSettings.panic_button_shortcut = 'H';
    } else {
      newSettings.panic_button_shortcut = sc;
    }
    if (newSettings.panic_button_enabled === undefined && newSettings.boss_key_enabled !== undefined) {
      newSettings.panic_button_enabled = newSettings.boss_key_enabled;
    }
    if (newSettings.panic_button_enabled === undefined) {
      newSettings.panic_button_enabled = true;
    }
    if (newSettings.providers && Array.isArray(newSettings.providers)) {
      providerState.providers = newSettings.providers;
    }
    this.settings = newSettings;
    document.documentElement.setAttribute('data-theme', newSettings.theme);
  }
}

export const configState = new ConfigState();
