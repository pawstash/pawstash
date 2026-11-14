<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import type { AppSettings } from '$lib/types/config';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';

  let settings = $derived(ctl.settings);
</script>

<SettingsGroup label={i18n.t('settings.group_viewer_exit')}>
  <SettingChoice
    title={i18n.t('settings.viewer_close_action')}
    description={i18n.t('settings.viewer_close_action_desc')}
    options={[
      { value: 'close', label: i18n.t('settings.viewer_close_close') },
      { value: 'dock', label: i18n.t('settings.viewer_close_dock') }
    ]}
    value={settings.viewer_close_action || 'close'}
    onchange={(value) => void ctl.save('viewer_close_action', value as AppSettings['viewer_close_action'])}
  />

  <SettingChoice
    title={i18n.t('settings.viewer_esc_action')}
    description={i18n.t('settings.viewer_esc_action_desc')}
    options={[
      { value: 'same', label: i18n.t('settings.viewer_esc_same') },
      { value: 'close', label: i18n.t('settings.viewer_esc_close') }
    ]}
    value={settings.viewer_esc_action || 'same'}
    onchange={(value) => void ctl.save('viewer_esc_action', value as AppSettings['viewer_esc_action'])}
  />
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_viewer_dock')}>
  <SettingChoice
    title={i18n.t('settings.viewer_dock_size')}
    description={i18n.t('settings.viewer_dock_size_desc')}
    options={[
      { value: 'small', label: i18n.t('settings.viewer_size_small') },
      { value: 'medium', label: i18n.t('settings.viewer_size_medium') },
      { value: 'large', label: i18n.t('settings.viewer_size_large') }
    ]}
    value={settings.viewer_dock_size || 'medium'}
    onchange={(value) => void ctl.save('viewer_dock_size', value as AppSettings['viewer_dock_size'])}
  />

  <SettingItem
    title={i18n.t('settings.viewer_dock_autohide')}
    description={i18n.t('settings.viewer_dock_autohide_desc')}
    checked={settings.viewer_dock_autohide ?? true}
    onCheckedChange={(checked: boolean) => void ctl.save('viewer_dock_autohide', checked)}
  />

  <SettingChoice
    title={i18n.t('settings.viewer_dock_delay')}
    description={i18n.t('settings.viewer_dock_delay_desc')}
    options={[
      { value: '0', label: i18n.t('settings.viewer_immediately') },
      { value: '1', label: i18n.t('settings.viewer_seconds', { count: 1 }) },
      { value: '2', label: i18n.t('settings.viewer_seconds', { count: 2 }) },
      { value: '4', label: i18n.t('settings.viewer_seconds', { count: 4 }) },
      { value: '8', label: i18n.t('settings.viewer_seconds', { count: 8 }) }
    ]}
    value={String(settings.viewer_dock_autohide_delay ?? 1)}
    onchange={(value) => void ctl.save('viewer_dock_autohide_delay', Number(value))}
  />

  <SettingChoice
    title={i18n.t('settings.viewer_dock_peek')}
    description={i18n.t('settings.viewer_dock_peek_desc')}
    options={[
      { value: '4', label: '4 px' },
      { value: '8', label: '8 px' },
      { value: '12', label: '12 px' },
      { value: '20', label: '20 px' },
      { value: '32', label: '32 px' }
    ]}
    value={String(settings.viewer_dock_peek ?? 12)}
    onchange={(value) => void ctl.save('viewer_dock_peek', Number(value))}
  />

  <SettingItem
    title={i18n.t('settings.viewer_dock_keep_playing')}
    description={i18n.t('settings.viewer_dock_keep_playing_desc')}
    checked={settings.viewer_dock_keep_playing ?? true}
    onCheckedChange={(checked: boolean) => void ctl.save('viewer_dock_keep_playing', checked)}
  />

  {#if layoutState.isMobileDevice || layoutState.isMobile}
    <SettingItem
      title={i18n.t('settings.viewer_dock_on_touch')}
      description={i18n.t('settings.viewer_dock_on_touch_desc')}
      checked={settings.viewer_dock_on_touch ?? true}
      onCheckedChange={(checked: boolean) => void ctl.save('viewer_dock_on_touch', checked)}
    />
  {/if}
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_viewer_playback')}>
  <SettingChoice
    title={i18n.t('settings.viewer_playback_rate')}
    description={i18n.t('settings.viewer_playback_rate_desc')}
    options={[
      { value: '0.5', label: '0.5×' },
      { value: '0.75', label: '0.75×' },
      { value: '1', label: '1×' },
      { value: '1.25', label: '1.25×' },
      { value: '1.5', label: '1.5×' },
      { value: '2', label: '2×' }
    ]}
    value={String(settings.viewer_playback_rate ?? 1)}
    onchange={(value) => void ctl.save('viewer_playback_rate', Number(value))}
  />

  <SettingItem
    title={i18n.t('settings.viewer_pause_on_blur')}
    description={i18n.t('settings.viewer_pause_on_blur_desc')}
    checked={settings.viewer_pause_on_blur ?? false}
    onCheckedChange={(checked: boolean) => void ctl.save('viewer_pause_on_blur', checked)}
  />
</SettingsGroup>
