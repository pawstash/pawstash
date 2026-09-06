<script lang="ts">
  import { onMount } from 'svelte';
  import { providerState } from '$lib/state/providerState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { feedState } from '$lib/state/feedState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { contentState } from '$lib/state/contentState.svelte';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import { formatProviderName } from '$lib/utils/media';
  import { logger } from '$lib/utils/logger';
  import {
    apiSaveSettings,
    apiGetDefaultSettings,
    apiGetProviderAuthSchema,
    apiLogoutProviderSession,
    apiSyncProviderFavorites
  } from '$lib/utils/ipc';
  import type { ProviderConfig, ProviderAuthSchema } from '$lib/types/provider';
  import type { AppSettings } from '$lib/types/config';
  import SectionTitle from '$lib/components/layout/SectionTitle.svelte';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import ChoiceGroup from '$lib/components/ui/ChoiceGroup.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import ProviderAuthModal from '$lib/components/providers/ProviderAuthModal.svelte';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconMerge from '~icons/fluent/merge-24-regular';
  import IconEyeOff from '~icons/fluent/eye-off-24-regular';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconKey from '~icons/fluent/key-24-regular';
  import IconArrowSync from '~icons/fluent/arrow-sync-24-regular';
  import IconPerson from '~icons/fluent/person-24-regular';
  import IconPower from '~icons/fluent/power-24-regular';
  import IconLink from '~icons/fluent/link-24-regular';

  let defaultSettings = $state<AppSettings>({ ...configState.settings });
  let authSchemas = $state<Record<string, ProviderAuthSchema>>({});
  let activeAuthModalProvider = $state<ProviderConfig | null>(null);
  let logoutConfirmProvider = $state<ProviderConfig | null>(null);
  let isLoggingOut = $state(false);
  let syncingProviderId = $state<string | null>(null);

  onMount(async () => {
    try {
      defaultSettings = await apiGetDefaultSettings();
    } catch (err) {
      logger.warn('[ProviderSettings] Failed to fetch default settings:', err);
    }
    if (providerState.providers.length === 0) {
      await providerState.loadProviders();
    }
    await loadAllSchemas();
  });

  async function loadAllSchemas() {
    for (const p of providerState.providers) {
      try {
        const schema = await apiGetProviderAuthSchema(p.id);
        authSchemas[p.id] = schema;
      } catch {
        authSchemas[p.id] = {
          provider_id: p.id,
          supports_auth: p.id === 'pawchive' || p.id === 'coomer',
          supports_remote_favorites: p.id === 'pawchive' || p.id === 'coomer',
          supports_push_favorites: p.id === 'pawchive' || p.id === 'coomer',
          auth_fields: []
        };
      }
    }
  }

  async function handleToggleEnabled(provider: ProviderConfig, enabled: boolean) {
    const updated: ProviderConfig = { ...provider, enabled };
    await providerState.updateProvider(updated);
    contentState.clearAllCache();
    void feedState.refresh();
    void creatorsState.refresh();
  }

  async function handleUpdateApiUrl(provider: ProviderConfig, raw: string) {
    const trimmed = raw.trim();
    if (!trimmed || trimmed === provider.api_url) return;
    const updated: ProviderConfig = { ...provider, api_url: trimmed };
    await providerState.updateProvider(updated);
    contentState.clearAllCache();
    void feedState.refresh();
    void creatorsState.refresh();
  }

  async function handleSyncFavorites(provider: ProviderConfig) {
    syncingProviderId = provider.id;
    const pName = formatProviderName(provider.name || provider.id);
    try {
      const res = await apiSyncProviderFavorites(provider.id, 'both');
      notify.success(
        i18n.t('settings.auth_sync_success', {
          pulled: res.pulled_count,
          pushed: res.pushed_count
        })
      );
      contentState.clearAllCache();
      void feedState.refresh();
      void creatorsState.refresh();
    } catch (e: any) {
      notify.error(typeof e === 'string' ? e : e?.message || 'Sync failed');
    } finally {
      syncingProviderId = null;
    }
  }

  async function handleConfirmLogout(removeFavorites: boolean) {
    if (!logoutConfirmProvider) return;
    const p = logoutConfirmProvider;
    isLoggingOut = true;
    try {
      await apiLogoutProviderSession(p.id, removeFavorites);
      await providerState.loadProviders();
      notify.success(i18n.t('settings.auth_logout_success', { provider: formatProviderName(p.name) }));
      contentState.clearAllCache();
      void feedState.refresh();
      void creatorsState.refresh();
    } catch (e: any) {
      notify.error(typeof e === 'string' ? e : e?.message || 'Logout failed');
    } finally {
      isLoggingOut = false;
      logoutConfirmProvider = null;
    }
  }

  function handleResetContentSources() {
    const defVal = defaultSettings.smart_merge_attachments ?? true;
    const next = { ...configState.settings, smart_merge_attachments: defVal };
    configState.updateSettings(next);
    void apiSaveSettings(next);
    notify.success(
      i18n.t('settings.reset_section_success'),
      i18n.t('settings.providers_section')
    );
  }

  async function handleResetProvider(provider: ProviderConfig) {
    const defProv = defaultSettings.providers?.find((p) => p.id === provider.id);
    const updated: ProviderConfig = {
      ...provider,
      enabled: defProv?.enabled ?? true,
      api_url: defProv?.api_url ?? provider.api_url
    };
    await providerState.updateProvider(updated);
    if (provider.id === 'pawchive') {
      const next = { ...configState.settings, pawchive_hide_ai: defaultSettings.pawchive_hide_ai ?? false };
      configState.updateSettings(next);
      void apiSaveSettings(next);
    }
    contentState.clearAllCache();
    void feedState.refresh();
    void creatorsState.refresh();
    notify.success(
      i18n.t('settings.reset_section_success'),
      formatProviderName(provider.name || provider.id)
    );
  }
