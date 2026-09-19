<script lang="ts">
  import type { Snippet, Component } from 'svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import Modal from '$lib/components/ui/Modal.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconWarning from '~icons/fluent/warning-24-filled';

  interface Props {
    isOpen?: boolean;
    title: string;
    description?: string;
    confirmLabel?: string;
    confirmVariant?: 'accent' | 'danger' | 'primary';
    confirmIcon?: Component;
    cancelLabel?: string;
    heroIcon?: Component;
    loading?: boolean;
    disabled?: boolean;
    onconfirm?: () => void;
    oncancel?: () => void;
    onclose: () => void;
    children?: Snippet;
    actions?: Snippet;
  }

  let {
    isOpen = false,
    title,
    description,
    confirmLabel,
    confirmVariant = 'accent',
    confirmIcon,
    cancelLabel,
    heroIcon,
    loading = false,
    disabled = false,
    onconfirm,
    oncancel,
    onclose,
    children,
    actions
  }: Props = $props();

  const heroTone = $derived(confirmVariant === 'danger' ? 'danger' : 'accent');
  const desktopHero = $derived(heroIcon ?? (confirmVariant === 'danger' ? IconWarning : undefined));

  function handleClose() {
    if (loading) return;
    oncancel?.();
    onclose();
  }

  function handleConfirm() {
    if (loading || disabled) return;
    onconfirm?.();
  }
</script>

{#snippet bodyContent()}
  {#if children}
    {@render children()}
  {:else if description}
    <p class="confirm-desc">{description}</p>
  {/if}
{/snippet}

{#snippet actionsContent()}
  {#if actions}
    {@render actions()}
  {:else}
    <div class="dialog-actions">
      <Button
        variant="ghost"
        size="md"
        class="justify-center"
        disabled={loading || disabled}
        onclick={handleClose}
      >
        <span class="truncate">{cancelLabel || i18n.t('common.cancel')}</span>
      </Button>

      <Button
        variant="ghost"
        data-tone={confirmVariant}
        size="md"
        class="justify-center"
        disabled={loading || disabled}
        onclick={handleConfirm}
      >
        {#if loading}
          <IconLoading class="w-5 h-5 mr-1.5 shrink-0" />
        {:else if confirmIcon}
          {@const Icon = confirmIcon}
          <Icon class="w-5 h-5 mr-1.5 shrink-0" />
        {/if}
        <span class="truncate">{confirmLabel || i18n.t('common.confirm')}</span>
      </Button>
    </div>
  {/if}
{/snippet}

{#if layoutState.isMobile}
  <BottomSheet
    {isOpen}
    {title}
    closeLabel={cancelLabel || i18n.t('common.cancel')}
    onclose={handleClose}
  >
    {#snippet children()}
      {@render bodyContent()}
    {/snippet}
    {#snippet footer()}
      {@render actionsContent()}
    {/snippet}
  </BottomSheet>
{:else}
  <Modal
    {isOpen}
    {title}
    size="sm"
    icon={desktopHero}
    tone={heroTone}
    dismissible={false}
    borderlessFooter
    onclose={handleClose}
  >
    {#snippet children()}
      {@render bodyContent()}
    {/snippet}
    {#snippet footer()}
      {@render actionsContent()}
    {/snippet}
  </Modal>
{/if}

<style>
  .confirm-desc {
    margin: 0;
    font-size: calc(var(--text-sm, 13px) * var(--ui-scale, 1));
    line-height: var(--leading-relaxed, 1.5);
    color: var(--text-secondary);
    box-sizing: border-box;
  }

  :global(.bottom-sheet) .confirm-desc {
    font-size: calc(var(--text-sm, 14px) * var(--ui-scale, 1));
  }

</style>
