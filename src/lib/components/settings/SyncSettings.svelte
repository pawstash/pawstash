<script lang="ts">
  import { onMount } from 'svelte';
  import { syncState } from '$lib/state/syncState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import Select from '$lib/components/ui/Select.svelte';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import ChoiceGroup from '$lib/components/ui/ChoiceGroup.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { apiGetDefaultSettings } from '$lib/utils/ipc';
  import { logger } from '$lib/utils/logger';
  import type { AppSettings } from '$lib/types/config';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconPersonKey from '~icons/fluent/person-key-24-regular';
  import IconCloudSync from '~icons/fluent/cloud-sync-24-regular';
  import IconCloudArrowUp from '~icons/fluent/cloud-arrow-up-24-regular';
  import IconCloudArrowDown from '~icons/fluent/cloud-arrow-down-24-regular';
  import IconFlash from '~icons/fluent/flash-24-regular';
  import IconArrowSync from '~icons/fluent/arrow-sync-24-regular';
  import IconHeart from '~icons/fluent/heart-24-regular';

  let defaultSettings = $state<AppSettings>({ ...configState.settings });

  onMount(async () => {
    try {
      defaultSettings = await apiGetDefaultSettings();
    } catch (err) {
      logger.warn('[SyncSettings] Failed to fetch default settings:', err);
    }
  });

  async function updateSetting<K extends keyof AppSettings>(
    key: K,
    val: AppSettings[K]
  ) {
    const updated = { ...configState.settings, [key]: val };
    configState.updateSettings(updated);
    try {
      await invoke('save_settings', { settings: updated });
      if (key === 'sync_enabled') {
        await syncState.setEnabled(Boolean(val));
      }
      if (key === 'sync_pawchive_session' && syncState.status.configured && syncState.status.enabled && syncState.status.unlocked) {
        void syncState.sync();
      }
    } catch (err) {
      logger.error('[SyncSettings] Failed to save setting:', key, err);
    }
  }

  const pushIntervalOptions = $derived([
    { value: 15, label: i18n.t('sync.interval_15s') },
    { value: 30, label: i18n.t('sync.interval_30s') },
    { value: 60, label: i18n.t('sync.interval_1m') },
    { value: 120, label: i18n.t('sync.interval_2m') },
    { value: 300, label: i18n.t('sync.interval_5m') },
    { value: 900, label: i18n.t('sync.interval_15m') },
    { value: 1800, label: i18n.t('sync.interval_30m') }
  ]);

  const pullIntervalOptions = $derived([
    { value: 30, label: i18n.t('sync.interval_30s') },
    { value: 60, label: i18n.t('sync.interval_1m') },
    { value: 120, label: i18n.t('sync.interval_2m') },
    { value: 300, label: i18n.t('sync.interval_5m') },
    { value: 600, label: i18n.t('sync.interval_10m') },
    { value: 900, label: i18n.t('sync.interval_15m') },
    { value: 1800, label: i18n.t('sync.interval_30m') }
  ]);
</script>

<div class="settings-list">
  <SettingItem
    title={i18n.t('sync.enable_sync')}
    description={i18n.t('sync.enable_sync_desc')}
    icon={IconCloudSync}
    align="right"
    value={configState.settings.sync_enabled}
    defaultValue={defaultSettings.sync_enabled}
    onReset={() => void updateSetting('sync_enabled', defaultSettings.sync_enabled)}
  >
    <ChoiceGroup
      options={[
        { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
        { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
      ]}
      value={configState.settings.sync_enabled}
      onchange={(val) => void updateSetting('sync_enabled', Boolean(val))}
    />
  </SettingItem>

  <SettingItem
    title={i18n.t('sync.auto_sync')}
    description={i18n.t('sync.auto_sync_desc')}
    icon={IconArrowSync}
    align="right"
    value={configState.settings.sync_auto}
    defaultValue={defaultSettings.sync_auto}
    onReset={() => void updateSetting('sync_auto', defaultSettings.sync_auto)}
  >
    <ChoiceGroup
      options={[
        { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
        { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
      ]}
      value={configState.settings.sync_auto}
      onchange={(val) => void updateSetting('sync_auto', Boolean(val))}
    />
  </SettingItem>

  <SettingItem
    title={i18n.t('sync.sync_on_change')}
    description={i18n.t('sync.sync_on_change_desc')}
    icon={IconFlash}
    align="right"
    value={configState.settings.sync_on_change}
    defaultValue={defaultSettings.sync_on_change}
    onReset={() => void updateSetting('sync_on_change', defaultSettings.sync_on_change)}
  >
    <ChoiceGroup
      options={[
        { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
        { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
      ]}
      value={configState.settings.sync_on_change}
      onchange={(val) => void updateSetting('sync_on_change', Boolean(val))}
    />
  </SettingItem>

  <SettingItem
    title={i18n.t('sync.sync_pawchive_session')}
    description={i18n.t('sync.sync_pawchive_session_desc')}
    icon={IconPersonKey}
    align="right"
    value={configState.settings.sync_pawchive_session}
    defaultValue={defaultSettings.sync_pawchive_session}
    onReset={() => void updateSetting('sync_pawchive_session', defaultSettings.sync_pawchive_session)}
  >
    <ChoiceGroup
      options={[
        { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
        { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
      ]}
      value={configState.settings.sync_pawchive_session}
      onchange={(val) => void updateSetting('sync_pawchive_session', Boolean(val))}
    />
  </SettingItem>

  <SettingItem
    title={i18n.t('sync.persist_in_app_favorites_locally')}
    description={i18n.t('sync.persist_in_app_favorites_locally_desc')}
    icon={IconHeart}
    align="right"
    value={configState.settings.persist_in_app_favorites_locally ?? true}
    defaultValue={defaultSettings.persist_in_app_favorites_locally ?? true}
    onReset={() => void updateSetting('persist_in_app_favorites_locally', defaultSettings.persist_in_app_favorites_locally ?? true)}
  >
    <ChoiceGroup
      options={[
        { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
        { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
      ]}
      value={configState.settings.persist_in_app_favorites_locally ?? true}
      onchange={(val) => void updateSetting('persist_in_app_favorites_locally', Boolean(val))}
    />
  </SettingItem>

  <SettingItem
    title={i18n.t('sync.push_interval')}
    description={i18n.t('sync.push_interval_desc')}
    icon={IconCloudArrowUp}
    value={configState.settings.sync_push_interval_seconds}
    defaultValue={defaultSettings.sync_push_interval_seconds}
    onReset={() => void updateSetting('sync_push_interval_seconds', defaultSettings.sync_push_interval_seconds)}
  >
    <div class="w-full">
      <Select
        options={pushIntervalOptions}
        value={configState.settings.sync_push_interval_seconds}
        onchange={(val) => void updateSetting('sync_push_interval_seconds', Number(val))}
      />
    </div>
  </SettingItem>

  <SettingItem
    title={i18n.t('sync.pull_interval')}
    description={i18n.t('sync.pull_interval_desc')}
    icon={IconCloudArrowDown}
    value={configState.settings.sync_pull_interval_seconds}
    defaultValue={defaultSettings.sync_pull_interval_seconds}
    onReset={() => void updateSetting('sync_pull_interval_seconds', defaultSettings.sync_pull_interval_seconds)}
  >
    <div class="w-full">
      <Select
        options={pullIntervalOptions}
        value={configState.settings.sync_pull_interval_seconds}
        onchange={(val) => void updateSetting('sync_pull_interval_seconds', Number(val))}
      />
    </div>
  </SettingItem>
</div>
