<script lang="ts">
  import { i18n } from '$lib/i18n';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import NumberStepper from '$lib/components/ui/NumberStepper.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import type { AppSettings } from '$lib/types/config';
  import SettingsGroup from '../SettingsGroup.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';

  let settings = $derived(ctl.settings);
  let defaults = $derived(ctl.networkDefaults);
  let enabled = $derived(settings.cloud_scraping_enabled ?? true);
</script>

<SettingsGroup label={i18n.t('settings.group_cloud_links')}>
  <SettingItem
    title={i18n.t('settings.cloud_scraping_enabled')}
    description={i18n.t('settings.cloud_scraping_enabled_desc')}
    checked={enabled}
    onCheckedChange={(value) => void ctl.save('cloud_scraping_enabled', value)}
  />
  <SettingItem
    title={i18n.t('settings.content_indexing_enabled')}
    description={i18n.t('settings.content_indexing_enabled_desc')}
    checked={settings.content_indexing_enabled ?? true}
    onCheckedChange={(value) => void ctl.save('content_indexing_enabled', value)}
  />

  {#if enabled}
    <SettingItem
      title={i18n.t('settings.cloud_links_in_comments')}
      description={i18n.t('settings.cloud_links_in_comments_desc')}
      checked={settings.cloud_links_in_comments ?? false}
      onCheckedChange={(value) => void ctl.save('cloud_links_in_comments', value)}
    />
    {#if settings.cloud_links_in_comments}
      <SettingChoice
        title={i18n.t('settings.cloud_links_in_comments_from')}
        description={i18n.t('settings.cloud_links_in_comments_from_desc')}
        options={[
          { value: 'all', label: i18n.t('settings.comment_cloud_source_all') },
          { value: 'author', label: i18n.t('settings.comment_cloud_source_author') },
          { value: 'others', label: i18n.t('settings.comment_cloud_source_others') }
        ] as { value: NonNullable<AppSettings['cloud_links_in_comments_from']>; label: string }[]}
        value={settings.cloud_links_in_comments_from || 'all'}
        onchange={(source) => void ctl.save('cloud_links_in_comments_from', source)}
      />
    {/if}    <SettingItem title={i18n.t('settings.cloud_proxy_url')} description={i18n.t('settings.cloud_proxy_url_desc')}>
      <Input
        clearable={true}
        placeholder={i18n.t('settings.inherits_global_proxy')}
        bind:value={ctl.settings.cloud_proxy_url}
        onchange={() => void ctl.save('cloud_proxy_url', ctl.settings.cloud_proxy_url)}
      />
    </SettingItem>
    <SettingItem title={i18n.t('settings.cloud_user_agent')} description={i18n.t('settings.cloud_user_agent_desc')}>
      <Input
        clearable={true}
        placeholder={settings.network_browser_user_agent || defaults.browser_user_agent}
        bind:value={ctl.settings.cloud_user_agent}
        onchange={() => void ctl.save('cloud_user_agent', ctl.settings.cloud_user_agent)}
      />
    </SettingItem>
    <SettingItem title={i18n.t('settings.cloud_timeout')} description={i18n.t('settings.cloud_timeout_desc')}>
      <NumberStepper
        value={settings.cloud_timeout_secs || defaults.cloud_timeout_secs}
        min={5}
        max={600}
        step={5}
        onchange={(value) => void ctl.save('cloud_timeout_secs', value)}
      />
    </SettingItem>
    <SettingItem title={i18n.t('settings.cloud_max_redirects')} description={i18n.t('settings.cloud_max_redirects_desc')}>
      <NumberStepper
        value={settings.cloud_max_redirects || defaults.cloud_max_redirects}
        min={1}
        max={30}
        step={1}
        onchange={(value) => void ctl.save('cloud_max_redirects', value)}
      />
    </SettingItem>
  {/if}
</SettingsGroup>