</script>

<div id="settings-providers" class="settings-section">
  <SectionTitle icon={IconGlobe} title={i18n.t('settings.providers_section')} onreset={handleResetContentSources} />
  <div class="settings-list">
    <SettingItem
      title={i18n.t('settings.smart_merge_attachments')}
      description={i18n.t('settings.smart_merge_attachments_desc')}
      icon={IconMerge}
      align="right"
      value={configState.settings.smart_merge_attachments ?? true}
      defaultValue={defaultSettings.smart_merge_attachments ?? true}
      onReset={() => {
        const defVal = defaultSettings.smart_merge_attachments ?? true;
        const next = { ...configState.settings, smart_merge_attachments: defVal };
        configState.updateSettings(next);
        void apiSaveSettings(next);
      }}
    >
      <ChoiceGroup
        options={[
          { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
          { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
        ]}
        value={configState.settings.smart_merge_attachments ?? true}
        onchange={(val) => {
          const next = { ...configState.settings, smart_merge_attachments: Boolean(val) };
          configState.updateSettings(next);
          void apiSaveSettings(next);
        }}
      />
    </SettingItem>
  </div>
</div>

{#each providerState.providers as provider (provider.id)}
  {@const pName = formatProviderName(provider.name || provider.id)}
  {@const schema = authSchemas[provider.id]}
  {@const isAuthenticated = Boolean(provider.session_cookie && provider.session_cookie.trim())}
  {@const defProv = defaultSettings.providers?.find((p) => p.id === provider.id)}

  <div id="settings-provider-{provider.id}" class="settings-section">
    <SectionTitle icon={IconGlobe} title={pName} onreset={() => void handleResetProvider(provider)} />

    <div class="settings-list">
      <SettingItem
        title={i18n.t('settings.provider_active')}
        description={i18n.t('settings.provider_enable_desc', { provider: pName })}
        icon={IconPower}
        align="right"
        value={provider.enabled}
        defaultValue={defProv?.enabled ?? true}
        onReset={() => void handleToggleEnabled(provider, defProv?.enabled ?? true)}
      >
        <ChoiceGroup
          options={[
            { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
            { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
          ]}
          value={provider.enabled}
          onchange={(val) => handleToggleEnabled(provider, Boolean(val))}
        />
      </SettingItem>

      <SettingItem
        title={i18n.t('settings.provider_api_url')}
        description={i18n.t('settings.provider_api_url_desc', { provider: pName })}
        icon={IconLink}
        value={provider.api_url}
        defaultValue={defProv?.api_url ?? ''}
        onReset={() => void handleUpdateApiUrl(provider, defProv?.api_url ?? '')}
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

      {#if schema?.supports_auth}
        <SettingItem
          title={i18n.t('settings.provider_auth')}
          description={isAuthenticated
            ? i18n.t('settings.provider_logged_in_as', { username: provider.username || 'Session' })
            : i18n.t('settings.provider_guest')}
          icon={isAuthenticated ? IconPerson : IconKey}
        >
          <div class="flex items-center gap-2">
            {#if isAuthenticated}
              {#if schema.supports_remote_favorites}
                <Button
                  variant="primary"
                  onclick={() => handleSyncFavorites(provider)}
                  disabled={syncingProviderId === provider.id}
                >
                  <IconArrowSync
                    class="w-4 h-4 mr-1.5 {syncingProviderId === provider.id ? 'animate-spin' : ''}"
                  />
                  <span>{i18n.t('settings.provider_sync_favorites')}</span>
                </Button>
              {/if}
              <Button
                variant="danger"
                onclick={() => (logoutConfirmProvider = provider)}
              >
                <IconDismiss class="w-4 h-4 mr-1.5" />
                <span>{i18n.t('settings.provider_logout')}</span>
              </Button>
            {:else}
              <Button
                variant="primary"
                onclick={() => (activeAuthModalProvider = provider)}
              >
                <IconKey class="w-4 h-4 mr-1.5" />
                <span>{i18n.t('settings.provider_login')}</span>
              </Button>
            {/if}
          </div>
        </SettingItem>
      {/if}

      {#if provider.id === 'pawchive'}
        <SettingItem
          title={i18n.t('settings.pawchive_hide_ai')}
          description={i18n.t('settings.pawchive_hide_ai_desc')}
          icon={IconEyeOff}
          align="right"
          value={configState.settings.pawchive_hide_ai ?? false}
          defaultValue={defaultSettings.pawchive_hide_ai ?? false}
          onReset={() => {
            const defVal = defaultSettings.pawchive_hide_ai ?? false;
            const next = { ...configState.settings, pawchive_hide_ai: defVal };
            configState.updateSettings(next);
            void apiSaveSettings(next);
            contentState.posts = {};
            void feedState.refresh();
            void creatorsState.refresh();
          }}
        >
          <ChoiceGroup
            options={[
              { value: false, label: i18n.t('settings.no'), icon: IconDismiss },
              { value: true, label: i18n.t('settings.yes'), icon: IconCheck }
            ]}
            value={configState.settings.pawchive_hide_ai ?? false}
            onchange={(val) => {
              const next = { ...configState.settings, pawchive_hide_ai: Boolean(val) };
              configState.updateSettings(next);
              void apiSaveSettings(next);
              contentState.posts = {};
              void feedState.refresh();
              void creatorsState.refresh();
            }}
          />
        </SettingItem>
      {/if}
    </div>
  </div>
{/each}

{#if activeAuthModalProvider}
  <ProviderAuthModal
    isOpen={Boolean(activeAuthModalProvider)}
    provider={activeAuthModalProvider}
    onclose={() => (activeAuthModalProvider = null)}
    onsuccess={async () => {
      await providerState.loadProviders();
      contentState.posts = {};
      void feedState.refresh();
      void creatorsState.refresh();
    }}
  />
{/if}

{#if logoutConfirmProvider}
  <ConfirmDialog
    isOpen={Boolean(logoutConfirmProvider)}
    title={i18n.t('settings.auth_logout_confirm_title', {
      provider: formatProviderName(logoutConfirmProvider.name)
    })}
    description={i18n.t('settings.auth_logout_confirm_desc')}
    onclose={() => (logoutConfirmProvider = null)}
  >
    {#snippet actions()}
      <div class="flex flex-col gap-2 w-full">
        <Button
          variant="ghost"
          size="md"
          class="w-full justify-center px-3 border border-[var(--border-color)]"
          onclick={() => handleConfirmLogout(false)}
          disabled={isLoggingOut}
        >
          <IconCheck class="w-5 h-5 mr-1.5 shrink-0" />
          <span class="truncate">{i18n.t('settings.auth_logout_keep')}</span>
        </Button>

        <Button
          variant="danger"
          size="md"
          class="w-full justify-center px-3"
          onclick={() => handleConfirmLogout(true)}
          disabled={isLoggingOut}
        >
          <IconDismiss class="w-5 h-5 mr-1.5 shrink-0" />
          <span class="truncate">{i18n.t('settings.auth_logout_remove')}</span>
        </Button>
      </div>
    {/snippet}
  </ConfirmDialog>
{/if}
