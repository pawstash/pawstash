<script lang="ts">
  import { untrack } from 'svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { noticeState } from '$lib/state/noticeState.svelte';
  import { openExternalUrl } from '$lib/utils/ipc';
  import { i18n } from '$lib/i18n';
  import MaterialShape from '$lib/components/ui/MaterialShape.svelte';
  import { heroShapeCycle } from '$lib/utils/materialShapes';
  import IconWarning from '~icons/fluent/warning-24-filled';
  import IconError from '~icons/fluent/error-circle-24-filled';

  let notice = $derived(noticeState.current);
  let heroNonce = $state(Math.random());
  let lastNotice = untrack(() => notice);
  $effect.pre(() => {
    if (!notice) {
      lastNotice = null;
      return;
    }
    if (notice !== lastNotice) {
      heroNonce = Math.random();
      lastNotice = notice;
    }
  });
  let noticeTone = $derived<'accent' | 'danger'>(notice?.severity === 'critical' ? 'danger' : 'accent');
  let heroCycle = $derived(heroShapeCycle(`${notice?.title || 'notice'}\u0001${heroNonce}`, noticeTone));
  let heroShape = $derived(heroCycle[0] ?? 'circle');
  let sheet = $state<BottomSheet>();

  function dismiss() {
    if (layoutState.isMobile) sheet?.close();
    else noticeState.close();
  }

  function follow(url: string) {
    void openExternalUrl(url);
    dismiss();
  }
</script>

{#snippet body()}
  {#if notice}
    <div class="notice-body" class:is-critical={notice.severity === 'critical'}>
      <span class="notice-glyph" aria-hidden="true">
        <MaterialShape
          name={heroShape}
          size={40}
          duration={700}
          cycle={heroCycle}
          spin
          class="notice-glyph__shape"
        />
        <span class="notice-glyph__glyph">
          {#if notice.severity === 'critical'}<IconError />{:else}<IconWarning />{/if}
        </span>
      </span>
      {#if notice.body}<p class="notice-text">{notice.body}</p>{/if}
    </div>
  {/if}
{/snippet}

{#snippet actions()}
  {#if notice}
    <div class="dialog-actions">
      <Button variant="ghost" size="md" onclick={dismiss}>
        {i18n.t('common.close')}
      </Button>
      {#each notice.actions as action, index (index)}
        <Button variant="ghost" size="md" data-tone={index === 0 ? 'accent' : undefined} onclick={() => follow(action.url)}>
          {action.label}
        </Button>
      {/each}
    </div>
  {/if}
{/snippet}

{#if notice}
  {#if layoutState.isMobile}
    <BottomSheet bind:this={sheet} isOpen title={notice.title} closeLabel={i18n.t('common.close')} onclose={() => noticeState.close()}>
      {@render body()}
      {#snippet footer()}{@render actions()}{/snippet}
    </BottomSheet>
  {:else}
    <Modal isOpen title={notice.title} size="md" borderlessFooter onclose={() => noticeState.close()}>
      {@render body()}
      {#snippet footer()}{@render actions()}{/snippet}
    </Modal>
  {/if}
{/if}

<style>
  .notice-body {
    display: flex;
    align-items: flex-start;
    gap: 14px;
  }

  .notice-glyph {
    position: relative;
    display: grid;
    flex: none;
    place-items: center;
    width: 40px;
    height: 40px;
    color: var(--status-warning);
  }

  .notice-glyph :global(.notice-glyph__shape) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    color: color-mix(in srgb, var(--status-warning) 16%, transparent);
  }

  .is-critical .notice-glyph {
    color: var(--status-error);
  }

  .is-critical .notice-glyph :global(.notice-glyph__shape) {
    color: color-mix(in srgb, var(--status-error) 16%, transparent);
  }

  .notice-glyph__glyph {
    position: relative;
    display: grid;
    place-items: center;
  }

  .notice-glyph__glyph :global(svg) {
    width: 22px;
    height: 22px;
  }

  .notice-text {
    margin: 0;
    padding-top: 8px;
    color: var(--text-primary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
    white-space: pre-line;
    overflow-wrap: anywhere;
  }
</style>
