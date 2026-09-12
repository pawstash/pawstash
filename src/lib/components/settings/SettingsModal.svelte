<script lang="ts">
  import { configState } from '$lib/state/configState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { onMount, onDestroy } from 'svelte';
  import SettingsNav from './SettingsNav.svelte';
  import {
    backgroundState,
    defaultBackgroundType,
    supportedBackgroundTypes,
    type BackgroundType,
    type CustomBackgroundKind
  } from '$lib/theme/backgroundState.svelte';
  import type { AppSettings } from '$lib/types/config';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { i18n, LOCALES, type Locale } from '$lib/i18n';
  import {
    apiClearContentCache,
    apiClearAllContentCache,
    apiGetCacheStats,
    apiGetDefaultSettings,
    apiGetSettings,
    apiSaveSettings,
    apiUpdatePanicKey,
    openExternalUrl,
    apiWipeAllData,
    type CacheStats
  } from '$lib/utils/ipc';
  import { formatBytes } from '$lib/utils/formatters';
  import { formatProviderName } from '$lib/utils/media';
  import { APP_VERSION, BUILD_TIME, COMMIT_HASH } from '$lib/version';
  import { notify } from '$lib/utils/toast';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import SectionTitle from '$lib/components/layout/SectionTitle.svelte';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import ChoiceGroup from '$lib/components/ui/ChoiceGroup.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import Slider from '$lib/components/ui/Slider.svelte';
  import NumberStepper from '$lib/components/ui/NumberStepper.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import TemplateInput, { type TemplateTag } from '$lib/components/ui/TemplateInput.svelte';
  import ShortcutInput from '$lib/components/ui/ShortcutInput.svelte';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconKey from '~icons/fluent/key-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconPaint from '~icons/fluent/color-24-regular';
  import IconEye from '~icons/fluent/eye-24-regular';
  import IconTranslate from '~icons/fluent/translate-24-regular';
  import IconTextFont from '~icons/fluent/text-font-24-regular';
  import IconFlagUs from '~icons/circle-flags/us';
  import IconFlagRu from '~icons/circle-flags/ru';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconGrid from '~icons/fluent/grid-24-regular';
  import IconDatabase from '~icons/fluent/database-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconPaintBrush from '~icons/fluent/paint-brush-24-regular';
  import IconEyedropper from '~icons/fluent/eyedropper-24-regular';
  import IconDockRow from '~icons/fluent/dock-row-24-regular';
  import IconBlur from '~icons/fluent/blur-24-regular';
  import IconImageOff from '~icons/fluent/image-off-24-regular';
  import IconWindowApps from '~icons/fluent/window-apps-24-regular';
  import IconAlert from '~icons/fluent/alert-24-regular';
  import IconSpeaker2 from '~icons/fluent/speaker-2-24-regular';
  import IconWarning from '~icons/fluent/warning-24-regular';
  import IconKeyboard from '~icons/fluent/keyboard-24-regular';
  import IconWallpaper from '~icons/fluent/wallpaper-24-regular';
  import IconColorFill from '~icons/fluent/color-fill-24-regular';
  import IconCircleHalfFill from '~icons/fluent/circle-half-fill-24-regular';
  import IconBrightnessHigh from '~icons/fluent/brightness-high-24-regular';
  import IconZoomIn from '~icons/fluent/zoom-in-24-regular';
  import IconCrop from '~icons/fluent/crop-24-regular';
  import IconCardUi from '~icons/fluent/card-ui-24-regular';
  import IconArrowRouting from '~icons/fluent/arrow-routing-24-regular';
  import IconLink from '~icons/fluent/link-24-regular';
  import IconShieldCheckmark from '~icons/fluent/shield-checkmark-24-regular';
  import IconHardDrive from '~icons/fluent/hard-drive-24-regular';
  import IconFolderPerson from '~icons/fluent/folder-person-24-regular';
  import IconRename from '~icons/fluent/rename-24-regular';
  import IconDocumentText from '~icons/fluent/document-text-24-regular';
  import IconCode from '~icons/fluent/code-24-regular';
  import IconRocket from '~icons/fluent/rocket-24-regular';
  import IconArrowSplit from '~icons/fluent/arrow-split-24-regular';
  import IconTasksApp from '~icons/fluent/tasks-app-24-regular';
  import IconGauge from '~icons/fluent/gauge-24-regular';
  import IconEraserMedium from '~icons/fluent/eraser-medium-24-regular';
  import IconDocumentBulletList from '~icons/fluent/document-bullet-list-24-regular';
  import IconInfo from '~icons/fluent/info-24-regular';
  import IconBranchFork from '~icons/fluent/branch-fork-24-regular';
  import IconImageMultiple from '~icons/fluent/image-multiple-24-regular';
  import pawstashLogo from '$lib/assets/pawstash.png';
  import { logoFlightState } from '$lib/state/logoFlightState.svelte';
  import type { AccentColor } from '$lib/theme/tokens';
  import { ripple } from '$lib/motion';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';
  import Button from '$lib/components/ui/Button.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import PaletteCircle from '$lib/components/ui/PaletteCircle.svelte';
  import { PRESET_QUADRANTS, generateAccentPalette } from '$lib/theme/palette';
  import SyncSettings from './SyncSettings.svelte';
  import ProviderSettings from './ProviderSettings.svelte';
  import { updateState } from '$lib/state/updateState.svelte';
  import { syncState } from '$lib/state/syncState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import IconUser from '~icons/fluent/person-24-regular';
  import IconChevronRight from '~icons/fluent/chevron-right-24-regular';
  import IconCloudSync from '~icons/fluent/cloud-sync-24-regular';
  import IconArrowReset from '~icons/fluent/arrow-reset-24-regular';
  import IconMoreHorizontal from '~icons/fluent/more-horizontal-24-regular';
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
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
  import { readRecentLogs, openLogsFolder, clearLogs, logger } from '$lib/utils/logger';
  import { providerState } from '$lib/state/providerState.svelte';

  let settings = $state({ ...configState.settings });
  let defaultSettings = $state<AppSettings>({ ...configState.settings });
  let resetPending = $state(false);
  let settingsMenuOpen = $state(false);
  let stickySettingsMenuOpen = $state(false);
  let activeCategory = $state('appearance');
  let viewportEl = $state<HTMLElement | null>(null);
  let isProgrammaticScroll = false;
  let programmaticScrollTimeout: ReturnType<typeof setTimeout> | undefined;
  let scrollSpyRaf: number | undefined;
  let availableBackgroundTypes = $state<BackgroundType[]>(supportedBackgroundTypes());
  let cacheStats = $state<CacheStats | null>(null);
  let cacheBusy = $state<'images' | 'all' | null>(null);
  let copyingLogs = $state(false);
  let clearingLogs = $state(false);
  let heroLogoEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (heroLogoEl) {
      logoFlightState.registerHero(heroLogoEl);
      requestAnimationFrame(() => {
        logoFlightState.flyToHero();
      });
    }
    return () => {
      logoFlightState.unregisterHero();
    };
  });

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

  const categories = $derived([
    { id: 'appearance', label: i18n.t('settings.appearance_section') },
    { id: 'background', label: i18n.t('settings.background_section') },
    { id: 'grid', label: i18n.t('settings.grid_section') },
    { id: 'providers', label: i18n.t('settings.providers_section') },
    ...providerState.providers.map((p) => ({
      id: `provider-${p.id}`,
      label: formatProviderName(p.name || p.id)
    })),
    { id: 'proxy', label: i18n.t('settings.proxy_section') },
    { id: 'downloads', label: i18n.t('settings.download_section') },
    { id: 'notifications', label: i18n.t('settings.notifications_section') },
    { id: 'cache', label: i18n.t('settings.cache_section') },
    { id: 'sync', label: i18n.t('sync.title') },
    { id: 'updates', label: i18n.t('settings.updates_section') }
  ]);

  onMount(() => {
    availableBackgroundTypes = supportedBackgroundTypes();

    void apiGetDefaultSettings()
      .then((defaults) => {
        defaultSettings = defaults;
      })
      .catch((err) => {
        logger.warn('Failed to fetch default settings:', err);
      });

    void apiGetSettings()
      .then((loaded) => {
        const hasDiff = Object.keys(loaded).some(
          (k) => (loaded as any)[k] !== (settings as any)[k]
        );
        if (hasDiff) {
          configState.updateSettings(loaded);
          Object.assign(settings, loaded);
        }
      })
      .catch((err) => {
        logger.warn('Failed to refresh settings from disk:', err);
      });
  });

  $effect(() => {
    if (activeCategory === 'cache' && !cacheStats && !cacheBusy) {
      void loadCacheStats();
    }
  });

  function handleScrollSpy() {
    if (isProgrammaticScroll || !viewportEl) return;

    if (scrollSpyRaf !== undefined) cancelAnimationFrame(scrollSpyRaf);
    scrollSpyRaf = requestAnimationFrame(() => {
      scrollSpyRaf = undefined;
      if (isProgrammaticScroll || !viewportEl) return;

      const scrollTop = viewportEl.scrollTop;
      const scrollHeight = viewportEl.scrollHeight;
      const clientHeight = viewportEl.clientHeight;

      if (scrollTop + clientHeight >= scrollHeight - 36) {
        const lastCat = categories[categories.length - 1];
        if (lastCat && activeCategory !== lastCat.id) {
          activeCategory = lastCat.id;
        }
        return;
      }

      const probeOffset = scrollTop + (layoutState.isMobile ? 80 : 110);
      let currentId = categories[0]?.id || 'appearance';
      for (const cat of categories) {
        const section = document.getElementById(`settings-${cat.id}`);
        if (section && section.offsetTop <= probeOffset) {
          currentId = cat.id;
        }
      }

      if (activeCategory !== currentId) {
        activeCategory = currentId;
      }
    });
  }

  onDestroy(() => {
    if (scrollSpyRaf !== undefined) cancelAnimationFrame(scrollSpyRaf);
    if (programmaticScrollTimeout) clearTimeout(programmaticScrollTimeout);
  });

  async function selectDownloadDir() {
    try {
      if (layoutState.isMobileDevice) {
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
    try {
      let selected: string | null | undefined;
      if (layoutState.isMobileDevice) {
        selected = await invoke<string | null>('pick_file', { kind });
      } else {
        selected = await open({
          multiple: false,
          title: i18n.t(kind === 'image' ? 'settings.choose_background_image' : 'settings.choose_background_video'),
          filters: [{
            name: kind === 'image' ? 'Images' : 'Videos',
            extensions: kind === 'image'
              ? ['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif']
              : ['mp4', 'webm', 'm4v', 'mov']
          }]
        });
      }
      if (selected && typeof selected === 'string') {
        const validatedPath = await invoke<string>('store_custom_background', { sourcePath: selected, kind });
        if (kind === 'image') backgroundState.setImageUrl(validatedPath);
        else backgroundState.setVideoUrl(validatedPath);
        notify.success(
          i18n.t(kind === 'image' ? 'settings.background_saved' : 'settings.background_video_saved'),
          validatedPath.split(/[/\\]/).pop() || validatedPath
        );
      }
    } catch (error) {
      notify.error(i18n.t('settings.background_file_failed'), error);
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

  const presetAccents: { id: AccentColor; labelKey: string; quadrants: [string, string, string, string] }[] = [
    { id: '#D69085', labelKey: '#D69085', quadrants: PRESET_QUADRANTS['#D69085'] },
    { id: 'rose', labelKey: 'rose', quadrants: PRESET_QUADRANTS.rose },
    { id: 'violet', labelKey: 'violet', quadrants: PRESET_QUADRANTS.violet },
    { id: 'cyan', labelKey: 'cyan', quadrants: PRESET_QUADRANTS.cyan },
    { id: 'emerald', labelKey: 'emerald', quadrants: PRESET_QUADRANTS.emerald },
    { id: 'amber', labelKey: 'amber', quadrants: PRESET_QUADRANTS.amber },
    { id: 'indigo', labelKey: 'indigo', quadrants: PRESET_QUADRANTS.indigo }
  ];

  const isCustomActive = $derived(
    themeState.tokens.accent !== 'system' &&
    !presetAccents.some((p) => p.id.toLowerCase() === themeState.tokens.accent.toLowerCase())
  );

  const customQuadrants = $derived.by<[string, string, string, string]>(() => {
    if (isCustomActive && themeState.tokens.accent.startsWith('#')) {
      return generateAccentPalette(themeState.tokens.accent).quadrants;
    }
    return ['#d69085', '#f59e0b', '#10b981', '#6366f1'];
  });

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

  function resetSetting<K extends keyof AppSettings>(key: K) {
    const defVal = defaultSettings[key];
    (settings as any)[key] = defVal;
    void updateAndSaveSetting(key, defVal);
  }

  function resetPanicKey() {
    const defKey = defaultSettings.panic_button_shortcut || 'H';
    const defEnabled = defaultSettings.panic_button_enabled ?? true;
    resetSetting('panic_button_shortcut');
    void apiUpdatePanicKey(defKey, defEnabled);
  }

  function resetPanicEnabled() {
    const defEnabled = defaultSettings.panic_button_enabled ?? true;
    resetSetting('panic_button_enabled');
    void apiUpdatePanicKey(settings.panic_button_shortcut || 'H', defEnabled);
  }

  function openCategory(id: string) {
    activeCategory = id;
    if (id === 'cache' && !cacheStats && !cacheBusy) {
      void loadCacheStats();
    }

    const targetEl = document.getElementById(`settings-${id}`);
    if (!targetEl || !viewportEl) {
      targetEl?.scrollIntoView({
        behavior: 'smooth',
        block: 'start'
      });
      return;
    }

    isProgrammaticScroll = true;
    if (programmaticScrollTimeout) clearTimeout(programmaticScrollTimeout);

    const isMobile = layoutState.isMobile;
    const headerOffset = isMobile ? 64 : 88;
    const targetTop = Math.max(0, targetEl.offsetTop - headerOffset);
    const maxScrollTop = viewportEl.scrollHeight - viewportEl.clientHeight;
    const finalTop = Math.min(targetTop, maxScrollTop);

    viewportEl.scrollTo({
      top: finalTop,
      behavior: 'smooth'
    });

    programmaticScrollTimeout = setTimeout(() => {
      isProgrammaticScroll = false;
    }, 600);
  }

  let showResetConfirm = $state(false);
  let showWipeConfirm = $state(false);
  let showClearAllCacheConfirm = $state(false);
  let showClearBgMediaConfirm = $state(false);
  let sectionToReset = $state<string | null>(null);

  async function handleCopyLogs() {
    if (copyingLogs) return;
    copyingLogs = true;
    try {
      const rawLogs = await readRecentLogs(500);
      await navigator.clipboard.writeText(rawLogs.trim());
      notify.success(i18n.t('settings.logs_copied'));
    } catch (e) {
      notify.error(i18n.t('settings.logs_copy_failed'));
      logger.error('Failed to copy debug logs', e);
    } finally {
      copyingLogs = false;
    }
  }

  async function handleOpenLogsFolder() {
    try {
      await openLogsFolder();
    } catch (e) {
      notify.error('Failed to open logs folder');
      logger.error('Failed to open logs folder', e);
    }
  }

  async function handleClearLogs() {
    if (clearingLogs) return;
    clearingLogs = true;
    try {
      await clearLogs();
      notify.success(i18n.t('settings.logs_cleared'));
    } catch (e) {
      notify.error('Failed to clear logs');
      logger.error('Failed to clear logs', e);
    } finally {
      clearingLogs = false;
    }
  }

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
          next.card_view_mode = defaults.card_view_mode;
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

        case 'notifications':
          next.notifications_enabled = defaults.notifications_enabled;
          next.notifications_download_completed = defaults.notifications_download_completed;
          next.notifications_download_progress = defaults.notifications_download_progress;
          next.notifications_show_preview = defaults.notifications_show_preview;
          next.notifications_sound = defaults.notifications_sound;
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
  <SettingsNav
    {categories}
    {activeCategory}
    onselect={openCategory}
  />
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
        use:ripple
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

      <button
        type="button"
        class="settings-menu-option text-red-400 hover:text-red-300 hover:bg-red-500/10"
        use:ripple
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
        use:ripple
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

      <button
        type="button"
        class="settings-menu-option text-red-400 hover:text-red-300 hover:bg-red-500/10"
        use:ripple
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
    <div class="flex items-center justify-center gap-2 text-[16px] font-bold text-white/95">
      <span>{i18n.t('settings.made_with')}</span>
      <IconHeart class="w-4 h-4 text-[var(--color-danger,#f43f5e)] fill-current shrink-0" />
      <span>{i18n.t('settings.by_nichind')}</span>
    </div>
    <span class="text-[13px] text-white/50 mt-1 mb-3 select-none">
      {i18n.t('settings.check_out_my_pages')}
    </span>

    <div class="grid grid-cols-2 gap-2.5 w-full">
      <Button
        variant="ghost"
        class="w-full justify-center gap-2 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
        onclick={() => openExternalUrl('https://nichind.dev')}
      >
        <svg viewBox="0 0 106 78" fill="currentColor" class="w-4 h-3.5 opacity-70 shrink-0">
          <path d="M106 78H71.7471L30.3184 30.6006V78H0V0H41.4277L106 78ZM106 24.375H87.873L67.7383 0H106V24.375Z" />
        </svg>
        <span class="truncate">nichind.dev</span>
      </Button>

      <Button
        variant="ghost"
        class="w-full justify-center gap-2 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
        onclick={() => openExternalUrl('https://github.com/nichind')}
      >
        <IconGithub class="w-4 h-4 opacity-70 shrink-0" />
        <span class="truncate">GitHub</span>
      </Button>
    </div>
  </div>
{/snippet}

<PageShell
  scrollable={true}
  scrollKey={navigationState.entryKey}
  bind:viewport={viewportEl}
  onscroll={handleScrollSpy}
>
  {#snippet overlay()}
    <StickyHeader threshold={120}>
      <div class="sticky-settings-toolbar">
        {@render categoryTabs()}
        {@render settingsMenu('sticky')}
      </div>
    </StickyHeader>
  {/snippet}

  <div class="settings-page" data-control-size="base">
    <div class="settings-toolbar">
      {@render categoryTabs()}
      {@render settingsMenu('main')}
    </div>

    <div class="settings-hero-brand">
      <img
        bind:this={heroLogoEl}
        src={pawstashLogo}
        alt="Pawstash"
        class="settings-hero-logo"
        class:is-flying={logoFlightState.isFlying}
      />
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
          </div>

          <div class="mobile-hero-footer">
            <span>{i18n.t('profile.title')}</span>
            <IconChevronRight class="w-3.5 h-3.5 text-[var(--accent)]" />
          </div>
        </button>

        <div class="flex items-center justify-center gap-3 w-full mt-0.5">
          <div class="h-[1px] flex-1 bg-white/[0.06]"></div>
          <span class="text-[11px] font-semibold uppercase tracking-wider text-white/35 select-none">
            {i18n.t('settings.community')}
          </span>
          <div class="h-[1px] flex-1 bg-white/[0.06]"></div>
        </div>

        <div class="grid grid-cols-2 gap-2.5 w-full">
          <Button
            variant="ghost"
            class="w-full justify-center gap-2 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
            onclick={() => openExternalUrl('https://t.me/pawstashapp')}
          >
            <IconTelegram class="w-4 h-4 opacity-70 shrink-0" />
            <span class="truncate">Telegram</span>
          </Button>

          <Button
            variant="ghost"
            class="w-full justify-center gap-2 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
            onclick={() => openExternalUrl('https://discord.gg/ahcx8ub5Ck')}
          >
            <IconDiscord class="w-4 h-4 opacity-70 shrink-0" />
            <span class="truncate">Discord</span>
          </Button>

          <Button
            variant="ghost"
            class="w-full justify-center gap-2 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
            onclick={() => openExternalUrl('https://reddit.com/r/pawstash')}
          >
            <IconReddit class="w-4 h-4 opacity-70 shrink-0" />
            <span class="truncate">r/pawstash</span>
          </Button>

          <Button
            variant="ghost"
            class="w-full justify-center gap-2 border border-white/8 hover:border-white/16 bg-white/[0.03] hover:bg-white/[0.07]"
            onclick={() => openExternalUrl('https://github.com/pawstash')}
          >
            <IconGithub class="w-4 h-4 opacity-70 shrink-0" />
            <span class="truncate">{i18n.t('settings.contribute')}</span>
          </Button>
        </div>

        {@render authorBuildBar()}
      </div>
    {/if}

    <div id="settings-appearance" class="settings-section">
      <SectionTitle icon={IconPaintBrush} title={i18n.t('settings.appearance_section')} onreset={() => (sectionToReset = 'appearance')} />

      <div class="settings-list">
        <SettingItem
          title={i18n.t('settings.language')}
          icon={IconTranslate}
          value={i18n.currentLocale}
          defaultValue="en"
          onReset={() => i18n.setLocale('en')}
        >
          <ChoiceGroup
            options={[
              { value: 'en', label: 'English', icon: IconFlagUs },
              { value: 'ru', label: 'Русский', icon: IconFlagRu }
            ]}
            value={i18n.currentLocale}
            onchange={(val) => i18n.setLocale(val as any)}
          />
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.font_family')}
          description={i18n.t('settings.font_family_desc')}
          icon={IconTextFont}
          value={themeState.tokens.fontFamily || ''}
          defaultValue=""
          onReset={() => themeState.setFontFamily('')}
        >
          <div class="w-full">
            <Input
              value={themeState.tokens.fontFamily || ''}
              placeholder={i18n.t('settings.font_family_placeholder')}
              clearable={true}
              oninput={(e) => {
                const target = e.target as HTMLInputElement | null;
                themeState.setFontFamily(target ? target.value : '');
              }}
              onchange={(e) => {
                const target = e.target as HTMLInputElement | null;
                themeState.setFontFamily(target ? target.value : '');
              }}
            />
          </div>
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.accent_color')}
          description={i18n.t('settings.accent_color_desc')}
          icon={IconPaint}
          value={themeState.tokens.accent}
          defaultValue={themeState.systemPalette ? 'system' : '#D69085'}
          onReset={() => themeState.setAccent(themeState.systemPalette ? 'system' : '#D69085')}
        >
          <div class="settings-accent-controls flex items-center gap-2 flex-wrap">
            {#if themeState.systemPalette}
              <PaletteCircle
                quadrants={themeState.systemPalette.quadrants}
                active={themeState.tokens.accent === 'system'}
                label={i18n.t('settings.system_accent')}
                onclick={() => themeState.setAccent('system')}
              />
            {/if}

            {#each presetAccents as c}
              <PaletteCircle
                quadrants={c.quadrants}
                active={themeState.tokens.accent.toLowerCase() === c.id.toLowerCase()}
                label={i18n.t('settings.set_accent', { color: c.labelKey })}
                onclick={() => themeState.setAccent(c.id)}
              />
            {/each}

            <div class="w-[1px] h-5 bg-white/10 mx-0.5"></div>

            <div class="relative w-[26px] h-[26px] flex items-center justify-center">
              <PaletteCircle
                quadrants={customQuadrants}
                active={isCustomActive}
                label={i18n.t('settings.custom_accent')}
              />
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
          icon={IconEyedropper}
          align="right"
          value={settings.dynamic_accent}
          defaultValue={defaultSettings.dynamic_accent}
          onReset={() => resetSetting('dynamic_accent')}
        >
          <ChoiceGroup
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
          icon={IconDockRow}
          align="right"
          value={settings.sticky_header}
          defaultValue={defaultSettings.sticky_header}
          onReset={() => resetSetting('sticky_header')}
        >
          <ChoiceGroup
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
          icon={IconBlur}
          align="right"
          value={settings.scroll_edge_mask ?? true}
          defaultValue={defaultSettings.scroll_edge_mask ?? true}
          onReset={() => resetSetting('scroll_edge_mask')}
        >
          <ChoiceGroup
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.scroll_edge_mask ?? true}
            onchange={(value) => updateAndSaveSetting('scroll_edge_mask', value)}
          />
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.disable_blur_placeholders')}
          description={i18n.t('settings.disable_blur_placeholders_desc')}
          icon={IconImageOff}
          align="right"
          value={settings.disable_blur_placeholders ?? false}
          defaultValue={defaultSettings.disable_blur_placeholders ?? false}
          onReset={() => resetSetting('disable_blur_placeholders')}
        >
          <ChoiceGroup
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.disable_blur_placeholders ?? false}
            onchange={(value) => updateAndSaveSetting('disable_blur_placeholders', value)}
          />
        </SettingItem>

        {#if !layoutState.isMobile}
          <SettingItem
            title={i18n.t('settings.titlebar_style')}
            description={i18n.t('settings.titlebar_style_desc')}
            icon={IconWindowApps}
            align="right"
            value={settings.titlebar_style || 'auto'}
            defaultValue={defaultSettings.titlebar_style || 'auto'}
            onReset={() => resetSetting('titlebar_style')}
          >
            <ChoiceGroup
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
          icon={IconAlert}
          value={settings.toast_position || 'auto'}
          defaultValue={defaultSettings.toast_position || 'auto'}
          onReset={() => resetSetting('toast_position')}
        >
          <div class="w-full">
            <Select
              options={toastPositionOptions}
              value={settings.toast_position || 'auto'}
              onchange={(val) => updateAndSaveSetting('toast_position', val)}
            />
          </div>
        </SettingItem>

        {#if !layoutState.isMobile}
          <SettingItem
            title={i18n.t('settings.panic_button')}
            description={i18n.t('settings.panic_button_desc')}
            icon={IconWarning}
            align="right"
            value={settings.panic_button_enabled ?? settings.boss_key_enabled ?? true}
            defaultValue={defaultSettings.panic_button_enabled ?? true}
            onReset={resetPanicEnabled}
          >
            <ChoiceGroup
              options={[
                { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
                { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
              ]}
              value={settings.panic_button_enabled ?? settings.boss_key_enabled ?? true}
              onchange={(val) => {
                updateAndSaveSetting('panic_button_enabled', val);
                void apiUpdatePanicKey(settings.panic_button_shortcut || settings.boss_key_shortcut || 'H', val);
              }}
            />
          </SettingItem>

          {#if (settings.panic_button_enabled ?? settings.boss_key_enabled ?? true)}
            <SettingItem
              title={i18n.t('settings.panic_button_shortcut')}
              description={i18n.t('settings.panic_button_shortcut_desc')}
              icon={IconKeyboard}
              value={settings.panic_button_shortcut || 'H'}
              defaultValue={defaultSettings.panic_button_shortcut || 'H'}
              onReset={resetPanicKey}
            >
              <div class="w-full">
                <ShortcutInput
                  bind:value={settings.panic_button_shortcut}
                  defaultValue="H"
                  onchange={(val) => {
                    updateAndSaveSetting('panic_button_shortcut', val || 'H');
                    void apiUpdatePanicKey(val || 'H', settings.panic_button_enabled ?? settings.boss_key_enabled ?? true);
                  }}
                />
              </div>
            </SettingItem>
          {/if}
        {/if}
      </div>
    </div>

    <div id="settings-background" class="settings-section">
      <SectionTitle icon={IconWallpaper} title={i18n.t('settings.background_section')} onreset={() => (sectionToReset = 'background')} />

      <div class="settings-list">
        <SettingItem
          title={i18n.t('settings.bg_type')}
          description={i18n.t('settings.bg_type_desc')}
          icon={IconWallpaper}
          value={backgroundState.settings.type}
          defaultValue={defaultBackgroundType()}
          onReset={() => backgroundState.setType(defaultBackgroundType())}
        >
          <Select
            options={bgTypes.map((t) => ({ value: t.id, label: t.label }))}
            value={backgroundState.settings.type}
            onchange={(val) => backgroundState.setType(val as BackgroundType)}
          />
        </SettingItem>

        {#if backgroundState.settings.type === 'custom'}
          <SettingItem
            title={i18n.t('settings.background_source')}
            description={i18n.t('settings.background_source_desc')}
            icon={IconImageMultiple}
            value={backgroundState.settings.customKind}
            defaultValue="color"
            onReset={() => backgroundState.setCustomKind('color')}
          >
            <Select
              options={[
                { value: 'color', label: i18n.t('settings.background_source_color') },
                { value: 'palette', label: i18n.t('settings.background_source_palette') },
                { value: 'image', label: i18n.t('settings.background_source_image') },
                { value: 'video', label: i18n.t('settings.background_source_video') }
              ]}
              value={backgroundState.settings.customKind}
              onchange={(value) => backgroundState.setCustomKind(value as CustomBackgroundKind)}
            />
          </SettingItem>

          {#if backgroundState.settings.customKind === 'color'}
            <SettingItem
              title={i18n.t('settings.background_primary')}
              description={i18n.t('settings.background_primary_desc')}
              icon={IconColorFill}
              value={backgroundState.settings.solidColor}
              defaultValue="#000000"
              onReset={() => backgroundState.setSolidColor('#000000')}
            >
              <input class="background-color-input" type="color" value={backgroundState.settings.solidColor} oninput={(event) => backgroundState.setSolidColor(event.currentTarget.value)} />
            </SettingItem>

            <SettingItem
              title={i18n.t('settings.background_secondary')}
              description={i18n.t('settings.background_secondary_desc')}
              icon={IconCircleHalfFill}
              value={backgroundState.settings.gradientSecondary}
              defaultValue="#111827"
              onReset={() => backgroundState.setGradientSecondary('#111827')}
            >
              <input class="background-color-input" type="color" value={backgroundState.settings.gradientSecondary} oninput={(event) => backgroundState.setGradientSecondary(event.currentTarget.value)} />
            </SettingItem>
          {:else if backgroundState.settings.customKind === 'image'}
            <SettingItem title={i18n.t('settings.background_image')} description={i18n.t('settings.background_image_desc')} icon={IconImage}>
              <Input placeholder={i18n.t('settings.background_image_placeholder')} value={backgroundState.settings.imageUrl} readonly={true} onBrowse={() => void selectCustomBackground('image')} />
            </SettingItem>
          {:else if backgroundState.settings.customKind === 'video'}
            <SettingItem title={i18n.t('settings.background_video')} description={i18n.t('settings.background_video_desc')} icon={IconVideo}>
              <Input placeholder={i18n.t('settings.background_video_placeholder')} value={backgroundState.settings.videoUrl} readonly={true} onBrowse={() => void selectCustomBackground('video')} />
            </SettingItem>
          {/if}

          {#if backgroundState.settings.customKind !== 'color'}
            <SettingItem
              title={i18n.t('settings.background_blur')}
              description={i18n.t('settings.background_blur_desc')}
              icon={IconBlur}
              value={backgroundState.settings.blurPx}
              defaultValue={24}
              onReset={() => backgroundState.setBlur(24)}
            >
              <div class="flex items-center gap-4 w-full">
                <Slider min={0} max={40} value={backgroundState.settings.blurPx} oninput={(value) => backgroundState.setBlur(value)} />
                <span class="text-[13px] font-mono text-gray-300 w-10 text-right shrink-0">{backgroundState.settings.blurPx}px</span>
              </div>
            </SettingItem>

            <SettingItem
              title={i18n.t('settings.background_opacity')}
              description={i18n.t('settings.background_opacity_desc')}
              icon={IconCircleHalfFill}
              value={backgroundState.settings.opacity}
              defaultValue={0.85}
              onReset={() => backgroundState.setOpacity(0.85)}
            >
              <div class="flex items-center gap-4 w-full">
                <Slider min={0.1} max={1} step={0.05} value={backgroundState.settings.opacity} oninput={(value) => backgroundState.setOpacity(value)} />
                <span class="text-[13px] font-mono text-gray-300 w-12 text-right shrink-0">{Math.round(backgroundState.settings.opacity * 100)}%</span>
              </div>
            </SettingItem>

            <SettingItem
              title={i18n.t('settings.background_brightness')}
              description={i18n.t('settings.background_brightness_desc')}
              icon={IconBrightnessHigh}
              value={backgroundState.settings.brightness}
              defaultValue={0.5}
              onReset={() => backgroundState.setBrightness(0.5)}
            >
              <div class="flex items-center gap-4 w-full">
                <Slider min={0.2} max={1.5} step={0.05} value={backgroundState.settings.brightness} oninput={(value) => backgroundState.setBrightness(value)} />
                <span class="text-[13px] font-mono text-gray-300 w-12 text-right shrink-0">{Math.round(backgroundState.settings.brightness * 100)}%</span>
              </div>
            </SettingItem>

            <SettingItem
              title={i18n.t('settings.background_saturation')}
              description={i18n.t('settings.background_saturation_desc')}
              icon={IconPaint}
              value={backgroundState.settings.saturation}
              defaultValue={1.2}
              onReset={() => backgroundState.setSaturation(1.2)}
            >
              <div class="flex items-center gap-4 w-full">
                <Slider min={0} max={2} step={0.05} value={backgroundState.settings.saturation} oninput={(value) => backgroundState.setSaturation(value)} />
                <span class="text-[13px] font-mono text-gray-300 w-12 text-right shrink-0">{Math.round(backgroundState.settings.saturation * 100)}%</span>
              </div>
            </SettingItem>

            {#if (backgroundState.settings.customKind === 'image' && backgroundState.settings.imageUrl) || (backgroundState.settings.customKind === 'video' && backgroundState.settings.videoUrl)}
              <SettingItem title={i18n.t('settings.background_media')} description={i18n.t('settings.background_media_desc')} icon={IconDelete}>
                <Button variant="ghost" onclick={() => (showClearBgMediaConfirm = true)}>
                  <IconDelete class="w-4 h-4 mr-1.5" />
                  {i18n.t('settings.background_media_clear')}
                </Button>
              </SettingItem>
            {/if}
          {/if}
        {/if}
      </div>
    </div>

    <div id="settings-grid" class="settings-section">
      <SectionTitle icon={IconGrid} title={i18n.t('settings.grid_section')} onreset={() => (sectionToReset = 'grid')} />

      <div class="settings-list">
        <SettingItem
          title={i18n.t('settings.grid_scale')}
          description={i18n.t('settings.grid_scale_desc')}
          icon={IconZoomIn}
          value={settings.grid_scale}
          defaultValue={defaultSettings.grid_scale}
          onReset={() => resetSetting('grid_scale')}
        >
          <div class="flex items-center gap-4 w-full">
            <Slider
              min={60}
              max={160}
              value={settings.grid_scale}
              oninput={(value) => updateAndSaveSetting('grid_scale', Math.round(value / 5) * 5)}
            />
            <span class="text-[13px] font-mono text-gray-300 w-12 text-right shrink-0">{settings.grid_scale}%</span>
          </div>
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.grid_ratio')}
          description={i18n.t('settings.grid_ratio_desc')}
          icon={IconCrop}
          align="right"
          value={settings.grid_aspect_ratio}
          defaultValue={defaultSettings.grid_aspect_ratio}
          onReset={() => resetSetting('grid_aspect_ratio')}
        >
          <ChoiceGroup
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

        <SettingItem
          title={i18n.t('settings.card_view_mode')}
          description={i18n.t('settings.card_view_mode_desc')}
          icon={IconCardUi}
          value={settings.card_view_mode || 'detailed'}
          defaultValue={defaultSettings.card_view_mode || 'detailed'}
          onReset={() => resetSetting('card_view_mode')}
        >
          <ChoiceGroup
            options={[
              { value: 'detailed', label: i18n.t('settings.card_view_mode_detailed') },
              { value: 'lite', label: i18n.t('settings.card_view_mode_lite') }
            ]}
            value={settings.card_view_mode || 'detailed'}
            onchange={(value) => updateAndSaveSetting('card_view_mode', value)}
          />
        </SettingItem>
      </div>
    </div>

    <ProviderSettings />

    <div id="settings-proxy" class="settings-section">
      <SectionTitle icon={IconGlobe} title={i18n.t('settings.proxy_section')} onreset={() => (sectionToReset = 'proxy')} />

      <div class="settings-list">
        <SettingItem
          title={i18n.t('settings.proxy_mode')}
          description={i18n.t('settings.proxy_mode_desc')}
          icon={IconArrowRouting}
          align="right"
          value={settings.proxy_mode}
          defaultValue={defaultSettings.proxy_mode}
          onReset={() => resetSetting('proxy_mode')}
        >
          <ChoiceGroup
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
          <SettingItem
            title={i18n.t('settings.proxy_url')}
            description={i18n.t('settings.proxy_url_desc')}
            icon={IconLink}
            value={settings.proxy_url}
            defaultValue={defaultSettings.proxy_url}
            onReset={() => resetSetting('proxy_url')}
          >
            <div class="w-full">
              <Input
                clearable={true}
                placeholder="http://127.0.0.1:8080"
                bind:value={settings.proxy_url}
                onblur={() => updateAndSaveSetting('proxy_url', settings.proxy_url)}
              />
            </div>
          </SettingItem>

          <SettingItem
            title={i18n.t('settings.proxy_username')}
            description={i18n.t('settings.proxy_username_desc')}
            icon={IconUser}
            value={settings.proxy_username}
            defaultValue={defaultSettings.proxy_username}
            onReset={() => resetSetting('proxy_username')}
          >
            <div class="w-full">
              <Input
                clearable={true}
                placeholder={i18n.t('settings.optional')}
                bind:value={settings.proxy_username}
                onblur={() => updateAndSaveSetting('proxy_username', settings.proxy_username)}
              />
            </div>
          </SettingItem>

          <SettingItem
            title={i18n.t('settings.proxy_password')}
            description={i18n.t('settings.proxy_password_desc')}
            icon={IconKey}
            value={settings.proxy_password}
            defaultValue={defaultSettings.proxy_password}
            onReset={() => resetSetting('proxy_password')}
          >
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
            icon={IconShieldCheckmark}
            align="right"
            value={settings.proxy_bypass_local}
            defaultValue={defaultSettings.proxy_bypass_local}
            onReset={() => resetSetting('proxy_bypass_local')}
          >
            <ChoiceGroup
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

    <div id="settings-downloads" class="settings-section">
      <SectionTitle icon={IconDownload} title={i18n.t('settings.download_section')} onreset={() => (sectionToReset = 'downloads')} />

      <div class="settings-list">
        <SettingItem
          title={i18n.t('settings.downloads_storage')}
          description={i18n.t('settings.downloads_storage_desc')}
          icon={IconHardDrive}
        >
          <DownloadsStatsBar downloads={downloadState.downloads} />
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.download_dir')}
          description={i18n.t('settings.download_dir_desc')}
          icon={IconFolder}
          value={settings.download_dir}
          defaultValue={defaultSettings.download_dir}
          onReset={() => resetSetting('download_dir')}
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
          icon={IconFolderPerson}
          align="right"
          value={settings.download_group_by_creator}
          defaultValue={defaultSettings.download_group_by_creator}
          onReset={() => resetSetting('download_group_by_creator')}
        >
          <ChoiceGroup
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
            icon={IconRename}
            value={settings.download_creator_folder_template}
            defaultValue={defaultSettings.download_creator_folder_template}
            onReset={() => resetSetting('download_creator_folder_template')}
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
          value={settings.download_group_by_post}
          defaultValue={defaultSettings.download_group_by_post}
          onReset={() => resetSetting('download_group_by_post')}
        >
          <ChoiceGroup
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
            icon={IconRename}
            value={settings.download_post_folder_template}
            defaultValue={defaultSettings.download_post_folder_template}
            onReset={() => resetSetting('download_post_folder_template')}
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
          icon={IconRename}
          value={settings.download_filename_template}
          defaultValue={defaultSettings.download_filename_template}
          onReset={() => resetSetting('download_filename_template')}
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
          title={i18n.t('settings.download_save_metadata')}
          description={i18n.t('settings.download_save_metadata_desc')}
          icon={IconDocumentText}
          align="right"
          value={settings.download_save_metadata ?? false}
          defaultValue={defaultSettings.download_save_metadata ?? false}
          onReset={() => resetSetting('download_save_metadata')}
        >
          <ChoiceGroup
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.download_save_metadata ?? false}
            onchange={(val) => updateAndSaveSetting('download_save_metadata', val)}
          />
        </SettingItem>

        {#if settings.download_save_metadata}
          <SettingItem
            title={i18n.t('settings.download_metadata_format')}
            description={i18n.t('settings.download_metadata_format_desc')}
            icon={IconCode}
            align="right"
            value={settings.download_metadata_format || 'txt'}
            defaultValue={defaultSettings.download_metadata_format || 'txt'}
            onReset={() => resetSetting('download_metadata_format')}
          >
            <ChoiceGroup
              options={[
                { value: 'txt', label: '.txt' },
                { value: 'json', label: '.json' },
                { value: 'both', label: '.txt + .json' }
              ]}
              value={settings.download_metadata_format || 'txt'}
              onchange={(val) => updateAndSaveSetting('download_metadata_format', val)}
            />
          </SettingItem>
        {/if}

        <SettingItem
          title={i18n.t('settings.aria2c_engine')}
          description={i18n.t('settings.aria2c_engine_desc')}
          icon={IconRocket}
          align="right"
          value={settings.use_aria2c}
          defaultValue={defaultSettings.use_aria2c}
          onReset={() => resetSetting('use_aria2c')}
        >
          <ChoiceGroup
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
            icon={IconArrowSplit}
            value={settings.aria2_connections}
            defaultValue={defaultSettings.aria2_connections}
            onReset={() => resetSetting('aria2_connections')}
          >
            <NumberStepper
              min={1}
              max={32}
              value={settings.aria2_connections}
              ariaLabel={i18n.t('settings.parallel_connections')}
              onchange={(value) => updateAndSaveSetting('aria2_connections', value)}
            />
          </SettingItem>
        {/if}

        <SettingItem
          title={i18n.t('settings.download_max_concurrent')}
          description={i18n.t('settings.download_max_concurrent_desc')}
          icon={IconTasksApp}
          value={settings.download_max_concurrent ?? 3}
          defaultValue={defaultSettings.download_max_concurrent ?? 3}
          onReset={() => resetSetting('download_max_concurrent')}
        >
          <NumberStepper
            min={1}
            max={10}
            value={settings.download_max_concurrent ?? 3}
            ariaLabel={i18n.t('settings.download_max_concurrent')}
            onchange={(value) => updateAndSaveSetting('download_max_concurrent', value)}
          />
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.template_preview')}
          description={i18n.t('settings.download_preview_desc') || 'Resolved destination path for saved files'}
          icon={IconEye}
          class="col-span-full"
        >
          <div class="w-full">
            <Input
              value={fullDownloadPathPreview}
              readonly={true}
              class="font-mono text-[13px]"
              actionIcon={copiedPreview ? IconCheck : IconCopy}
              actionTooltip={copiedPreview ? (i18n.t('common.copied') || 'Copied') : (i18n.t('common.copy') || 'Copy')}
              onAction={copyPreviewPath}
            />
          </div>
        </SettingItem>
      </div>
    </div>

    <div id="settings-notifications" class="settings-section">
      <SectionTitle icon={IconAlert} title={i18n.t('settings.notifications_section')} onreset={() => (sectionToReset = 'notifications')} />

      <div class="settings-list">
        <SettingItem
          title={i18n.t('settings.notifications_enabled')}
          description={i18n.t('settings.notifications_enabled_desc')}
          icon={IconAlert}
          align="right"
          value={settings.notifications_enabled}
          defaultValue={defaultSettings.notifications_enabled}
          onReset={() => resetSetting('notifications_enabled')}
        >
          <ChoiceGroup
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={settings.notifications_enabled}
            onchange={(val) => updateAndSaveSetting('notifications_enabled', val)}
          />
        </SettingItem>

        {#if settings.notifications_enabled}
          <SettingItem
            title={i18n.t('settings.notifications_download_completed')}
            description={i18n.t('settings.notifications_download_completed_desc')}
            icon={IconCheck}
            align="right"
            value={settings.notifications_download_completed}
            defaultValue={defaultSettings.notifications_download_completed}
            onReset={() => resetSetting('notifications_download_completed')}
          >
            <ChoiceGroup
              options={[
                { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
                { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
              ]}
              value={settings.notifications_download_completed}
              onchange={(val) => updateAndSaveSetting('notifications_download_completed', val)}
            />
          </SettingItem>

          <SettingItem
            title={i18n.t('settings.notifications_download_progress')}
            description={i18n.t('settings.notifications_download_progress_desc')}
            icon={IconDownload}
            align="right"
            value={settings.notifications_download_progress}
            defaultValue={defaultSettings.notifications_download_progress}
            onReset={() => resetSetting('notifications_download_progress')}
          >
            <ChoiceGroup
              options={[
                { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
                { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
              ]}
              value={settings.notifications_download_progress}
              onchange={(val) => updateAndSaveSetting('notifications_download_progress', val)}
            />
          </SettingItem>

          <SettingItem
            title={i18n.t('settings.notifications_show_preview')}
            description={i18n.t('settings.notifications_show_preview_desc')}
            icon={IconImageMultiple}
            align="right"
            value={settings.notifications_show_preview}
            defaultValue={defaultSettings.notifications_show_preview}
            onReset={() => resetSetting('notifications_show_preview')}
          >
            <ChoiceGroup
              options={[
                { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
                { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
              ]}
              value={settings.notifications_show_preview}
              onchange={(val) => updateAndSaveSetting('notifications_show_preview', val)}
            />
          </SettingItem>

          <SettingItem
            title={i18n.t('settings.notifications_sound')}
            description={i18n.t('settings.notifications_sound_desc')}
            icon={IconSpeaker2}
            align="right"
            value={settings.notifications_sound}
            defaultValue={defaultSettings.notifications_sound}
            onReset={() => resetSetting('notifications_sound')}
          >
            <ChoiceGroup
              options={[
                { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
                { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
              ]}
              value={settings.notifications_sound}
              onchange={(val) => updateAndSaveSetting('notifications_sound', val)}
            />
          </SettingItem>
        {/if}
      </div>
    </div>

    <div id="settings-cache" class="settings-section">
      <SectionTitle icon={IconDatabase} title={i18n.t('settings.cache_section')} onreset={() => (sectionToReset = 'cache')} />

      <div class="settings-list">
        <SettingItem title={i18n.t('settings.cache_usage')} description={i18n.t('settings.cache_usage_desc')} icon={IconHardDrive}>
          {#if cacheStats}
            <StorageBar stats={cacheStats} limitMb={settings.cache_max_mb} {formatBytes} />
          {:else}
            <div class="flex items-center gap-2 py-3 text-white/50 text-xs">
              <IconLoading />
              <span>{i18n.t('settings.cache_section')}...</span>
            </div>
          {/if}
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.cache_limit')}
          description={i18n.t('settings.cache_limit_desc')}
          icon={IconGauge}
          value={settings.cache_max_mb}
          defaultValue={defaultSettings.cache_max_mb}
          onReset={() => resetSetting('cache_max_mb')}
        >
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

        <SettingItem title={i18n.t('settings.cache_clear')} description={i18n.t('settings.cache_clear_desc')} icon={IconEraserMedium}>
          <Button
            variant="ghost"
            disabled={!!cacheBusy || !cacheStats || cacheStats.total_bytes === 0}
            onclick={() => void clearCache('images')}
          >
            {#if cacheBusy === 'images'}<IconLoading class="w-4 h-4 mr-1.5" />{:else}<IconDelete class="w-4 h-4 mr-1.5" />{/if}
            {i18n.t('settings.cache_clear_action')}
          </Button>
        </SettingItem>

        <SettingItem title={i18n.t('settings.cache_clear_all')} description={i18n.t('settings.cache_clear_all_desc')} icon={IconDelete}>
          <Button
            variant="danger"
            disabled={!!cacheBusy || !cacheStats || cacheStats.total_bytes + cacheStats.metadata_bytes === 0}
            onclick={() => (showClearAllCacheConfirm = true)}
          >
            {#if cacheBusy === 'all'}<IconLoading class="w-4 h-4 mr-1.5" />{:else}<IconDelete class="w-4 h-4 mr-1.5" />{/if}
            {i18n.t('settings.cache_clear_all_action')}
          </Button>
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.diagnostics_logs')}
          description={i18n.t('settings.diagnostics_logs_desc')}
          icon={IconDocumentBulletList}
        >
          <div class="settings-action-group">
            <Button
              variant="ghost"
              disabled={copyingLogs}
              onclick={() => void handleCopyLogs()}
            >
              {#if copyingLogs}<IconLoading class="w-4 h-4 mr-1.5" />{:else}<IconCopy class="w-4 h-4 mr-1.5" />{/if}
              {i18n.t('settings.copy_logs')}
            </Button>
            <Button
              variant="ghost"
              onclick={() => void handleOpenLogsFolder()}
            >
              <IconFolder class="w-4 h-4 mr-1.5" />
              {i18n.t('settings.open_logs_folder')}
            </Button>
            <Button
              variant="ghost"
              disabled={clearingLogs}
              onclick={() => void handleClearLogs()}
            >
              {#if clearingLogs}<IconLoading class="w-4 h-4 mr-1.5" />{:else}<IconDelete class="w-4 h-4 mr-1.5" />{/if}
              {i18n.t('settings.clear_logs')}
            </Button>
          </div>
        </SettingItem>

        <SettingItem
          title={i18n.t('settings.wipe_all_data')}
          description={i18n.t('settings.wipe_all_data_desc')}
          icon={IconWarning}
        >
          <Button
            variant="danger"
            disabled={wipePending}
            onclick={() => (showWipeConfirm = true)}
          >
            {#if wipePending}<IconLoading class="w-4 h-4 mr-1.5" />{:else}<IconDelete class="w-4 h-4 mr-1.5" />{/if}
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
      <SectionTitle icon={IconArrowSync} title={i18n.t('settings.updates_section')} onreset={() => (sectionToReset = 'updates')} />

      <div class="settings-list">
        <SettingItem
          title={i18n.t('settings.version_title')}
          description={i18n.t('settings.version_desc', { version: APP_VERSION })}
          icon={IconInfo}
        >
          <div class="settings-action-group">
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
          value={settings.auto_check_updates ?? true}
          defaultValue={defaultSettings.auto_check_updates ?? true}
          onReset={() => resetSetting('auto_check_updates')}
        >
          <ChoiceGroup
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
          icon={IconBranchFork}
          align="right"
          value={settings.include_prereleases ?? false}
          defaultValue={defaultSettings.include_prereleases ?? false}
          onReset={() => resetSetting('include_prereleases')}
        >
          <ChoiceGroup
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
      <span class="text-[11.5px] text-white/70 mt-0.5">
        {i18n.t('settings.built_on')} {formattedBuildTime}
      </span>
    </div>
  </div>
</PageShell>

<ConfirmDialog
  isOpen={showResetConfirm}
  title={i18n.t('settings.reset_all')}
  description={i18n.t('settings.reset_all_confirm')}
  confirmLabel={i18n.t('common.reset')}
  confirmVariant="danger"
  confirmIcon={IconArrowReset}
  loading={resetPending}
  onconfirm={() => void executeResetAllSettings()}
  onclose={() => (showResetConfirm = false)}
/>

<ConfirmDialog
  isOpen={Boolean(sectionToReset)}
  title={i18n.t('settings.reset_section_title')}
  description={i18n.t('settings.reset_section_confirm')}
  confirmLabel={i18n.t('common.reset')}
  confirmVariant="danger"
  confirmIcon={IconArrowReset}
  onconfirm={() => {
    if (sectionToReset) {
      const sec = sectionToReset;
      sectionToReset = null;
      void resetSection(sec);
    }
  }}
  onclose={() => (sectionToReset = null)}
/>

<ConfirmDialog
  isOpen={showClearAllCacheConfirm}
  title={i18n.t('settings.cache_clear_all')}
  description={i18n.t('settings.cache_clear_all_confirm')}
  confirmLabel={i18n.t('common.delete')}
  confirmVariant="danger"
  confirmIcon={IconDelete}
  loading={cacheBusy === 'all'}
  onconfirm={() => {
    showClearAllCacheConfirm = false;
    void clearCache('all');
  }}
  onclose={() => (showClearAllCacheConfirm = false)}
/>

<ConfirmDialog
  isOpen={showClearBgMediaConfirm}
  title={i18n.t('settings.background_media_clear')}
  description={i18n.t('settings.background_media_clear_confirm')}
  confirmLabel={i18n.t('common.delete')}
  confirmVariant="danger"
  confirmIcon={IconDelete}
  onconfirm={() => {
    showClearBgMediaConfirm = false;
    backgroundState.clearCustomMedia(backgroundState.settings.customKind as 'image' | 'video');
  }}
  onclose={() => (showClearBgMediaConfirm = false)}
/>

<ConfirmDialog
  isOpen={showWipeConfirm}
  title={i18n.t('settings.wipe_all_data')}
  description={i18n.t('settings.wipe_all_data_confirm')}
  confirmLabel={i18n.t('common.delete')}
  confirmVariant="danger"
  confirmIcon={IconDelete}
  loading={wipePending}
  onconfirm={() => void executeWipeAllData()}
  onclose={() => (showWipeConfirm = false)}
/>


<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: 100%;
    min-width: 0;
    gap: calc(32px * var(--ui-scale, 1));
    padding-bottom: 48px;
    overflow-x: clip;

    --control-height: var(--control-height-base);
    --control-font-size: var(--control-font-base);
    --control-icon-size: var(--control-icon-base);
    --control-padding-x: var(--control-padding-base);
    --control-radius: var(--control-radius-base);
  }

  .settings-hero-brand {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    padding-top: calc(8px * var(--ui-scale, 1));
  }

  .settings-hero-logo {
    height: calc(88px * var(--ui-scale, 1));
    width: auto;
    object-fit: contain;
    user-select: none;
    -webkit-user-drag: none;
    filter: drop-shadow(0 8px 36px rgba(254, 184, 173, 0.28));
    will-change: opacity, transform;
    transition: transform 300ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .settings-hero-logo.is-flying {
    opacity: 0;
    pointer-events: none;
  }

  .settings-hero-logo:hover {
    transform: scale(1.03);
  }

  .settings-page :global(.btn:not(.settings-toolbar *)) {
    --control-height: var(--control-height-base);
    --control-font-size: var(--control-font-base);
    --control-icon-size: var(--control-icon-base);
    --control-padding-x: var(--control-padding-base);
  }

  .settings-toolbar,
  .sticky-settings-toolbar {
    display: flex;
    align-items: center;
    width: 100%;
    min-width: 0;
    gap: 16px;
    min-height: calc(var(--control-height-md, 46px) * var(--ui-scale, 1));

    --control-height: var(--control-height-md, 46px) !important;
    --control-font-size: var(--control-font-md, 14px) !important;
    --control-icon-size: var(--control-icon-md, 20px) !important;
    --control-padding-x: var(--control-padding-md, 20px) !important;
    --control-radius: var(--control-radius-md, var(--radius-full)) !important;
  }

  .settings-toolbar :global(.btn),
  .sticky-settings-toolbar :global(.btn) {
    --control-height: var(--control-height-md, 46px) !important;
    --control-font-size: var(--control-font-md, 14px) !important;
    --control-icon-size: var(--control-icon-md, 20px) !important;
    --control-padding-x: var(--control-padding-md, 20px) !important;
  }

  .settings-toolbar {
    margin-bottom: 0;
  }

  .background-color-input {
    width: 100%;
    min-width: 0;
    height: var(--control-height, 40px);
    padding: 3px;
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-full);
    background: var(--bg-input);
    cursor: pointer;
  }

  .settings-accent-controls {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    width: 100%;
    max-width: 100%;
    min-width: 0;
    flex-wrap: wrap;
    min-height: var(--control-height, 40px);
    gap: 16px;
    margin-left: auto;
  }

  :global([data-layout='mobile']) .settings-accent-controls {
    justify-content: flex-start;
    margin-left: 0;
    gap: 14px;
  }

  @media (max-width: 640px) {
    .settings-accent-controls {
      justify-content: flex-start;
      margin-left: 0;
      gap: 14px;
    }
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
    font-family: var(--font-sans);
    max-width: 100%;
  }

  .mobile-pillar-sub {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.45);
    font-weight: 300;
    max-width: 100%;
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
    background: var(--accent-primary, #d69085);
  }

  .mobile-hero-dot.offline {
    background: rgba(255, 255, 255, 0.3);
  }

  @media (max-width: 640px) {
    .settings-page {
      gap: calc(28px * var(--ui-scale, 1));
      padding-bottom: 24px;
    }

    .settings-hero-logo {
      height: calc(64px * var(--ui-scale, 1));
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
