<script lang="ts">
  import { configState } from '$lib/state/configState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { onMount } from 'svelte';
  import {
    backgroundState,
    supportedBackgroundTypes,
    type BackgroundType,
    type CustomBackgroundKind
  } from '$lib/theme/backgroundState.svelte';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { i18n, LOCALES, type Locale } from '$lib/i18n';
  import {
    apiClearContentCache,
    apiClearAllContentCache,
    apiGetCacheStats,
    apiGetDefaultSettings,
    apiGetSettings,
    apiSaveSettings,
    openExternalUrl,
    apiWipeAllData,
    type CacheStats
  } from '$lib/utils/ipc';
  import { formatBytes } from '$lib/utils/formatters';
  import { APP_VERSION, BUILD_TIME, COMMIT_HASH } from '$lib/version';
  import { notify } from '$lib/utils/toast';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import SectionTitle from '$lib/components/layout/SectionTitle.svelte';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import SegmentedControl from '$lib/components/ui/SegmentedControl.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import Slider from '$lib/components/ui/Slider.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import TemplateInput, { type TemplateTag } from '$lib/components/ui/TemplateInput.svelte';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconKey from '~icons/fluent/key-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconPaint from '~icons/fluent/color-24-regular';
  import IconEye from '~icons/fluent/eye-24-regular';
  import IconTranslate from '~icons/fluent/translate-24-regular';
  import IconFlagUs from '~icons/circle-flags/us';
  import IconFlagRu from '~icons/circle-flags/ru';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconGrid from '~icons/fluent/grid-24-regular';
  import IconDatabase from '~icons/fluent/database-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import type { AccentColor } from '$lib/theme/tokens';
  import { ripple } from '$lib/motion';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';
  import Button from '$lib/components/ui/Button.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import SyncSettings from './SyncSettings.svelte';
  import { updateState } from '$lib/state/updateState.svelte';
  import { syncState } from '$lib/state/syncState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import IconUser from '~icons/fluent/person-24-regular';
  import pawchiveLogo from '$lib/components/pawchive/pawchive-favicon.png';
  import IconChevronRight from '~icons/fluent/chevron-right-24-regular';
  import IconCloudSync from '~icons/fluent/cloud-sync-24-regular';
  import IconArrowReset from '~icons/fluent/arrow-reset-24-regular';
  import IconMoreHorizontal from '~icons/fluent/more-horizontal-24-regular';
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import StorageBar from './StorageBar.svelte';
  import DownloadsStatsBar from '$lib/components/downloads/DownloadsStatsBar.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import IconArrowSync from '~icons/fluent/arrow-sync-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconTelegram from '~icons/simple-icons/telegram';
  import IconDiscord from '~icons/simple-icons/discord';
  import IconReddit from '~icons/simple-icons/reddit';
  import IconGithub from '~icons/simple-icons/github';
  import IconHeart from '~icons/fluent/heart-24-filled';

  let settings = $state({ ...configState.settings });
  let resetPending = $state(false);
  let settingsMenuOpen = $state(false);
  let stickySettingsMenuOpen = $state(false);
  let activeCategory = $state('appearance');
  let categoryScrollFrame: number | undefined;
  let availableBackgroundTypes = $state<BackgroundType[]>(supportedBackgroundTypes());
  let cacheStats = $state<CacheStats | null>(null);
  let cacheBusy = $state<'images' | 'all' | null>(null);

  let profileName = $derived(
    syncState.status.account_id || i18n.t('profile.local')
  );

  let cloudAccountName = $derived(
    syncState.status.account_id || i18n.t('profile.local')
  );

  let syncDotStatus = $derived.by(() => {
    if (syncState.status.configured) {
      if (!syncState.status.unlocked) return 'locked';
      if (syncState.busy) return 'syncing';
      return 'active';
    }
    return 'offline';
  });

  let formattedBuildTime = $derived.by(() => {
    try {
      const d = new Date(BUILD_TIME);
      if (isNaN(d.getTime())) return BUILD_TIME;
      return d.toLocaleString(i18n.currentLocale === 'ru' ? 'ru-RU' : 'en-US', {
        day: 'numeric',
        month: 'short',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      });
    } catch {
      return BUILD_TIME;
    }
  });

  const toastPositionOptions = $derived([
    { value: 'auto', label: i18n.t('settings.toast_auto') },
    { value: 'top-center', label: i18n.t('settings.toast_top_center') },
    { value: 'top-right', label: i18n.t('settings.toast_top_right') },
    { value: 'top-left', label: i18n.t('settings.toast_top_left') },
    { value: 'bottom-center', label: i18n.t('settings.toast_bottom_center') },
    { value: 'bottom-right', label: i18n.t('settings.toast_bottom_right') },
    { value: 'bottom-left', label: i18n.t('settings.toast_bottom_left') }
  ]);

  const categories = [
    { id: 'appearance', labelKey: 'settings.appearance_section' },
    { id: 'grid', labelKey: 'settings.grid_section' },
    { id: 'proxy', labelKey: 'settings.proxy_section' },
    { id: 'background', labelKey: 'settings.background_section' },
    { id: 'network', labelKey: 'settings.network_section' },
    { id: 'downloads', labelKey: 'settings.download_section' },
    { id: 'cache', labelKey: 'settings.cache_section' },
    { id: 'sync', labelKey: 'sync.title' },
    { id: 'updates', labelKey: 'settings.updates_section' }
  ];

  onMount(async () => {
    availableBackgroundTypes = supportedBackgroundTypes();
    try {
      const loaded = await apiGetSettings();
      configState.updateSettings(loaded);
      settings = { ...loaded };
      await loadCacheStats();
    } catch (err) {
      notify.error(i18n.t('settings.load_failed') || 'Failed to load settings', err);
    }
  });

  onMount(() => {
    const observer = new IntersectionObserver((entries) => {
      const visible = entries
        .filter((entry) => entry.isIntersecting)
        .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)[0];
      if (visible?.target.id.startsWith('settings-')) {
        activeCategory = visible.target.id.slice('settings-'.length);
        revealCategory(activeCategory);
      }
    }, {
      rootMargin: '-18% 0px -68% 0px',
      threshold: 0
    });
    const frame = requestAnimationFrame(() => {
      for (const category of categories) {
        const section = document.getElementById(`settings-${category.id}`);
        if (section) observer.observe(section);
      }
    });
    return () => {
      cancelAnimationFrame(frame);
      if (categoryScrollFrame !== undefined) cancelAnimationFrame(categoryScrollFrame);
      observer.disconnect();
    };
  });

  let bgImageInput = $state<HTMLInputElement | null>(null);
  let bgVideoInput = $state<HTMLInputElement | null>(null);

  async function selectDownloadDir() {
    try {
      if (layoutState.isMobile) {
        const selected = await invoke<string | null>('pick_folder');
        if (selected && typeof selected === 'string') {
          settings.download_dir = selected;
          updateAndSaveSetting('download_dir', selected);
          notify.success(i18n.t('settings.download_path_updated'), selected);
        }
      } else {
        const selected = await open({
          directory: true,
          multiple: false,
          title: i18n.t('settings.download_dir')
        });
        if (selected && typeof selected === 'string') {
          settings.download_dir = selected;
          updateAndSaveSetting('download_dir', selected);
          notify.success(i18n.t('settings.download_path_updated'), selected);
        }
      }
    } catch (err) {
      notify.error(i18n.t('settings.download_dir'), err);
    }
  }

  async function selectCustomBackground(kind: 'image' | 'video') {
    if (layoutState.isMobile) {
      if (kind === 'image') bgImageInput?.click();
      else bgVideoInput?.click();
      return;
    }
    try {
      const selected = await open({
        multiple: false,
        title: i18n.t(kind === 'image' ? 'settings.choose_background_image' : 'settings.choose_background_video'),
        filters: [{
          name: kind === 'image' ? 'Images' : 'Videos',
          extensions: kind === 'image'
            ? ['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif']
            : ['mp4', 'webm']
        }]
      });
      if (selected && typeof selected === 'string') {
        const storedPath = await invoke<string>('store_custom_background', { sourcePath: selected, kind });
        if (kind === 'image') backgroundState.setImageUrl(storedPath);
        else backgroundState.setVideoUrl(storedPath);
        notify.success(
          i18n.t(kind === 'image' ? 'settings.background_saved' : 'settings.background_video_saved'),
          storedPath.split(/[/\\]/).pop() || storedPath
        );
      }
    } catch {
      if (kind === 'image') bgImageInput?.click();
      else bgVideoInput?.click();
    }
  }

  async function handleFileInputChange(event: Event, kind: 'image' | 'video') {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;
    try {
      const reader = new FileReader();
      const base64Data = await new Promise<string>((resolve, reject) => {
        reader.onload = () => resolve(String(reader.result));
        reader.onerror = (e) => reject(e);
        reader.readAsDataURL(file);
      });
      const ext = file.name.split('.').pop() || (kind === 'image' ? 'png' : 'mp4');
      const storedPath = await invoke<string>('store_custom_background_bytes', {
        dataBase64: base64Data,
        extension: ext,
        kind
      });
      if (kind === 'image') backgroundState.setImageUrl(storedPath);
      else backgroundState.setVideoUrl(storedPath);
      notify.success(
        i18n.t(kind === 'image' ? 'settings.background_saved' : 'settings.background_video_saved'),
        file.name
      );
    } catch (error) {
      notify.error(i18n.t('settings.background_file_failed'), error);
    } finally {
      target.value = '';
    }
  }

  async function loadCacheStats() {
    try {
      cacheStats = await apiGetCacheStats();
    } catch (error) {
      notify.error(i18n.t('settings.cache_stats_failed'), error);
    }
  }

  async function clearCache(scope: 'images' | 'all') {
    if (cacheBusy) return;
    cacheBusy = scope;
    try {
      cacheStats = scope === 'all' ? await apiClearAllContentCache() : await apiClearContentCache();
      notify.success(
        i18n.t(scope === 'all' ? 'settings.cache_all_cleared' : 'settings.cache_cleared'),
        i18n.t('settings.cache_usage_desc')
      );
    } catch (error) {
      notify.error(i18n.t('settings.cache_clear_failed'), error);
    } finally {
      cacheBusy = null;
    }
  }

  const isCustomActive = $derived(
    !['violet', 'indigo', 'cyan', 'emerald', 'amber', 'rose', 'rgb'].includes(themeState.tokens.accent)
  );

  async function updateAndSaveSetting(key: keyof typeof settings, val: any) {
    const previousValue = settings[key];
    (settings as any)[key] = val;
    const nextSettings = { ...settings };
    configState.updateSettings(nextSettings);
    try {
      await apiSaveSettings(nextSettings);
      if (key === 'cache_max_mb') await loadCacheStats();
    } catch (err: any) {
      (settings as any)[key] = previousValue;
      configState.updateSettings({ ...settings });
      notify.error(i18n.t('settings.save_failed') || 'Failed to save settings', err);
    }
  }

  function openCategory(id: string) {
    activeCategory = id;
    revealCategory(id);
    document.getElementById(`settings-${id}`)?.scrollIntoView({
      behavior: 'smooth',
      block: 'start'
    });
  }

  function revealCategory(id: string) {
    if (categoryScrollFrame !== undefined) cancelAnimationFrame(categoryScrollFrame);
    categoryScrollFrame = requestAnimationFrame(() => {
      const behavior: ScrollBehavior = window.matchMedia('(prefers-reduced-motion: reduce)').matches
        ? 'auto'
        : 'smooth';

      for (const list of document.querySelectorAll<HTMLElement>('.settings-categories')) {
        const button = list.querySelector<HTMLElement>(`[data-settings-category="${id}"]`);
        if (!button) continue;

        const listRect = list.getBoundingClientRect();
        const buttonRect = button.getBoundingClientRect();
        const left =
          list.scrollLeft +
          buttonRect.left -
          listRect.left -
          (list.clientWidth - buttonRect.width) / 2;
        list.scrollTo({ left: Math.max(0, left), behavior });
      }
      categoryScrollFrame = undefined;
    });
  }

  let showResetConfirm = $state(false);
  let showWipeConfirm = $state(false);

  async function executeResetAllSettings() {
    if (resetPending) return;
    resetPending = true;
    try {
      const defaults = await apiGetDefaultSettings();
      await apiSaveSettings(defaults);
      configState.updateSettings(defaults);
      settings = { ...defaults };
      themeState.reset();
      backgroundState.reset();
      await loadCacheStats();
      showResetConfirm = false;
      notify.success(
        i18n.t('settings.reset_success'),
        i18n.t('settings.reset_all_desc')
      );
    } catch (e) {
      notify.error(i18n.t('settings.reset_failed'), e);
    } finally {
      resetPending = false;
    }
  }

  let wipePending = $state(false);

  async function executeWipeAllData() {
    if (wipePending) return;
    wipePending = true;
    try {
      const stats = await apiWipeAllData();
      cacheStats = stats;
      showWipeConfirm = false;
      notify.success(
        i18n.t('settings.wipe_all_data_success'),
        i18n.t('settings.wipe_all_data_desc')
      );
      await loadCacheStats();
      await downloadState.refresh();
    } catch (e: any) {
      notify.error(i18n.t('settings.wipe_all_data_failed'), e);
    } finally {
      wipePending = false;
    }
  }

  async function resetSection(sectionId: string) {
    try {
      const defaults = await apiGetDefaultSettings();
      const next = { ...settings };

      switch (sectionId) {
        case 'appearance':
          themeState.reset();
          backgroundState.reset();
          next.theme = defaults.theme;
          next.dynamic_accent = defaults.dynamic_accent;
          next.sticky_header = defaults.sticky_header;
          next.layout_mode = defaults.layout_mode;
          next.scroll_edge_mask = defaults.scroll_edge_mask;
          next.titlebar_style = defaults.titlebar_style;
          next.toast_position = defaults.toast_position;
          break;

        case 'grid':
          next.grid_aspect_ratio = defaults.grid_aspect_ratio;
          next.grid_scale = defaults.grid_scale;
          break;

        case 'downloads':
          next.aria2_connections = defaults.aria2_connections;
          next.use_aria2c = defaults.use_aria2c;
          next.download_dir = defaults.download_dir;
          next.download_group_by_creator = defaults.download_group_by_creator;
          next.download_creator_folder_template = defaults.download_creator_folder_template;
          next.download_group_by_post = defaults.download_group_by_post;
          next.download_post_folder_template = defaults.download_post_folder_template;
          next.download_filename_template = defaults.download_filename_template;
          break;

        case 'network':
          next.proxy_mode = defaults.proxy_mode;
          next.proxy_url = defaults.proxy_url;
          next.proxy_username = defaults.proxy_username;
          next.proxy_password = defaults.proxy_password;
          next.proxy_bypass_local = defaults.proxy_bypass_local;
          next.api_domain = defaults.api_domain;
          next.file_domain = defaults.file_domain;
          next.image_domain = defaults.image_domain;
          break;

        case 'cache':
          next.cache_max_mb = defaults.cache_max_mb;
          break;

        case 'updates':
          next.auto_check_updates = defaults.auto_check_updates;
          next.include_prereleases = defaults.include_prereleases;
          break;
      }

      settings = next;
      configState.updateSettings(next);
      await apiSaveSettings(next);
      notify.success(
        i18n.t('settings.reset_section_success'),
        i18n.t(`settings.section_${sectionId}`)
      );
    } catch (err) {
      notify.error(i18n.t('settings.reset_failed'), err);
    }
  }

  const backgroundLabelKeys: Record<BackgroundType, string> = {
    acrylic: 'settings.bg_acrylic',
    vibrancy: 'settings.bg_vibrancy',
    'mica-dark': 'settings.bg_mica_dark',
    tabbed: 'settings.bg_tabbed',
    oled: 'settings.bg_oled',
    custom: 'settings.bg_custom'
  };
  let bgTypes = $derived(
    availableBackgroundTypes.map((id) => ({ id, label: i18n.t(backgroundLabelKeys[id]) }))
  );

  const accentColors: { id: AccentColor; color: string; label: string }[] = [
    { id: 'violet', color: '#8b5cf6', label: 'Violet' },
    { id: 'indigo', color: '#6366f1', label: 'Indigo' },
    { id: 'cyan', color: '#06b6d4', label: 'Cyan' },
    { id: 'emerald', color: '#10b981', label: 'Emerald' },
    { id: 'amber', color: '#f59e0b', label: 'Amber' },
    { id: 'rose', color: '#f43f5e', label: 'Rose' }
  ];

  let creatorFolderTags = $derived<TemplateTag[]>([
    { tag: '{creator}', label: i18n.t('settings.tag_creator'), example: 'AuthorName' },
    { tag: '{service}', label: i18n.t('settings.tag_service'), example: 'Platform' },
    { tag: '{creator_id}', label: i18n.t('settings.tag_creator_id'), example: '12345' },
    { tag: '{year}', label: i18n.t('settings.tag_year'), example: '2024' },
    { tag: '{month}', label: i18n.t('settings.tag_month'), example: '08' }
  ]);

  let postFolderTags = $derived<TemplateTag[]>([
    { tag: '{post_title}', label: i18n.t('settings.tag_post_title'), example: 'PostTitle' },
    { tag: '{post_id}', label: i18n.t('settings.tag_post_id'), example: '67890' },
    { tag: '{date}', label: i18n.t('settings.tag_date'), example: '2024-08-20' },
    { tag: '{year}', label: i18n.t('settings.tag_year'), example: '2024' },
    { tag: '{month}', label: i18n.t('settings.tag_month'), example: '08' },
    { tag: '{day}', label: i18n.t('settings.tag_day'), example: '20' },
    { tag: '{creator}', label: i18n.t('settings.tag_creator'), example: 'AuthorName' },
    { tag: '{service}', label: i18n.t('settings.tag_service'), example: 'Platform' }
  ]);

  let filenameTags = $derived<TemplateTag[]>([
    { tag: '{post_title}', label: i18n.t('settings.tag_post_title'), example: 'PostTitle' },
    { tag: '{filename}', label: i18n.t('settings.tag_filename'), example: 'OriginalFilename.png' },
    { tag: '{name}', label: i18n.t('settings.tag_name'), example: 'OriginalFilename' },
    { tag: '{ext}', label: i18n.t('settings.tag_ext'), example: 'png' },
    { tag: '{index}', label: i18n.t('settings.tag_index'), example: '1' },
    { tag: '{date}', label: i18n.t('settings.tag_date'), example: '2024-08-20' },
    { tag: '{year}', label: i18n.t('settings.tag_year'), example: '2024' },
    { tag: '{month}', label: i18n.t('settings.tag_month'), example: '08' },
    { tag: '{day}', label: i18n.t('settings.tag_day'), example: '20' },
    { tag: '{creator}', label: i18n.t('settings.tag_creator'), example: 'AuthorName' },
    { tag: '{service}', label: i18n.t('settings.tag_service'), example: 'Platform' },
    { tag: '{post_id}', label: i18n.t('settings.tag_post_id'), example: '67890' }
  ]);

  function getPreviewPath(template: string, type: 'creator' | 'post' | 'file'): string {
    let t = (template || '').trim();
    if (type === 'creator') {
      if (!t) t = '{creator}';
      return t
        .replaceAll('{creator}', 'AuthorName')
        .replaceAll('{author}', 'AuthorName')
        .replaceAll('{name}', 'AuthorName')
        .replaceAll('{service}', 'Platform')
        .replaceAll('{platform}', 'Platform')
        .replaceAll('{creator_id}', '12345')
        .replaceAll('{id}', '12345')
        .replaceAll('{date}', '2024-08-20')
        .replaceAll('{published}', '2024-08-20')
        .replaceAll('{date_compact}', '20240820')
        .replaceAll('{date_dots}', '2024.08.20')
        .replaceAll('{year}', '2024')
        .replaceAll('{yyyy}', '2024')
        .replaceAll('{year_short}', '24')
        .replaceAll('{yy}', '24')
        .replaceAll('{month}', '08')
        .replaceAll('{mm}', '08')
        .replaceAll('{day}', '20')
        .replaceAll('{dd}', '20')
        .replaceAll('{year_month}', '2024-08');
    }
    if (type === 'post') {
      if (!t) t = '{post_title}';
      return t
        .replaceAll('{post_title}', 'PostTitle')
        .replaceAll('{title}', 'PostTitle')
        .replaceAll('{post_id}', '67890')
        .replaceAll('{id}', '67890')
        .replaceAll('{creator}', 'AuthorName')
        .replaceAll('{author}', 'AuthorName')
        .replaceAll('{name}', 'AuthorName')
        .replaceAll('{service}', 'Platform')
        .replaceAll('{platform}', 'Platform')
        .replaceAll('{date}', '2024-08-20')
        .replaceAll('{published}', '2024-08-20')
        .replaceAll('{date_compact}', '20240820')
        .replaceAll('{date_dots}', '2024.08.20')
        .replaceAll('{year}', '2024')
        .replaceAll('{yyyy}', '2024')
        .replaceAll('{year_short}', '24')
        .replaceAll('{yy}', '24')
        .replaceAll('{month}', '08')
        .replaceAll('{mm}', '08')
        .replaceAll('{day}', '20')
        .replaceAll('{dd}', '20')
        .replaceAll('{year_month}', '2024-08');
    }
    if (!t) t = '{post_title} - {filename}';
    let res = t
      .replaceAll('{post_title}', 'PostTitle')
      .replaceAll('{title}', 'PostTitle')
      .replaceAll('{post_id}', '67890')
      .replaceAll('{creator}', 'AuthorName')
      .replaceAll('{author}', 'AuthorName')
      .replaceAll('{service}', 'Platform')
      .replaceAll('{platform}', 'Platform')
      .replaceAll('{filename}', 'OriginalFilename.png')
      .replaceAll('{original_name}', 'OriginalFilename.png')
      .replaceAll('{name}', 'OriginalFilename')
      .replaceAll('{ext}', 'png')
      .replaceAll('{index}', '1')
      .replaceAll('{date}', '2024-08-20')
      .replaceAll('{published}', '2024-08-20')
      .replaceAll('{date_compact}', '20240820')
      .replaceAll('{date_dots}', '2024.08.20')
      .replaceAll('{year}', '2024')
      .replaceAll('{yyyy}', '2024')
      .replaceAll('{year_short}', '24')
      .replaceAll('{yy}', '24')
      .replaceAll('{month}', '08')
      .replaceAll('{mm}', '08')
      .replaceAll('{day}', '20')
      .replaceAll('{dd}', '20')
      .replaceAll('{year_month}', '2024-08')
      .replaceAll('{media_id}', 'MediaID');

    if (res.toLowerCase().endsWith('.png')) {
      return res;
    }
    const cleanStem = res.replace(/\.png/gi, '');
    return `${cleanStem}.png`;
  }

  let previewRoot = $derived(
    settings.download_dir.replace(/\\/g, '/').replace(/\/+$/, '') || 'Downloads/Pawstash'
  );
  let previewCreator = $derived(
    getPreviewPath(settings.download_creator_folder_template, 'creator')
  );
  let previewPost = $derived(
    getPreviewPath(settings.download_post_folder_template, 'post')
  );
  let previewFilename = $derived(
    getPreviewPath(settings.download_filename_template, 'file')
  );

  let fullDownloadPathPreview = $derived.by(() => {
    const parts = [previewRoot];
    if (settings.download_group_by_creator) {
      parts.push(previewCreator);
    }
    if (settings.download_group_by_post) {
      parts.push(previewPost);
    }
    parts.push(previewFilename);
    return parts.join('/');
  });

  let copiedPreview = $state(false);
  async function copyPreviewPath() {
    try {
      await navigator.clipboard.writeText(fullDownloadPathPreview);
      copiedPreview = true;
      notify.success(i18n.t('settings.copied_to_clipboard') || 'Copied to clipboard');
      setTimeout(() => {
        copiedPreview = false;
      }, 2000);
    } catch {
      // ignore
    }
  }
