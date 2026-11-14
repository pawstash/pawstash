<script lang="ts" generics="T extends string | number">
  import { i18n } from '$lib/i18n';
  import { ripple, tooltip } from '$lib/motion';
  import Select from '$lib/components/ui/Select.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import type { SortOption } from '$lib/utils/sort';
  import IconArrowSort from '~icons/fluent/arrow-sort-24-regular';

  interface Props {
    options: SortOption<T>[];
    value: T;
    onchange: (value: T) => void;
    variant?: 'addon' | 'standalone';
  }

  let { options, value, onchange, variant = 'addon' }: Props = $props();

  let current = $derived(options.find((option) => option.value === value));
  let label = $derived(`${i18n.t('sort.by')}: ${current?.ariaLabel ?? current?.label ?? ''}`);
</script>

<Select
  {options}
  {value}
  onchange={(next) => onchange(next as T)}
  class="sort-control is-{variant}"
  align={variant === 'addon' ? 'left' : 'right'}
>
  {#snippet trigger({ toggle, open })}
    {#if variant === 'addon'}
      <button
        type="button"
        use:ripple
        class="sort-control__addon"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={label}
        use:tooltip={label}
        onclick={toggle}
      >
        <IconArrowSort />
      </button>
    {:else}
      <Button
        variant="ghost"
        class="btn-icon"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={label}
        title={label}
        onclick={toggle}
      >
        <IconArrowSort />
      </Button>
    {/if}
  {/snippet}
</Select>

<style>
  :global(.sort-control) {
    flex: none;
  }

  :global(.sort-control.is-addon),
  :global(.sort-control.is-addon .select-custom-trigger) {
    height: 100%;
  }

  .sort-control__addon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--choice-addon-width, 40px);
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    position: relative;
    overflow: hidden;
  }

  @media (hover: hover) {
    .sort-control__addon:hover {
      background: var(--state-layer-hover);
    }
  }

  .sort-control__addon:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .sort-control__addon :global(svg) {
    width: var(--control-icon-size, 18px);
    height: var(--control-icon-size, 18px);
  }
</style>
