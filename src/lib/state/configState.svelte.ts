import type { AppSettings } from '$lib/types/config';

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
    titlebar_style: 'auto'
  });

  updateSettings(newSettings: AppSettings) {
    this.settings = newSettings;
    document.documentElement.setAttribute('data-theme', newSettings.theme);
  }
}

export const configState = new ConfigState();