</script>

{#snippet categoryTabs()}
  <nav class="settings-categories" aria-label={i18n.t('settings.categories')}>
    {#each categories as category (category.id)}
      <Button
        variant={activeCategory === category.id ? 'accent' : 'ghost'}
        onclick={() => openCategory(category.id)}
        class="settings-category-btn"
        data-settings-category={category.id}
      >
        {i18n.t(category.labelKey)}
      </Button>
    {/each}
  </nav>
{/snippet}

{#snippet settingsMenu(source: 'main' | 'sticky')}
  {#if source === 'sticky'}
    <PopoverMenu
      bind:open={stickySettingsMenuOpen}
      title={i18n.t('settings.actions')}
      icon={IconMoreHorizontal}
      width="280px"
    >
      <button
        type="button"
        class="settings-menu-option"
        disabled={resetPending}
        onclick={() => {
          stickySettingsMenuOpen = false;
          showResetConfirm = true;
        }}
      >
        <span class="settings-menu-option-icon">
          {#if resetPending}<IconLoading />{:else}<IconArrowReset />{/if}
        </span>
        <span>
          <strong>{i18n.t('settings.reset_all')}</strong>
          <small>{i18n.t('settings.reset_all_desc')}</small>
        </span>
      </button>

      <div class="my-1 border-t border-white/[0.06]"></div>

      <button
        type="button"
        class="settings-menu-option text-red-400 hover:text-red-300 hover:bg-red-500/10"
        disabled={wipePending}
        onclick={() => {
          stickySettingsMenuOpen = false;
          showWipeConfirm = true;
        }}
      >
        <span class="settings-menu-option-icon text-red-400">
          {#if wipePending}<IconLoading />{:else}<IconDelete />{/if}
        </span>
        <span>
          <strong class="text-red-400">{i18n.t('settings.wipe_all_data')}</strong>
          <small class="text-red-300/60">{i18n.t('settings.wipe_all_data_desc')}</small>
        </span>
      </button>
    </PopoverMenu>
  {:else}
    <PopoverMenu
      bind:open={settingsMenuOpen}
      title={i18n.t('settings.actions')}
      icon={IconMoreHorizontal}
      width="280px"
    >
      <button
        type="button"
        class="settings-menu-option"
        disabled={resetPending}
        onclick={() => {
          settingsMenuOpen = false;
          showResetConfirm = true;
        }}
      >
        <span class="settings-menu-option-icon">
          {#if resetPending}<IconLoading />{:else}<IconArrowReset />{/if}
        </span>
        <span>
          <strong>{i18n.t('settings.reset_all')}</strong>
          <small>{i18n.t('settings.reset_all_desc')}</small>
        </span>
      </button>

      <div class="my-1 border-t border-white/[0.06]"></div>

      <button
        type="button"
        class="settings-menu-option text-red-400 hover:text-red-300 hover:bg-red-500/10"
        disabled={wipePending}
        onclick={() => {
          settingsMenuOpen = false;
          showWipeConfirm = true;
        }}
      >
        <span class="settings-menu-option-icon text-red-400">
          {#if wipePending}<IconLoading />{:else}<IconDelete />{/if}
        </span>
        <span>
          <strong class="text-red-400">{i18n.t('settings.wipe_all_data')}</strong>
          <small class="text-red-300/60">{i18n.t('settings.wipe_all_data_desc')}</small>
        </span>
      </button>
    </PopoverMenu>
  {/if}
{/snippet}

{#snippet authorBuildBar()}
  <div class="flex flex-col items-center justify-center text-center w-full mt-2">
    <div class="flex items-center justify-center gap-2 text-[16px] font-bold text-white/95 font-outfit">
      <span>{i18n.t('settings.made_with')}</span>
      <IconHeart class="w-4 h-4 text-[var(--color-danger,#f43f5e)] fill-current shrink-0" />
      <span>{i18n.t('settings.by_nichind')}</span>
    </div>
    <span class="text-[13px] text-white/50 mt-1 mb-3 font-outfit select-none">
      {i18n.t('settings.check_out_my_pages')}
    </span>

    <div class="grid grid-cols-2 gap-2.5 w-full">
      <Button
        variant="ghost"
        size="md"
        class="w-full justify-center gap-2.5 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
        onclick={() => openExternalUrl('https://nichind.dev')}
      >
        <svg viewBox="0 0 106 78" fill="currentColor" class="w-5 h-4 opacity-70 shrink-0">
          <path d="M106 78H71.7471L30.3184 30.6006V78H0V0H41.4277L106 78ZM106 24.375H87.873L67.7383 0H106V24.375Z" />
        </svg>
        <span class="truncate">nichind.dev</span>
      </Button>

      <Button
        variant="ghost"
        size="md"
        class="w-full justify-center gap-2.5 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
        onclick={() => openExternalUrl('https://github.com/nichind')}
      >
        <IconGithub class="w-5 h-5 opacity-70 shrink-0" />
        <span class="truncate">GitHub</span>
      </Button>
    </div>
  </div>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey}>
  {#snippet overlay()}
    <StickyHeader threshold={120}>
      <div class="sticky-settings-toolbar">
        {@render categoryTabs()}
        {@render settingsMenu('sticky')}
      </div>
    </StickyHeader>
  {/snippet}

  <div class="settings-page">
    <div class="settings-toolbar">
      {@render categoryTabs()}
      {@render settingsMenu('main')}
    </div>

    {#if !layoutState.isMobile}
      {@render authorBuildBar()}
    {/if}

    {#if layoutState.isMobile}
      <div class="flex flex-col gap-3.5 w-full">
        <button
          type="button"
          class="mobile-profile-hero"
          onclick={() => navigationState.navigateRoot('profile')}
        >
          <div class="mobile-hero-grid">
            <div class="mobile-hero-pillar">
              <div class="relative flex items-center justify-center">
                <svg viewBox="0 0 602 602" fill="none" class="w-10 h-10" xmlns="http://www.w3.org/2000/svg">
                  <defs>
                    <linearGradient id="logo-settings-hero" x1="301" y1="0" x2="-2.17166e-05" y2="584.337" gradientUnits="userSpaceOnUse">
                      <stop stop-color="#FCD8D2"/>
                      <stop offset="1" stop-color="#FEB8AD"/>
                    </linearGradient>
                  </defs>
                  <g transform="translate(0, 8.5)">
                    <path fill="url(#logo-settings-hero)" d="M130.548 56.3212L414.821 178.14L301 226.902L18.361 105.771C24.725 99.2782 32.508 94.0322 41.366 90.6352L130.548 56.3212ZM188.082 34.2192L254.732 8.59119C284.529 -2.86373 317.514 -2.86373 347.311 8.59119L560.677 90.6352C569.492 94.0752 577.275 99.2352 583.639 105.771L469.431 154.705L188.082 34.2192ZM601.742 144.815L322.5 264.484V584.834C330.957 583.401 339.227 581.136 347.311 578.04L560.677 495.953C572.841 491.269 583.301 483.01 590.677 472.264C598.054 461.517 602.002 448.788 602 435.753V150.835C602 148.829 601.9 146.822 601.699 144.815M279.5 584.834V264.484L0.300999 144.815C0.130354 146.818 0.0299598 148.826 0 150.835V435.753C0.00172613 448.793 3.95568 461.526 11.3404 472.273C18.7252 483.02 29.1939 491.276 41.366 495.953L254.689 578.04C262.773 581.136 271.043 583.401 279.5 584.834Z" />
                  </g>
                </svg>
                <span
                  class="mobile-hero-dot"
                  class:active={syncDotStatus === 'active'}
                  class:locked={syncDotStatus === 'locked'}
                  class:syncing={syncDotStatus === 'syncing'}
                  class:offline={syncDotStatus === 'offline'}
                ></span>
              </div>
              <div class="mobile-pillar-meta">
                <span class="mobile-pillar-tag">{i18n.t('profile.pawstash_sync')}</span>
                <span class="mobile-pillar-name truncate">{cloudAccountName}</span>
                <span class="mobile-pillar-sub truncate">
                  {#if syncState.status.configured}
                    {#if syncState.busy}
                      {i18n.t('sync.status_syncing')}
                    {:else if !syncState.status.unlocked}
                      {i18n.t('sync.locked')}
                    {:else}
                      rev {syncState.status.revision} · {i18n.t('profile.synced')}
                    {/if}
                  {:else}
                    {i18n.t('profile.offline_session')}
                  {/if}
                </span>
              </div>
            </div>

            <div class="mobile-hero-divider"></div>

            <div class="mobile-hero-pillar">
              <div class="relative flex items-center justify-center">
                <img src={pawchiveLogo} alt="Pawchive" class="w-10 h-10 object-contain" />
                <span
                  class="mobile-hero-dot"
                  class:active={accountState.session.authenticated}
                  class:offline={!accountState.session.authenticated}
                ></span>
              </div>
              <div class="mobile-pillar-meta">
                <span class="mobile-pillar-tag">Pawchive</span>
                <span class="mobile-pillar-name truncate">
                  {accountState.session.authenticated && accountState.session.username
                    ? `@${accountState.session.username}`
                    : i18n.t('profile.not_connected')}
                </span>
                <span class="mobile-pillar-sub truncate">
                  {#if accountState.session.authenticated && (configState.settings.sync_pawchive_session || syncState.status.configured)}
                    {i18n.t('profile.favorites_synced')}
                  {:else}
                    {i18n.t('profile.local_favorites')}
                  {/if}
                </span>
              </div>
            </div>
          </div>

          <div class="mobile-hero-footer">
            <span>{i18n.t('profile.title')}</span>
            <IconChevronRight class="w-3.5 h-3.5 text-[var(--accent)]" />
          </div>
        </button>

        <div class="flex items-center justify-center gap-3 w-full mt-0.5">
          <div class="h-[1px] flex-1 bg-white/[0.06]"></div>
          <span class="text-[11px] font-semibold uppercase tracking-wider text-white/35 font-outfit select-none">
            {i18n.t('settings.community')}
          </span>
          <div class="h-[1px] flex-1 bg-white/[0.06]"></div>
        </div>

        <div class="grid grid-cols-2 gap-2.5 w-full">
          <Button
            variant="ghost"
            size="md"
            class="w-full justify-center gap-2.5 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
            onclick={() => openExternalUrl('https://t.me/pawstashapp')}
          >
            <IconTelegram class="w-5 h-5 opacity-70 shrink-0" />
            <span class="truncate">Telegram</span>
          </Button>

          <Button
            variant="ghost"
            size="md"
            class="w-full justify-center gap-2.5 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
            onclick={() => openExternalUrl('https://discord.gg/ahcx8ub5Ck')}
          >
            <IconDiscord class="w-5 h-5 opacity-70 shrink-0" />
            <span class="truncate">Discord</span>
          </Button>

          <Button
            variant="ghost"
            size="md"
            class="w-full justify-center gap-2.5 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
            onclick={() => openExternalUrl('https://reddit.com/r/pawstash')}
          >
            <IconReddit class="w-5 h-5 opacity-70 shrink-0" />
            <span class="truncate">r/pawstash</span>
          </Button>

          <Button
            variant="ghost"
            size="md"
            class="w-full justify-center gap-2.5 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
            onclick={() => openExternalUrl('https://github.com/pawstash')}
          >
            <IconGithub class="w-5 h-5 opacity-70 shrink-0" />
            <span class="truncate">{i18n.t('settings.contribute')}</span>
          </Button>
        </div>

        {@render authorBuildBar()}
      </div>
    {/if}

    <div id="settings-appearance" class="settings-section">
      <SectionTitle icon={IconTranslate} title={i18n.t('settings.appearance_section')} onreset={() => resetSection('appearance')} />

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-16 gap-y-6">
        <SettingItem
          title={i18n.t('settings.language')}
          icon={IconTranslate}
        >
          <SegmentedControl
            options={[
              { value: 'en', label: 'English', icon: IconFlagUs },
              { value: 'ru', label: 'Русский', icon: IconFlagRu }
            ]}
            value={i18n.currentLocale}
            onchange={(val) => i18n.setLocale(val as any)}
            tabWidth={115}
          />
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.accent_color')}
          description={i18n.t('settings.accent_color_desc')}
          icon={IconPaint}
        >
          <div class="settings-accent-controls">
            {#each accentColors as c}
              <button
                type="button"
                use:ripple
                onclick={() => themeState.setAccent(c.id)}
                class="w-[24px] h-[24px] rounded-full transition-all duration-300 relative hover:scale-115 active:scale-90"
                style="
                  background-color: {c.color};
                  transform: {themeState.tokens.accent === c.id ? 'scale(1.15)' : 'scale(1)'};
                  border: {themeState.tokens.accent === c.id ? '2px solid #111215' : 'none'};
                  box-shadow: {themeState.tokens.accent === c.id ? '0 0 0 2px ' + c.color : 'none'};
                "
                aria-label={i18n.t('settings.set_accent', { color: c.label })}
              ></button>
            {/each}

            <button
              type="button"
              use:ripple
              onclick={() => themeState.setAccent('rgb')}
              class="w-[24px] h-[24px] rounded-full transition-all duration-300 relative hover:scale-115 active:scale-90"
              style="
                background: linear-gradient(135deg, #ff3b30, #ff9500, #ffcc00, #34c759, #007aff, #af52de);
                transform: {themeState.tokens.accent === 'rgb' ? 'scale(1.15)' : 'scale(1)'};
                border: {themeState.tokens.accent === 'rgb' ? '2px solid #111215' : 'none'};
                box-shadow: {themeState.tokens.accent === 'rgb' ? '0 0 0 2px var(--accent-primary)' : 'none'};
              "
              aria-label={i18n.t('settings.rgb_accent')}
            ></button>

            <div class="w-[1px] h-6 bg-white/10 mx-1"></div>

            <div class="relative w-[24px] h-[24px] flex items-center justify-center">
              <div
                class="w-[24px] h-[24px] rounded-full transition-all duration-300 relative pointer-events-none"
                style="
                  background: {isCustomActive ? themeState.tokens.accent : 'conic-gradient(from 0deg, #f43f5e, #f59e0b, #10b981, #06b6d4, #6366f1, #8b5cf6, #f43f5e)'};
                  transform: {isCustomActive ? 'scale(1.15)' : 'scale(1)'};
                  border: {isCustomActive ? '2px solid #111215' : 'none'};
                  box-shadow: {isCustomActive ? '0 0 0 2px ' + themeState.tokens.accent : 'none'};
                "
              ></div>
              <input
                type="color"
                value={isCustomActive ? themeState.tokens.accent : '#8b5cf6'}
                onclick={(e) => themeState.setAccent(e.currentTarget.value)}
                oninput={(e) => themeState.setAccent(e.currentTarget.value)}
                class="absolute inset-0 opacity-0 cursor-pointer w-full h-full z-10"
                aria-label={i18n.t('settings.custom_accent')}
              />
            </div>
          </div>
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.dynamic_accent')}
          description={i18n.t('settings.dynamic_accent_desc')}
          icon={IconPaint}
          align="right"
        >
          <SegmentedControl
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.dynamic_accent}
            onchange={(value) => updateAndSaveSetting('dynamic_accent', value)}
          />
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.sticky_header')}
          description={i18n.t('settings.sticky_header_desc')}
          icon={IconEye}
          align="right"
        >
          <SegmentedControl
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.sticky_header}
            onchange={(value) => updateAndSaveSetting('sticky_header', value)}
          />
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.scroll_edge_mask')}
          description={i18n.t('settings.scroll_edge_mask_desc')}
          icon={IconSparkle}
          align="right"
        >
          <SegmentedControl
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.scroll_edge_mask ?? true}
            onchange={(value) => updateAndSaveSetting('scroll_edge_mask', value)}
          />
        </SettingItem>

        {#if !layoutState.isMobile}
          <SettingItem
            title={i18n.t('settings.titlebar_style')}
            description={i18n.t('settings.titlebar_style_desc')}
            icon={IconSparkle}
            align="right"
          >
            <SegmentedControl
              options={[
                { value: 'auto', label: i18n.t('settings.titlebar_style_auto') },
                { value: 'windows', label: 'Windows' },
                { value: 'macos', label: 'macOS' }
              ]}
              value={settings.titlebar_style || 'auto'}
              onchange={(value) => updateAndSaveSetting('titlebar_style', value)}
            />
          </SettingItem>
        {/if}

        <SettingItem
          title={i18n.t('settings.toast_position')}
          description={i18n.t('settings.toast_position_desc')}
          icon={IconEye}
        >
          <div class="w-full">
            <Select
              options={toastPositionOptions}
              value={settings.toast_position || 'auto'}
              onchange={(val) => updateAndSaveSetting('toast_position', val)}
            />
          </div>
        </SettingItem>
      </div>
    </div>

    <div id="settings-grid" class="settings-section">
      <SectionTitle icon={IconGrid} title={i18n.t('settings.grid_section')} onreset={() => resetSection('grid')} />

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-16 gap-y-6">
        <SettingItem
          title={i18n.t('settings.grid_scale')}
          description={i18n.t('settings.grid_scale_desc')}
          icon={IconGrid}
        >
          <div class="flex items-center gap-4 w-full">
            <Slider
              min={60}
              max={160}
              value={settings.grid_scale}
              oninput={(value) => updateAndSaveSetting('grid_scale', Math.round(value / 5) * 5)}
            />
            <span class="text-sm font-mono text-gray-300 w-12 text-right shrink-0">{settings.grid_scale}%</span>
          </div>
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.grid_ratio')}
          description={i18n.t('settings.grid_ratio_desc')}
          icon={IconGrid}
          align="right"
        >
          <SegmentedControl
            compact={true}
            options={[
              { value: 'square', label: '1:1' },
              { value: 'portrait', label: '4:5' },
              { value: 'landscape', label: '3:2' },
              { value: 'widescreen', label: '16:9' }
            ]}
            value={settings.grid_aspect_ratio}
            onchange={(value) => updateAndSaveSetting('grid_aspect_ratio', value)}
          />
        </SettingItem>
      </div>
    </div>

    <div id="settings-proxy" class="settings-section">
      <SectionTitle icon={IconGlobe} title={i18n.t('settings.proxy_section')} onreset={() => resetSection('proxy')} />

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-16 gap-y-6">
        <SettingItem
          title={i18n.t('settings.proxy_mode')}
          description={i18n.t('settings.proxy_mode_desc')}
          icon={IconGlobe}
          align="right"
        >
          <SegmentedControl
            options={[
              { value: 'none', label: i18n.t('settings.proxy_none'), icon: IconDismiss },
              { value: 'system', label: i18n.t('settings.proxy_system'), icon: IconGlobe },
              { value: 'custom', label: i18n.t('settings.proxy_custom'), icon: IconKey }
            ]}
            value={settings.proxy_mode}
            onchange={(val) => updateAndSaveSetting('proxy_mode', val)}
            compact={true}
          />
        </SettingItem>

        {#if settings.proxy_mode === 'custom'}
          <SettingItem title={i18n.t('settings.proxy_url')} description={i18n.t('settings.proxy_url_desc')} icon={IconGlobe}>
            <div class="w-full">
              <Input
                placeholder="http://127.0.0.1:8080"
                bind:value={settings.proxy_url}
                onblur={() => updateAndSaveSetting('proxy_url', settings.proxy_url)}
              />
            </div>
          </SettingItem>

          <SettingItem title={i18n.t('settings.proxy_username')} description={i18n.t('settings.proxy_username_desc')} icon={IconKey}>
            <div class="w-full">
              <Input
                placeholder={i18n.t('settings.optional')}
                bind:value={settings.proxy_username}
                onblur={() => updateAndSaveSetting('proxy_username', settings.proxy_username)}
              />
            </div>
          </SettingItem>

          <SettingItem title={i18n.t('settings.proxy_password')} description={i18n.t('settings.proxy_password_desc')} icon={IconKey}>
            <div class="w-full">
              <Input
                type="password"
                placeholder={i18n.t('settings.optional')}
                bind:value={settings.proxy_password}
                onblur={() => updateAndSaveSetting('proxy_password', settings.proxy_password)}
              />
            </div>
          </SettingItem>

          <SettingItem
            title={i18n.t('settings.proxy_bypass_local')}
            description={i18n.t('settings.proxy_bypass_local_desc')}
            icon={IconGlobe}
            align="right"
          >
            <SegmentedControl
              options={[
                { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
                { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
              ]}
              value={settings.proxy_bypass_local}
              onchange={(value) => updateAndSaveSetting('proxy_bypass_local', value)}
            />
          </SettingItem>
        {/if}
      </div>
    </div>

    <div id="settings-background" class="settings-section">
      <SectionTitle icon={IconPaint} title={i18n.t('settings.background_section')} onreset={() => resetSection('background')} />

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-16 gap-y-6">
        <SettingItem
          title={i18n.t('settings.bg_type')}
          description={i18n.t('settings.bg_type_desc')}
          icon={IconPaint}
        >
          <Select
            options={bgTypes.map((t) => ({ value: t.id, label: t.label }))}
            value={backgroundState.settings.type}
            onchange={(val) => backgroundState.setType(val as BackgroundType)}
          />
        </SettingItem>

        {#if backgroundState.settings.type === 'custom'}
          <SettingItem title={i18n.t('settings.background_source')} description={i18n.t('settings.background_source_desc')} icon={IconPaint}>
            <Select
              options={[
                { value: 'color', label: i18n.t('settings.background_source_color') },
                { value: 'image', label: i18n.t('settings.background_source_image') },
                { value: 'video', label: i18n.t('settings.background_source_video') }
              ]}
              value={backgroundState.settings.customKind}
              onchange={(value) => backgroundState.setCustomKind(value as CustomBackgroundKind)}
            />
          </SettingItem>

          {#if backgroundState.settings.customKind === 'color'}
            <SettingItem title={i18n.t('settings.background_primary')} description={i18n.t('settings.background_primary_desc')} icon={IconPaint}>
              <input class="background-color-input" type="color" value={backgroundState.settings.solidColor} oninput={(event) => backgroundState.setSolidColor(event.currentTarget.value)} />
            </SettingItem>

            <SettingItem title={i18n.t('settings.background_secondary')} description={i18n.t('settings.background_secondary_desc')} icon={IconPaint}>
              <input class="background-color-input" type="color" value={backgroundState.settings.gradientSecondary} oninput={(event) => backgroundState.setGradientSecondary(event.currentTarget.value)} />
            </SettingItem>
          {:else if backgroundState.settings.customKind === 'image'}
            <SettingItem title={i18n.t('settings.background_image')} description={i18n.t('settings.background_image_desc')} icon={IconFolder}>
              <Input placeholder={i18n.t('settings.background_image_placeholder')} value={backgroundState.settings.imageUrl} readonly={true} onBrowse={() => void selectCustomBackground('image')} />
            </SettingItem>
          {:else}
            <SettingItem title={i18n.t('settings.background_video')} description={i18n.t('settings.background_video_desc')} icon={IconFolder}>
              <Input placeholder={i18n.t('settings.background_video_placeholder')} value={backgroundState.settings.videoUrl} readonly={true} onBrowse={() => void selectCustomBackground('video')} />
            </SettingItem>
          {/if}

          {#if backgroundState.settings.customKind !== 'color'}
            <SettingItem title={i18n.t('settings.background_blur')} description={i18n.t('settings.background_blur_desc')} icon={IconEye}>
              <div class="flex items-center gap-4 w-full">
                <Slider min={0} max={40} value={backgroundState.settings.blurPx} oninput={(value) => backgroundState.setBlur(value)} />
                <span class="text-sm font-mono text-gray-300 w-10 text-right shrink-0">{backgroundState.settings.blurPx}px</span>
              </div>
            </SettingItem>

          <SettingItem title={i18n.t('settings.background_opacity')} description={i18n.t('settings.background_opacity_desc')} icon={IconEye}>
            <div class="flex items-center gap-4 w-full">
              <Slider min={0.1} max={1} step={0.05} value={backgroundState.settings.opacity} oninput={(value) => backgroundState.setOpacity(value)} />
              <span class="text-sm font-mono text-gray-300 w-12 text-right shrink-0">{Math.round(backgroundState.settings.opacity * 100)}%</span>
            </div>
          </SettingItem>

          <SettingItem title={i18n.t('settings.background_brightness')} description={i18n.t('settings.background_brightness_desc')} icon={IconEye}>
            <div class="flex items-center gap-4 w-full">
              <Slider min={0.2} max={1.5} step={0.05} value={backgroundState.settings.brightness} oninput={(value) => backgroundState.setBrightness(value)} />
              <span class="text-sm font-mono text-gray-300 w-12 text-right shrink-0">{Math.round(backgroundState.settings.brightness * 100)}%</span>
            </div>
          </SettingItem>

          <SettingItem title={i18n.t('settings.background_saturation')} description={i18n.t('settings.background_saturation_desc')} icon={IconPaint}>
            <div class="flex items-center gap-4 w-full">
              <Slider min={0} max={2} step={0.05} value={backgroundState.settings.saturation} oninput={(value) => backgroundState.setSaturation(value)} />
              <span class="text-sm font-mono text-gray-300 w-12 text-right shrink-0">{Math.round(backgroundState.settings.saturation * 100)}%</span>
            </div>
          </SettingItem>

          {#if (backgroundState.settings.customKind === 'image' && backgroundState.settings.imageUrl) || (backgroundState.settings.customKind === 'video' && backgroundState.settings.videoUrl)}
            <SettingItem title={i18n.t('settings.background_media')} description={i18n.t('settings.background_media_desc')} icon={IconDelete}>
              <Button variant="ghost" onclick={() => backgroundState.clearCustomMedia(backgroundState.settings.customKind as 'image' | 'video')}>
                <IconDelete />
                {i18n.t('settings.background_media_clear')}
              </Button>
            </SettingItem>
          {/if}
          {/if}
        {/if}
      </div>
    </div>

    <div id="settings-network" class="settings-section">
      <SectionTitle icon={IconGlobe} title={i18n.t('settings.network_section')} onreset={() => resetSection('network')} />

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-16 gap-y-6">
        <SettingItem
          title={i18n.t('settings.api_domain')}
          description={i18n.t('settings.api_domain_desc')}
          icon={IconGlobe}
        >
          <div class="w-full">
            <Input
              placeholder="api.example.com"
              bind:value={settings.api_domain}
              onblur={() => updateAndSaveSetting('api_domain', settings.api_domain)}
            />
          </div>
        </SettingItem>

        <SettingItem title={i18n.t('settings.file_domain')} description={i18n.t('settings.file_domain_desc')} icon={IconGlobe}>
          <div class="w-full">
            <Input
              placeholder="files.example.com"
              bind:value={settings.file_domain}
              onblur={() => updateAndSaveSetting('file_domain', settings.file_domain)}
            />
          </div>
        </SettingItem>

        <SettingItem title={i18n.t('settings.image_domain')} description={i18n.t('settings.image_domain_desc')} icon={IconGlobe}>
          <div class="w-full">
            <Input
              placeholder="images.example.com"
              bind:value={settings.image_domain}
              onblur={() => updateAndSaveSetting('image_domain', settings.image_domain)}
            />
          </div>
        </SettingItem>
      </div>
    </div>

    <div id="settings-downloads" class="settings-section">
      <SectionTitle icon={IconDownload} title={i18n.t('settings.download_section')} onreset={() => resetSection('downloads')} />

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-16 gap-y-6">
        <SettingItem
          title={i18n.t('settings.downloads_storage')}
          description={i18n.t('settings.downloads_storage_desc')}
          icon={IconDownload}
        >
          <DownloadsStatsBar downloads={downloadState.downloads} />
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.download_dir')}
          description={i18n.t('settings.download_dir_desc')}
          icon={IconFolder}
        >
          <div class="w-full">
            <Input
              placeholder="Downloads/Pawstash"
              bind:value={settings.download_dir}
              onblur={() => updateAndSaveSetting('download_dir', settings.download_dir)}
              onBrowse={selectDownloadDir}
            />
          </div>
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.download_group_by_creator')}
          description={i18n.t('settings.download_group_by_creator_desc')}
          icon={IconFolder}
          align="right"
        >
          <SegmentedControl
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.download_group_by_creator}
            onchange={(val) => updateAndSaveSetting('download_group_by_creator', val)}
          />
        </SettingItem>

        {#if settings.download_group_by_creator}
          <SettingItem
            title={i18n.t('settings.download_creator_template')}
            description={i18n.t('settings.download_creator_template_desc')}
            icon={IconFolder}
          >
            <div class="w-full">
              <TemplateInput
                placeholder="&#123;creator&#125;"
                bind:value={settings.download_creator_folder_template}
                tags={creatorFolderTags}
                previewType="creator"
                onchange={(val) => updateAndSaveSetting('download_creator_folder_template', val)}
              />
            </div>
          </SettingItem>
        {/if}

        <SettingItem
          title={i18n.t('settings.download_group_by_post')}
          description={i18n.t('settings.download_group_by_post_desc')}
          icon={IconFolder}
          align="right"
        >
          <SegmentedControl
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.download_group_by_post}
            onchange={(val) => updateAndSaveSetting('download_group_by_post', val)}
          />
        </SettingItem>

        {#if settings.download_group_by_post}
          <SettingItem
            title={i18n.t('settings.download_post_template')}
            description={i18n.t('settings.download_post_template_desc')}
            icon={IconFolder}
          >
            <div class="w-full">
              <TemplateInput
                placeholder="&#123;post_title&#125;"
                bind:value={settings.download_post_folder_template}
                tags={postFolderTags}
                previewType="post"
                onchange={(val) => updateAndSaveSetting('download_post_folder_template', val)}
              />
            </div>
          </SettingItem>
        {/if}

        <SettingItem
          title={i18n.t('settings.download_filename_template')}
          description={i18n.t('settings.download_filename_template_desc')}
          icon={IconDownload}
        >
          <div class="w-full">
            <TemplateInput
              placeholder="&#123;post_title&#125; - &#123;filename&#125;"
              bind:value={settings.download_filename_template}
              tags={filenameTags}
              previewType="file"
              onchange={(val) => updateAndSaveSetting('download_filename_template', val)}
            />
          </div>
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.aria2c_engine')}
          description={i18n.t('settings.aria2c_engine_desc')}
          icon={IconDownload}
          align="right"
        >
          <SegmentedControl
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.use_aria2c}
            onchange={(val) => updateAndSaveSetting('use_aria2c', val)}
          />
        </SettingItem>

        {#if settings.use_aria2c}
          <SettingItem
            title={i18n.t('settings.parallel_connections')}
            description={i18n.t('settings.parallel_connections_desc')}
            icon={IconDownload}
          >
            <div class="flex items-center gap-4 w-full">
              <Slider
                min={1}
                max={32}
                value={settings.aria2_connections}
                oninput={(value) => updateAndSaveSetting('aria2_connections', value)}
              />
              <span class="text-sm font-mono text-gray-300 w-8 text-right shrink-0">{settings.aria2_connections}</span>
            </div>
          </SettingItem>
        {/if}

        <SettingItem
          title={i18n.t('settings.template_preview')}
          description={i18n.t('settings.download_preview_desc') || 'Resolved destination path for saved files'}
          icon={IconFolder}
          class="col-span-full"
        >
          <div class="w-full relative flex items-center">
            <div
              class="w-full h-[46px] px-4 pr-12 rounded-full border flex items-center select-all cursor-text font-mono text-[13px] overflow-hidden whitespace-nowrap text-ellipsis"
              style="background: var(--bg-card); border-color: var(--border-color); color: var(--text-secondary);"
            >
              {fullDownloadPathPreview}
            </div>
            <button
              type="button"
              use:ripple
              onclick={copyPreviewPath}
              style="position: absolute; right: 14px; top: 50%; transform: translateY(-50%); width: 20px; height: 20px; border: none; outline: none; background: transparent; display: flex; align-items: center; justify-content: center; color: white; opacity: 0.45; cursor: pointer; z-index: 10; padding: 0; transition: opacity 200ms ease, color 200ms ease;"
              onmouseenter={(e) => { e.currentTarget.style.opacity = '0.9'; }}
              onmouseleave={(e) => { e.currentTarget.style.opacity = '0.45'; }}
              title={copiedPreview ? (i18n.t('common.copied') || 'Copied') : (i18n.t('common.copy') || 'Copy')}
              aria-label="Copy path"
            >
              {#if copiedPreview}
                <IconCheck style="width: 18px; height: 18px; color: var(--accent);" />
              {:else}
                <IconCopy style="width: 18px; height: 18px;" />
              {/if}
            </button>
          </div>
        </SettingItem>
      </div>
    </div>

    <div id="settings-cache" class="settings-section">
      <SectionTitle icon={IconDatabase} title={i18n.t('settings.cache_section')} onreset={() => resetSection('cache')} />

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-16 gap-y-6">
        <SettingItem title={i18n.t('settings.cache_usage')} description={i18n.t('settings.cache_usage_desc')} icon={IconDatabase}>
          {#if cacheStats}
            <StorageBar stats={cacheStats} limitMb={settings.cache_max_mb} {formatBytes} />
          {:else}
            <div class="flex items-center gap-2 py-3 text-white/50 text-xs">
              <IconLoading />
              <span>{i18n.t('settings.cache_section')}...</span>
            </div>
          {/if}
        </SettingItem>

        <SettingItem title={i18n.t('settings.cache_limit')} description={i18n.t('settings.cache_limit_desc')} icon={IconDatabase}>
          <Select
            options={[
              { value: 64, label: '64 MB' },
              { value: 128, label: '128 MB' },
              { value: 256, label: '256 MB' },
              { value: 512, label: '512 MB' },
              { value: 1024, label: '1 GB' },
              { value: 2048, label: '2 GB' }
            ]}
            value={settings.cache_max_mb}
            onchange={(value) => void updateAndSaveSetting('cache_max_mb', Number(value))}
          />
        </SettingItem>

        <SettingItem title={i18n.t('settings.cache_clear')} description={i18n.t('settings.cache_clear_desc')} icon={IconDelete}>
          <Button
            variant="ghost"
            disabled={!!cacheBusy || !cacheStats || cacheStats.total_bytes === 0}
            onclick={() => void clearCache('images')}
          >
            {#if cacheBusy === 'images'}<IconLoading />{:else}<IconDelete />{/if}
            {i18n.t('settings.cache_clear_action')}
          </Button>
        </SettingItem>

        <SettingItem title={i18n.t('settings.cache_clear_all')} description={i18n.t('settings.cache_clear_all_desc')} icon={IconDelete}>
          <Button
            variant="danger"
            disabled={!!cacheBusy || !cacheStats || cacheStats.total_bytes + cacheStats.metadata_bytes === 0}
            onclick={() => void clearCache('all')}
          >
            {#if cacheBusy === 'all'}<IconLoading />{:else}<IconDelete />{/if}
            {i18n.t('settings.cache_clear_all_action')}
          </Button>
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.wipe_all_data')}
          description={i18n.t('settings.wipe_all_data_desc')}
          icon={IconDelete}
        >
          <Button
            variant="danger"
            disabled={wipePending}
            onclick={() => (showWipeConfirm = true)}
          >
            {#if wipePending}<IconLoading class="mr-1.5" />{:else}<IconDelete class="mr-1.5" />{/if}
            {i18n.t('settings.wipe_all_data')}
          </Button>
        </SettingItem>
      </div>
    </div>

    <div id="settings-sync" class="settings-section">
      <SectionTitle icon={IconCloudSync} title={i18n.t('sync.title')} />
      <SyncSettings />
    </div>

    <div id="settings-updates" class="settings-section">
      <SectionTitle icon={IconSparkle} title={i18n.t('settings.updates_section')} onreset={() => resetSection('updates')} />

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-16 gap-y-6">
        <SettingItem
          title={i18n.t('settings.version_title')}
          description={i18n.t('settings.version_desc', { version: APP_VERSION })}
          icon={IconSparkle}
        >
          <div class="flex flex-wrap items-center gap-2.5">
            <Button
              variant="ghost"
              disabled={updateState.checking}
              onclick={() => updateState.check(false)}
            >
              {#if updateState.checking}
                <IconLoading class="w-4 h-4 mr-1.5 shrink-0" />
                <span>{i18n.t('settings.checking_updates')}</span>
              {:else}
                <IconArrowSync class="w-4 h-4 mr-1.5 shrink-0" />
                <span>{i18n.t('settings.check_updates_now')}</span>
              {/if}
            </Button>
            {#if updateState.info?.available}
              <Button
                variant="accent"
                onclick={() => updateState.openModal()}
              >
                <IconSparkle class="w-4 h-4 mr-1.5 shrink-0" />
                <span>{i18n.t('settings.update_ready_btn', { version: updateState.info.latest_version })}</span>
              </Button>
            {/if}
          </div>
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.auto_check_updates')}
          description={i18n.t('settings.auto_check_updates_desc')}
          icon={IconArrowSync}
          align="right"
        >
          <SegmentedControl
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.auto_check_updates ?? true}
            onchange={(val: boolean) => {
              settings.auto_check_updates = val;
              updateAndSaveSetting('auto_check_updates', val);
            }}
          />
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.update_channel')}
          description={i18n.t('settings.update_channel_desc')}
          icon={IconSparkle}
          align="right"
        >
          <SegmentedControl
            options={[
              { value: false, label: i18n.t('settings.channel_stable'), icon: IconCheck },
              { value: true, label: i18n.t('settings.channel_prerelease'), icon: IconSparkle }
            ]}
            value={settings.include_prereleases ?? false}
            onchange={(val: boolean) => {
              settings.include_prereleases = val;
              updateAndSaveSetting('include_prereleases', val);
            }}
          />
        </SettingItem>
      </div>
    </div>

    <div class="flex flex-col items-center justify-center text-center w-full pt-4 pb-6 opacity-40 select-none">
      <span class="text-[12px] font-mono tracking-wider font-semibold text-white/90">
        Pawstash v{APP_VERSION} ({COMMIT_HASH})
      </span>
      <span class="text-[11.5px] text-white/70 mt-0.5 font-outfit">
        {i18n.t('settings.built_on')} {formattedBuildTime}
      </span>
    </div>
  </div>
</PageShell>

<Modal
  isOpen={showResetConfirm}
  title={i18n.t('settings.reset_all')}
  size="md"
  onclose={() => (showResetConfirm = false)}
>
  <div class="flex flex-col gap-5 pt-1">
    <p class="text-sm text-white/70 leading-relaxed m-0">
      {i18n.t('settings.reset_all_desc')}
    </p>

    <div class="flex flex-col-reverse sm:grid sm:grid-cols-2 gap-2.5 sm:gap-3 w-full pt-3 border-t border-white/6">
      <Button
        variant="ghost"
        size="md"
        class="w-full justify-center px-4 border border-white/8 hover:border-white/16"
        disabled={resetPending}
        onclick={() => (showResetConfirm = false)}
      >
        <span class="truncate">{i18n.t('common.cancel')}</span>
      </Button>

      <Button
        variant="accent"
        size="md"
        class="w-full justify-center px-4"
        disabled={resetPending}
        onclick={() => void executeResetAllSettings()}
      >
        {#if resetPending}
          <IconLoading class="mr-2 shrink-0" />
        {:else}
          <IconArrowReset class="mr-2 shrink-0" />
        {/if}
        <span class="truncate">{i18n.t('settings.reset_all')}</span>
      </Button>
    </div>
  </div>
</Modal>

<Modal
  isOpen={showWipeConfirm}
  title={i18n.t('settings.wipe_all_data')}
  size="md"
  onclose={() => (showWipeConfirm = false)}
>
  <div class="flex flex-col gap-5 pt-1">
    <p class="text-sm text-white/70 leading-relaxed m-0">
      {i18n.t('settings.wipe_all_data_confirm')}
    </p>

    <div class="flex flex-col-reverse sm:grid sm:grid-cols-2 gap-2.5 sm:gap-3 w-full pt-3 border-t border-white/6">
      <Button
        variant="ghost"
        size="md"
        class="w-full justify-center px-4 border border-white/8 hover:border-white/16"
        disabled={wipePending}
        onclick={() => (showWipeConfirm = false)}
      >
        <span class="truncate">{i18n.t('common.cancel')}</span>
      </Button>

      <Button
        variant="danger"
        size="md"
        class="w-full justify-center px-4"
        disabled={wipePending}
        onclick={() => void executeWipeAllData()}
      >
        {#if wipePending}
          <IconLoading class="mr-2 shrink-0" />
        {:else}
          <IconDelete class="mr-2 shrink-0" />
        {/if}
        <span class="truncate">{i18n.t('settings.wipe_all_data')}</span>
      </Button>
    </div>
  </div>
</Modal>

<input
  bind:this={bgImageInput}
  type="file"
  accept="image/png,image/jpeg,image/webp,image/gif,image/avif"
  class="hidden"
  onchange={(e) => void handleFileInputChange(e, 'image')}
/>

<input
  bind:this={bgVideoInput}
  type="file"
  accept="video/mp4,video/webm"
  class="hidden"
  onchange={(e) => void handleFileInputChange(e, 'video')}
/>

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: 100%;
    min-width: 0;
    gap: 56px;
    padding-bottom: 48px;
    overflow-x: clip;
  }

  .settings-toolbar,
  .sticky-settings-toolbar {
    display: flex;
    align-items: center;
    width: 100%;
    min-width: 0;
    gap: 16px;
  }

  .settings-toolbar {
    min-height: 44px;
    margin-bottom: 0;
  }

  .settings-categories {
    display: flex;
    align-items: center;
    flex: 1 1 auto;
    gap: 8px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .settings-categories::-webkit-scrollbar {
    display: none;
  }

  :global(.settings-category-btn.btn) {
    height: 44px !important;
    min-height: 44px !important;
    padding: 0 18px !important;
    border-radius: var(--radius-full) !important;
    font-size: 13.5px !important;
    white-space: nowrap;
    flex: 0 0 auto;
  }



  .settings-section {
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: 100%;
    min-width: 0;
    gap: 16px;
    scroll-margin-top: 92px;
    overflow-wrap: anywhere;
  }

  .settings-section :global(.select-root) {
    max-width: 100% !important;
    min-width: 0 !important;
  }

  .background-color-input {
    width: 100%;
    min-width: 0;
    height: 46px;
    padding: 5px;
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-full);
    background: var(--bg-input);
    cursor: pointer;
  }

  .settings-accent-controls {
    display: flex;
    align-items: center;
    width: 100%;
    max-width: 100%;
    min-width: 0;
    flex-wrap: wrap;
  }

  .settings-accent-controls {
    min-height: 46px;
    gap: 16px;
  }

  @media (max-width: 900px) {
    .settings-toolbar,
    .sticky-settings-toolbar {
      gap: 10px;
    }
  }

  .mobile-profile-hero {
    display: none;
    width: 100%;
    padding: 8px 6px;
    background: transparent;
    border: none;
    outline: none;
    border-radius: var(--radius-lg, 12px);
    cursor: pointer;
    text-align: center;
    transition: all 180ms ease;
    box-sizing: border-box;
    flex-direction: column;
    gap: 8px;
    user-select: none;
  }

  @media (max-width: 768px) {
    .mobile-profile-hero {
      display: flex;
    }
  }

  .mobile-profile-hero:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .mobile-profile-hero:active {
    transform: scale(0.99);
  }

  .mobile-hero-grid {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    gap: 10px;
  }

  .mobile-hero-pillar {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    flex: 1;
    min-width: 0;
    gap: 6px;
  }

  .mobile-pillar-meta {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 0;
    max-width: 100%;
    gap: 2px;
  }

  .mobile-pillar-tag {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: rgba(255, 255, 255, 0.35);
  }

  .mobile-pillar-name {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text-primary, #ffffff);
    font-family: var(--font-outfit, inherit);
    max-width: 100%;
  }

  .mobile-pillar-sub {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.45);
    font-weight: 300;
    max-width: 100%;
  }

  .mobile-hero-divider {
    width: 1px;
    height: 38px;
    background: rgba(255, 255, 255, 0.08);
    flex-shrink: 0;
  }

  .mobile-hero-footer {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--accent);
    padding-top: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    width: 100%;
  }

  .mobile-hero-dot {
    position: absolute;
    bottom: -1px;
    right: -1px;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    border: 1.5px solid var(--bg-surface, #121214);
  }

  .mobile-hero-dot.active {
    background: var(--color-success, #34d399);
  }

  .mobile-hero-dot.locked {
    background: var(--color-warning, #fbbf24);
  }

  .mobile-hero-dot.syncing {
    background: var(--accent, #f43f5e);
  }

  .mobile-hero-dot.offline {
    background: rgba(255, 255, 255, 0.3);
  }

  @media (max-width: 640px) {
    .settings-page {
      gap: 44px;
      padding-bottom: 24px;
    }

    .settings-toolbar,
    .sticky-settings-toolbar {
      align-items: center;
    }

    :global(.settings-category-btn.btn) {
      padding: 0 14px !important;
      font-size: 13px !important;
    }
  }

</style>
