<script lang="ts">
  import { onMount } from 'svelte';
  import { providerState } from '$lib/state/providerState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { feedState } from '$lib/state/feedState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { contentState } from '$lib/state/contentState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatProviderName } from '$lib/utils/media';
  import type { ProviderConfig } from '$lib/types/provider';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import SegmentedControl from '$lib/components/ui/SegmentedControl.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';

  onMount(async () => {
    if (providerState.providers.length === 0) {
      await providerState.loadProviders();
    }
  });

  async function handleToggleEnabled(provider: ProviderConfig, enabled: boolean) {
    const updated: ProviderConfig = { ...provider, enabled };
    await providerState.updateProvider(updated);
    contentState.posts = {};
    void feedState.refresh();
    void creatorsState.refresh();
  }

  async function handleUpdateApiUrl(provider: ProviderConfig, raw: string) {
    const trimmed = raw.trim();
    if (!trimmed || trimmed === provider.api_url) return;
    const updated: ProviderConfig = { ...provider, api_url: trimmed };
    await providerState.updateProvider(updated);
    contentState.posts = {};
    void feedState.refresh();
    void creatorsState.refresh();
  }
</script>

<div class="grid grid-cols-1 md:grid-cols-2 gap-x-16 gap-y-6">
  <SettingItem
    title={i18n.t('settings.smart_merge_attachments')}
    description={i18n.t('settings.smart_merge_attachments_desc')}
    icon={IconSparkle}
    align="right"
  >
    <SegmentedControl
      options={[
        { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
        { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
      ]}
      value={configState.settings.smart_merge_attachments ?? true}
      onchange={(val) => {
        configState.updateSettings({ ...configState.settings, smart_merge_attachments: Boolean(val) });
      }}
    />
  </SettingItem>

  {#each providerState.providers as provider (provider.id)}
    {@const pName = formatProviderName(provider.name)}
    {@const servicesFormatted = provider.services.length > 0 
      ? provider.services.map((s) => s.charAt(0).toUpperCase() + s.slice(1)).join(', ') 
      : 'All'}

    <SettingItem
      title={pName}
      description={i18n.t('settings.provider_enable_desc', { provider: pName, services: servicesFormatted })}
      icon={IconGlobe}
      align="right"
    >
      <SegmentedControl
        options={[
          { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
          { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
        ]}
        value={provider.enabled}
        onchange={(val) => handleToggleEnabled(provider, Boolean(val))}
      />
    </SettingItem>

    <SettingItem
      title={`${pName} — ${i18n.t('settings.provider_api_url')}`}
      description={i18n.t('settings.provider_api_url_desc', { provider: pName })}
      icon={IconGlobe}
    >
      <div class="w-full">
        <Input
          clearable={true}
          value={provider.api_url}
          placeholder="https://..."
          onblur={(e) => handleUpdateApiUrl(provider, (e.target as HTMLInputElement).value)}
        />
      </div>
    </SettingItem>
  {/each}
</div>
