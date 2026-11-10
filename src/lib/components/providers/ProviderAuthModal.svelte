<script lang="ts">
  import { untrack } from 'svelte';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import { formatProviderName } from '$lib/utils/media';
  import {
    apiGetProviderAuthSchema,
    apiLoginProvider,
    apiSyncProviderFavorites
  } from '$lib/utils/ipc';
  import type { ProviderConfig, ProviderAuthSchema } from '$lib/types/provider';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
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
    if (!isOpen) return;
    untrack(() => {
      step = 'login';
      errorMessage = null;
      formData = {
        session_cookie: provider.session_cookie || '',
        username: provider.username || ''
      };
      void loadSchema();
    });
  });

  function errorText(error: unknown, fallback: string): string {
    if (typeof error === 'string') return error;
    return (error as { message?: string } | null)?.message || fallback;
  }

  async function loadSchema() {
    loadingSchema = true;
    try {
      schema = await apiGetProviderAuthSchema(provider.id);
    } catch (error) {
      schema = null;
      errorMessage = errorText(error, i18n.t('settings.auth_login_failed'));
    } finally {
      loadingSchema = false;
    }
  }

  async function handleSubmitLogin() {
    errorMessage = null;

    if (schema?.auth_fields) {
      for (const field of schema.auth_fields) {
        if (field.required && !formData[field.key]?.trim()) {
          errorMessage = i18n.t('common.field_required', { field: i18n.t(field.label_key) });
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
    } catch (error) {
      errorMessage = errorText(error, i18n.t('settings.auth_login_failed'));
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
    } catch (error) {
      errorMessage = errorText(error, i18n.t('settings.auth_sync_failed'));
      step = 'sync_prompt';
    }
  }

  function handleSkipSync() {
    onsuccess?.();
    onclose();
  }
</script>

{#snippet authContent()}
  {#if step === 'login'}
    {#if errorMessage}
      {@render authError(errorMessage)}
    {/if}

    {#if schema?.auth_fields}
      <div class="auth-fields">
        {#each schema.auth_fields as field (field.key)}
          <div class="auth-field">
            <Input
              type={field.field_type === 'password' ? 'password' : 'text'}
              value={formData[field.key] || ''}
              label={i18n.t(field.label_key)}
              required={field.required}
              placeholder={field.placeholder || ''}
              oninput={(e) => {
                formData[field.key] = (e.target as HTMLInputElement).value;
              }}
            />
            {#if field.help_text_key}
              <p class="auth-help">{i18n.t(field.help_text_key)}</p>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  {:else if step === 'sync_prompt'}
    <p class="modal-confirm-desc">
      {i18n.t('settings.auth_sync_prompt_desc', { provider: providerName })}
    </p>

    {#if errorMessage}
      {@render authError(errorMessage)}
    {/if}
  {:else if step === 'syncing'}
    <div class="auth-syncing">
      <IconLoading />
      <p>{i18n.t('settings.auth_syncing')}</p>
    </div>
  {/if}
{/snippet}

{#snippet authError(message: string)}
  <div class="auth-error" role="alert">
    <IconDismiss />
    <span>{message}</span>
  </div>
{/snippet}

{#snippet authActions()}
  {#if step === 'login'}
    <div class="dialog-actions">
      <Button
        variant="ghost"
        size="md"
        class="justify-center"
        onclick={onclose}
        disabled={isSubmitting}
      >
        <span class="truncate">{i18n.t('common.cancel')}</span>
      </Button>
      <Button
        variant="ghost"
        data-tone="accent"
        size="md"
        class="justify-center"
        onclick={handleSubmitLogin}
        disabled={isSubmitting || loadingSchema || !schema}
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
    <div class="dialog-actions">
      <Button
        variant="ghost"
        size="md"
        class="justify-center"
        onclick={handleSkipSync}
      >
        <span class="truncate">{i18n.t('settings.auth_sync_skip')}</span>
      </Button>

      <Button
        variant="ghost"
        data-tone="accent"
        size="md"
        class="justify-center"
        onclick={() => handleExecuteSync('both')}
      >
        <IconArrowSync class="w-5 h-5 mr-1.5 shrink-0" />
        <span class="truncate">{i18n.t('settings.auth_sync_both')}</span>
      </Button>
    </div>
  {/if}
{/snippet}

{#if layoutState.isMobile}
  <BottomSheet
    {isOpen}
    title={step === 'login'
      ? i18n.t('settings.auth_modal_title', { provider: providerName })
      : i18n.t('settings.auth_sync_prompt_title')}
    closeLabel={i18n.t('common.cancel')}
    {onclose}
  >
    {#snippet children()}
      {@render authContent()}
    {/snippet}
    {#snippet footer()}
      {#if step !== 'syncing'}
        {@render authActions()}
      {/if}
    {/snippet}
  </BottomSheet>
{:else}
  <Modal
    {isOpen}
    title={step === 'login'
      ? i18n.t('settings.auth_modal_title', { provider: providerName })
      : i18n.t('settings.auth_sync_prompt_title')}
    size="sm"
    borderlessFooter
    {onclose}
  >
    <div class="modal-confirm-layout">
      {@render authContent()}
    </div>
    {#snippet footer()}{@render authActions()}{/snippet}
  </Modal>
{/if}

<style>
  .auth-fields {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .auth-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .auth-help {
    margin: 0;
    padding-inline: 4px;
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
  }

  .auth-error {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-bottom: 16px;
    padding: 12px 16px;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--status-error) 14%, transparent);
    color: var(--status-error);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
  }

  .auth-error :global(svg) {
    flex: none;
    width: 20px;
    height: 20px;
  }

  .auth-syncing {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding-block: 24px;
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
  }

  .auth-syncing p {
    margin: 0;
  }

  .auth-syncing :global(svg) {
    width: 32px;
    height: 32px;
    color: var(--accent-primary);
  }
</style>
