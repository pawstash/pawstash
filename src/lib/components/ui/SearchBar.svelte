<script lang="ts">
  import { tick } from 'svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import { tooltip } from '$lib/motion';
  import Button from '$lib/components/ui/Button.svelte';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';

  interface Props {
    value?: string;
    open?: boolean;
    placeholder?: string;
    expandable?: boolean;
    shortcutHint?: string;
    class?: string;
    onsearch?: (query: string) => void;
    onclose?: () => void;
  }

  let {
    value = $bindable(''),
    open = $bindable(false),
    placeholder,
    expandable = true,
    shortcutHint,
    class: extraClass = '',
    onsearch,
    onclose
  }: Props = $props();

  let inputEl: HTMLInputElement | undefined = $state();
  let opening = $state(false);

  const effectivePlaceholder = $derived(
    placeholder || i18n.t('feed.search_placeholder')
  );

  export async function openSearch() {
    opening = true;
    open = true;
    await tick();
    inputEl?.focus();
  }

  export function focusInput() {
    inputEl?.focus();
    inputEl?.select();
  }

  export function closeSearch() {
    opening = false;
    open = false;
    value = '';
    onsearch?.('');
    onclose?.();
  }

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    value = target.value;
    onsearch?.(value);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      closeSearch();
    }
  }

  function handleClearOrClose() {
    if (value) {
      value = '';
      onsearch?.('');
      inputEl?.focus();
    } else {
      closeSearch();
    }
  }
</script>

{#if expandable}
  {#if open}
    <div
      class="search-bar-expanded {extraClass}"
      class:is-mobile={layoutState.isMobile}
      class:is-opening={opening}
    >
      <span class="search-icon-wrapper">
        <IconSearch class="w-[18px] h-[18px]" />
      </span>
      <input
        bind:this={inputEl}
        type="text"
        {value}
        placeholder={effectivePlaceholder}
        oninput={handleInput}
        onkeydown={handleKeyDown}
        class="search-input-field"
      />
      <button
        type="button"
        class="search-action-btn"
        onclick={handleClearOrClose}
        aria-label={value ? i18n.t('common.clear_search') : i18n.t('common.close')}
        use:tooltip={value ? i18n.t('feed.clear') : i18n.t('nav.close')}
      >
        <IconDismiss class="w-5 h-5" />
      </button>
    </div>
  {:else}
    <Button
      variant="ghost"
      class="btn-icon {extraClass}"
      onclick={openSearch}
      title={effectivePlaceholder}
      aria-label={effectivePlaceholder}
    >
      <IconSearch class="w-5 h-5" />
    </Button>
  {/if}
{:else}
  <div class="search-bar-fixed {extraClass}" class:is-mobile={layoutState.isMobile}>
    <span class="search-icon-wrapper">
      <IconSearch class="w-[18px] h-[18px]" />
    </span>
    <input
      bind:this={inputEl}
      type="text"
      {value}
      placeholder={effectivePlaceholder}
      oninput={handleInput}
      onkeydown={handleKeyDown}
      class="search-input-field"
      class:has-kbd={!value && shortcutHint}
    />
    {#if value}
      <button
        type="button"
        class="search-action-btn"
        onclick={handleClearOrClose}
        aria-label={i18n.t('common.clear_search')}
      >
        <IconDismiss class="w-5 h-5" />
      </button>
    {:else if shortcutHint}
      <kbd class="search-kbd" aria-hidden="true">{shortcutHint}</kbd>
    {/if}
  </div>
{/if}

<style>
  .search-bar-expanded,
  .search-bar-fixed {
    position: relative;
    display: flex;
    align-items: center;
    width: 280px;
    height: var(--control-height, 40px);
    flex-shrink: 0;
  }

  .search-bar-expanded.is-mobile,
  .search-bar-fixed.is-mobile {
    width: 100%;
    flex: 1;
  }

  .search-bar-expanded.is-opening {
    animation: searchDesktopExpand var(--duration-fast, 180ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1));
  }

  .search-bar-expanded.is-mobile.is-opening {
    animation: searchMobileExpand 0.18s var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1));
  }

  @keyframes searchDesktopExpand {
    from {
      opacity: 0;
      width: var(--control-height, 40px);
    }
    to {
      opacity: 1;
      width: 280px;
    }
  }

  @keyframes searchMobileExpand {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .search-input-field {
    width: 100%;
    height: 100%;
    padding: 0 40px 0 42px;
    background: var(--content-surface-high);
    border: 0;
    border-radius: var(--radius-full);
    color: var(--text-primary);
    font-size: var(--type-body-md-size);
    font-family: var(--font-sans);
    line-height: normal;
    text-overflow: ellipsis;
    outline: none;
    box-sizing: border-box;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .search-input-field::placeholder {
    color: var(--text-secondary);
  }

  .search-input-field.has-kbd {
    padding-inline-end: 76px;
  }

  .search-input-field:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .search-icon-wrapper {
    position: absolute;
    left: 14px;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    pointer-events: none;
    z-index: 2;
    width: 18px;
    height: 18px;
  }

  .search-action-btn {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    border-radius: 50%;
    cursor: pointer;
    z-index: 2;
    padding: 0;
    transition: color var(--duration-fast), background var(--duration-fast);
  }

  .search-kbd {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    padding: 1px 6px;
    border-radius: var(--radius-xs);
    background: var(--surface-container-highest);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: var(--type-label-sm-size);
    line-height: var(--type-label-sm-line);
    pointer-events: none;
  }

  @media (hover: hover) {
    .search-action-btn:hover {
      background: var(--state-layer-hover);
      color: var(--text-primary);
    }
  }
</style>
