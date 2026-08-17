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

<div class="flex items-center justify-between gap-3.5 mb-4 select-none {extraClass}">
  <div class="flex items-center gap-3.5 min-w-0">
    {#if IconComponent}
      <div class="w-11 h-11 rounded-full bg-white/[0.06] flex items-center justify-center text-white shrink-0">
        <IconComponent class="w-[28px] h-[28px]" />
      </div>
    {/if}
    <h2 class="text-[18.5px] font-semibold text-white/90 font-outfit tracking-wide">{title}</h2>
  </div>

  {#if onreset}
    <Button
      variant="ghost"
      size="sm"
      class="opacity-20 hover:opacity-100 transition-opacity duration-200"
      onclick={onreset}
      title={i18n.t('settings.reset_section_tooltip') || 'Reset section to default settings'}
    >
      <IconArrowReset class="w-4 h-4" />
      <span>{defaultResetTitle}</span>
    </Button>
  {/if}
</div>
