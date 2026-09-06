<script lang="ts">
  import IconArrowReset from '~icons/fluent/arrow-reset-24-regular';
  import Button from '$lib/components/ui/Button.svelte';
  import { i18n } from '$lib/i18n';

  interface Props {
    title: string;
    icon?: any;
    class?: string;
    onreset?: () => void;
    resetTitle?: string;
  }

  let {
    title,
    icon: IconComponent,
    class: extraClass = '',
    onreset,
    resetTitle
  }: Props = $props();

  let defaultResetTitle = $derived(resetTitle || i18n.t('settings.reset_section') || 'Default');
</script>

<div class="section-title {extraClass}">
  <div class="section-title__main">
    {#if IconComponent}
      <div class="section-title__icon">
        <IconComponent />
      </div>
    {/if}
    <h2 class="section-title__text">{title}</h2>
  </div>

  {#if onreset}
    <Button
      variant="ghost"
      size="sm"
      class="section-title__reset"
      onclick={onreset}
      title={i18n.t('settings.reset_section_tooltip') || 'Reset section to default settings'}
    >
      <IconArrowReset class="w-4 h-4" />
      <span>{defaultResetTitle}</span>
    </Button>
  {/if}
</div>

<style>
  .section-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    min-width: 0;
    gap: calc(14px * var(--ui-scale, 1));
    margin: 0;
    user-select: none;
  }

  .section-title__main {
    display: flex;
    align-items: center;
    min-width: 0;
    gap: calc(10px * var(--ui-scale, 1));
  }

  .section-title__icon {
    display: grid;
    place-items: center;
    flex: none;
    width: calc(34px * var(--ui-scale, 1));
    height: calc(34px * var(--ui-scale, 1));
    border-radius: calc(var(--radius-md) * var(--ui-scale, 1));
    background: var(--bg-card-hover);
    color: var(--text-primary);
  }

  .section-title__icon :global(svg) {
    width: calc(20px * var(--ui-scale, 1));
    height: calc(20px * var(--ui-scale, 1));
  }

  .section-title__text {
    min-width: 0;
    margin: 0;
    color: var(--text-primary);
    font-family: var(--font-display);
    font-size: calc(18px * var(--ui-scale, 1));
    font-weight: var(--font-weight-medium);
    line-height: var(--leading-tight);
    overflow-wrap: anywhere;
  }

  :global(.section-title__reset.btn) {
    opacity: 0.35;
    transition: opacity var(--duration-normal) var(--ease-expo);
  }

  :global(.section-title__reset.btn:hover),
  :global(.section-title__reset.btn:focus-visible) {
    opacity: 1;
  }
</style>
