<script lang="ts">
  import type { Snippet, Component } from 'svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import Modal from '$lib/components/ui/Modal.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  interface Props {
    isOpen?: boolean;
    title: string;
    description?: string;
    confirmLabel?: string;
    confirmVariant?: 'accent' | 'danger' | 'primary';
    confirmIcon?: Component;
    cancelLabel?: string;
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
    loading = false,
    disabled = false,
    onconfirm,
    oncancel,
    onclose,
    children,
    actions
  }: Props = $props();

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
    <div class="confirm-actions">
      <Button
        variant="ghost"
        size="md"
        class="w-full justify-center px-3 border border-[var(--border-color)]"
        disabled={loading || disabled}
        onclick={handleClose}
      >
        <span class="truncate">{cancelLabel || i18n.t('common.cancel')}</span>
      </Button>

      <Button
        variant={confirmVariant}
        size="md"
        class="w-full justify-center px-3"
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
    onclose={handleClose}
  >
    <div class="modal-confirm-layout">
      {@render bodyContent()}
      {@render actionsContent()}
    </div>
  </Modal>
{/if}

<style>
  .confirm-desc {
    margin: 0;
    font-size: calc(var(--text-sm, 13px) * var(--ui-scale, 1));
    line-height: var(--leading-relaxed, 1.5);
    color: var(--text-secondary);
    padding: 0 0 calc(14px * var(--ui-scale, 1)) 0;
    box-sizing: border-box;
  }

  :global(.bottom-sheet) .confirm-desc {
    padding: 0 0 calc(6px * var(--ui-scale, 1)) 0;
    font-size: calc(var(--text-sm, 14px) * var(--ui-scale, 1));
  }

  .confirm-actions {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: calc(var(--floating-gap, 6px) * var(--ui-scale, 1));
    width: 100%;
    box-sizing: border-box;
  }

  :global(.bottom-sheet) .confirm-actions {
    gap: calc(10px * var(--ui-scale, 1));
  }
</style>
