<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import { formatProviderName } from '$lib/utils/media';
  import {
    apiGetProviderAuthSchema,
    apiLoginProvider,
    apiSyncProviderFavorites
  } from '$lib/utils/ipc';
  import type { ProviderConfig, ProviderAuthSchema } from '$lib/types/provider';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconArrowSync from '~icons/fluent/arrow-sync-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  interface Props {
    isOpen: boolean;
    provider: ProviderConfig;
    onclose: () => void;
    onsuccess?: () => void;
  }

  let { isOpen, provider, onclose, onsuccess }: Props = $props();

  let schema = $state<ProviderAuthSchema | null>(null);
  let loadingSchema = $state(true);
  let isSubmitting = $state(false);
  let step = $state<'login' | 'sync_prompt' | 'syncing'>('login');
  let formData = $state<Record<string, string>>({});
  let errorMessage = $state<string | null>(null);

  const providerName = $derived(formatProviderName(provider.name || provider.id));

  $effect(() => {
    if (isOpen && provider) {
      step = 'login';
      errorMessage = null;
      formData = {
        session_cookie: provider.session_cookie || '',
        username: provider.username || ''
      };
      void loadSchema();
    }
  });

  async function loadSchema() {
    loadingSchema = true;
    try {
      schema = await apiGetProviderAuthSchema(provider.id);
    } catch {
      schema = {
        provider_id: provider.id,
        supports_auth: true,
        supports_remote_favorites: false,
        supports_push_favorites: false,
        auth_fields: [
          {
            key: 'username',
            label_key: 'settings.username',
            field_type: 'text',
            placeholder: 'Username',
            help_text_key: undefined,
            required: true
          },
          {
            key: 'password',
            label_key: 'settings.password',
            field_type: 'password',
            placeholder: '••••••••',
            help_text_key: undefined,
            required: true
          }
        ],
        help_url: provider.api_url
      };
    } finally {
      loadingSchema = false;
    }
  }

  async function handleSubmitLogin() {
    errorMessage = null;

    if (schema?.auth_fields) {
      for (const field of schema.auth_fields) {
        if (field.required && !formData[field.key]?.trim()) {
          errorMessage = `${i18n.t(field.label_key)} is required`;
          return;
        }
      }
    }

    isSubmitting = true;
    try {
      await apiLoginProvider(provider.id, formData);
      notify.success(i18n.t('settings.auth_login_success', { provider: providerName }));
      
      if (schema?.supports_remote_favorites) {
        step = 'sync_prompt';
      } else {
        onsuccess?.();
        onclose();
      }
    } catch (e: any) {
      errorMessage = typeof e === 'string' ? e : e?.message || 'Login failed';
    } finally {
      isSubmitting = false;
    }
  }

  async function handleExecuteSync(direction: 'both' | 'pull') {
    step = 'syncing';
    errorMessage = null;
    try {
      const res = await apiSyncProviderFavorites(provider.id, direction);
      notify.success(
        i18n.t('settings.auth_sync_success', {
          pulled: res.pulled_count,
          pushed: res.pushed_count
        })
      );
      onsuccess?.();
      onclose();
    } catch (e: any) {
      errorMessage = typeof e === 'string' ? e : e?.message || 'Sync failed';
      step = 'sync_prompt';
    }
  }

  function handleSkipSync() {
    onsuccess?.();
    onclose();
  }
</script>

<Modal
  {isOpen}
  title={step === 'login'
    ? i18n.t('settings.auth_modal_title', { provider: providerName })
    : i18n.t('settings.auth_sync_prompt_title')}
  size="sm"
  {onclose}
>
  <div class="modal-confirm-layout">
    {#if step === 'login'}
      {#if errorMessage}
        <div class="mx-2.5 mb-3 p-2.5 rounded-xl bg-rose-500/10 border border-rose-500/20 text-rose-300 text-xs flex items-center gap-2">
          <IconDismiss class="w-4 h-4 shrink-0" />
          <span>{errorMessage}</span>
        </div>
      {/if}

      {#if schema?.auth_fields}
        <div class="flex flex-col gap-3 px-2.5 mb-3">
          {#each schema.auth_fields as field (field.key)}
            <div class="flex flex-col gap-1.5">
              <label for={`auth-field-${field.key}`} class="text-[11px] font-semibold uppercase tracking-wider text-[var(--text-secondary)]">
                {i18n.t(field.label_key)}
                {#if field.required}<span class="text-rose-400 ml-0.5">*</span>{/if}
              </label>
              <Input
                type={field.field_type === 'password' ? 'password' : 'text'}
                value={formData[field.key] || ''}
                placeholder={field.placeholder || ''}
                oninput={(e) => {
                  formData[field.key] = (e.target as HTMLInputElement).value;
                }}
              />
              {#if field.help_text_key}
                <p class="text-[11px] text-[var(--text-secondary)] leading-relaxed">
                  {i18n.t(field.help_text_key)}
                </p>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      <div class="modal-confirm-actions">
        <Button
          variant="ghost"
          size="md"
          class="w-full justify-center px-3 border border-[var(--border-color)]"
          onclick={onclose}
          disabled={isSubmitting}
        >
          <span class="truncate">{i18n.t('common.cancel')}</span>
        </Button>
        <Button
          variant="accent"
          size="md"
          class="w-full justify-center px-3"
          onclick={handleSubmitLogin}
          disabled={isSubmitting || loadingSchema}
        >
          {#if isSubmitting}
            <IconLoading class="w-5 h-5 mr-1.5 shrink-0" />
          {:else}
            <IconCheck class="w-5 h-5 mr-1.5 shrink-0" />
          {/if}
          <span class="truncate">{i18n.t('settings.provider_login')}</span>
        </Button>
      </div>

    {:else if step === 'sync_prompt'}
      <p class="modal-confirm-desc">
        {i18n.t('settings.auth_sync_prompt_desc', { provider: providerName })}
      </p>

      {#if errorMessage}
        <div class="mx-2.5 mb-3 p-2.5 rounded-xl bg-rose-500/10 border border-rose-500/20 text-rose-300 text-xs flex items-center gap-2">
          <IconDismiss class="w-4 h-4 shrink-0" />
          <span>{errorMessage}</span>
        </div>
      {/if}

      <div class="modal-confirm-actions">
        <Button
          variant="ghost"
          size="md"
          class="w-full justify-center px-3 border border-[var(--border-color)]"
          onclick={handleSkipSync}
        >
          <span class="truncate">{i18n.t('settings.auth_sync_skip')}</span>
        </Button>

        <Button
          variant="accent"
          size="md"
          class="w-full justify-center px-3"
          onclick={() => handleExecuteSync('both')}
        >
          <IconArrowSync class="w-5 h-5 mr-1.5 shrink-0" />
          <span class="truncate">{i18n.t('settings.auth_sync_both')}</span>
        </Button>
      </div>

    {:else if step === 'syncing'}
      <div class="flex flex-col items-center justify-center py-6 gap-3">
        <IconLoading class="w-8 h-8 text-[var(--accent)]" />
        <p class="modal-confirm-desc text-center !p-0">
          {i18n.t('settings.auth_syncing')}
        </p>
      </div>
    {/if}
  </div>
</Modal>
