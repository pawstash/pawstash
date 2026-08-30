<script lang="ts">
  import { syncState } from '$lib/state/syncState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import Select from '$lib/components/ui/Select.svelte';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import SegmentedControl from '$lib/components/ui/SegmentedControl.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconPersonKey from '~icons/fluent/person-key-24-regular';
  import IconCloudSync from '~icons/fluent/cloud-sync-24-regular';
  import IconTimer from '~icons/fluent/timer-24-regular';
  import IconFlash from '~icons/fluent/flash-24-regular';
  import IconArrowSync from '~icons/fluent/arrow-sync-24-regular';
  import IconHeart from '~icons/fluent/heart-24-regular';

  async function updateSetting<K extends keyof typeof configState.settings>(
    key: K,
    val: (typeof configState.settings)[K]
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
    } catch {}
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

<div class="grid grid-cols-1 md:grid-cols-2 gap-x-16 gap-y-6">
  <SettingItem
    title={i18n.t('sync.enable_sync')}
    description={i18n.t('sync.enable_sync_desc')}
    icon={IconCloudSync}
    align="right"
  >
    <SegmentedControl
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
  >
    <SegmentedControl
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
  >
    <SegmentedControl
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
  >
    <SegmentedControl
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
  >
    <SegmentedControl
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
    icon={IconTimer}
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
    icon={IconTimer}
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
