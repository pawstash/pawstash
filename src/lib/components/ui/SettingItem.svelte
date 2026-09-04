<script lang="ts">
  import type { Snippet } from 'svelte';
  import { tooltip } from '$lib/motion';
  import IconUndo from '~icons/fluent/arrow-undo-24-regular';

  interface Props {
    title: string;
    description?: string;
    icon?: any;
    value?: unknown;
    defaultValue?: unknown;
    onReset?: () => void;
    children?: Snippet;
    class?: string;
    align?: 'left' | 'right';
  }

  let {
    title,
    description = '',
    icon: IconComponent,
    value,
    defaultValue,
    onReset,
    children,
    class: extraClass = '',
    align = 'left'
  }: Props = $props();

  let showReset = $derived(
    value !== undefined && defaultValue !== undefined && value !== defaultValue
  );
</script>

<div class="flex flex-col h-full min-w-0 max-w-full py-3 px-1 {extraClass}">
  <div class="flex flex-col min-w-0">
    <div class="flex items-center gap-2.5">
      {#if IconComponent}
        <IconComponent class="w-[20px] h-[20px] text-white/50 shrink-0" />
      {/if}
      <span class="text-[15px] font-normal text-white/85">{title}</span>
      {#if onReset && showReset}
        <button
          use:tooltip={'Reset to default'}
          onclick={(e) => {
            e.stopPropagation();
            onReset();
          }}
          class="p-1 rounded bg-white/10 hover:bg-white/20 text-gray-300 hover:text-white transition-colors"
          aria-label="Reset setting"
        >
          <IconUndo class="w-3.5 h-3.5" />
        </button>
      {/if}
    </div>

    {#if description}
      <p class="text-[12.5px] font-light text-white/40 leading-relaxed mt-1">
        {description}
      </p>
    {/if}
  </div>

  <div
    class="flex items-center min-w-0 max-w-full w-full mt-auto pt-3"
    class:justify-end={align === 'right'}
    class:justify-start={align === 'left'}
  >
    {#if children}
      {@render children()}
    {/if}
  </div>
</div>
