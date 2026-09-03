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
  import {
    apiSaveSettings,
    apiGetProviderAuthSchema,
    apiLogoutProviderSession,
    apiSyncProviderFavorites
  } from '$lib/utils/ipc';
  import type { ProviderConfig, ProviderAuthSchema } from '$lib/types/provider';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import SegmentedControl from '$lib/components/ui/SegmentedControl.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ProviderAuthModal from '$lib/components/providers/ProviderAuthModal.svelte';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconKey from '~icons/fluent/key-24-regular';
  import IconArrowSync from '~icons/fluent/arrow-sync-24-regular';
  import IconPerson from '~icons/fluent/person-24-regular';

  let authSchemas = $state<Record<string, ProviderAuthSchema>>({});
  let activeAuthModalProvider = $state<ProviderConfig | null>(null);
  let logoutConfirmProvider = $state<ProviderConfig | null>(null);
  let isLoggingOut = $state(false);
  let syncingProviderId = $state<string | null>(null);

  onMount(async () => {
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
        const next = { ...configState.settings, smart_merge_attachments: Boolean(val) };
        configState.updateSettings(next);
        void apiSaveSettings(next);
      }}
    />
  </SettingItem>

  <SettingItem
    title={i18n.t('settings.pawchive_hide_ai')}
    description={i18n.t('settings.pawchive_hide_ai_desc')}
    icon={IconSparkle}
    align="right"
  >
    <SegmentedControl
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

  {#each providerState.providers as provider (provider.id)}
    {@const pName = formatProviderName(provider.name)}
    {@const schema = authSchemas[provider.id]}
    {@const isAuthenticated = Boolean(provider.session_cookie && provider.session_cookie.trim())}

    <SettingItem
      title={pName}
      description={i18n.t('settings.provider_enable_desc', { provider: pName })}
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

    {#if schema?.supports_auth}
      <SettingItem
        title={`${pName} — ${i18n.t('settings.provider_auth')}`}
        description={isAuthenticated
          ? i18n.t('settings.provider_logged_in_as', { username: provider.username || 'Session' })
          : i18n.t('settings.provider_guest')}
        icon={isAuthenticated ? IconPerson : IconKey}
        align="right"
      >
        <div class="flex items-center gap-2.5 flex-wrap justify-end">
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
  {/each}
</div>

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
  <Modal
    isOpen={Boolean(logoutConfirmProvider)}
    title={i18n.t('settings.auth_logout_confirm_title', {
      provider: formatProviderName(logoutConfirmProvider.name)
    })}
    size="sm"
    onclose={() => (logoutConfirmProvider = null)}
  >
    <div class="modal-confirm-layout">
      <p class="modal-confirm-desc">
        {i18n.t('settings.auth_logout_confirm_desc')}
      </p>

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
    </div>
  </Modal>
{/if}

