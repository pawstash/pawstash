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
    class?: string;
    onsearch?: (query: string) => void;
    onclose?: () => void;
  }

  let {
    value = $bindable(''),
    open = $bindable(false),
    placeholder,
    expandable = true,
    class: extraClass = '',
    onsearch,
    onclose
  }: Props = $props();

  let inputEl: HTMLInputElement | undefined = $state();

  const effectivePlaceholder = $derived(
    placeholder || i18n.t('feed.search_placeholder') || 'Search...'
  );

  export async function openSearch() {
    open = true;
    await tick();
    inputEl?.focus();
  }

  export function closeSearch() {
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
        aria-label={value ? 'Clear search' : 'Close search'}
        use:tooltip={value ? (i18n.t('feed.clear') || 'Clear') : (i18n.t('nav.close') || 'Close')}
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
    />
    {#if value}
      <button
        type="button"
        class="search-action-btn"
        onclick={handleClearOrClose}
        aria-label="Clear search"
      >
        <IconDismiss class="w-5 h-5" />
      </button>
    {/if}
  </div>
{/if}

<style>
  .search-bar-expanded,
  .search-bar-fixed {
    position: relative;
    display: flex;
    align-items: center;
    width: 240px;
    height: 44px;
    flex-shrink: 0;
    animation: searchDesktopExpand var(--duration-fast, 180ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1));
  }

  .search-bar-expanded.is-mobile,
  .search-bar-fixed.is-mobile {
    width: 100%;
    flex: 1;
    animation: searchMobileExpand 0.18s var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1));
  }

  @keyframes searchDesktopExpand {
    from {
      opacity: 0;
      width: 44px;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      width: 240px;
      transform: scale(1);
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
    height: 44px;
    padding: 0 44px 0 40px;
    background: var(--bg-card);
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-full);
    color: var(--text-primary);
    font-size: 14px;
    font-family: var(--font-sans);
    outline: none;
    box-sizing: border-box;
    transition: background var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo),
                box-shadow var(--duration-fast) var(--ease-expo);
  }

  .search-input-field::placeholder {
    color: var(--text-muted);
    opacity: 0.6;
  }

  .search-input-field:hover {
    background: var(--bg-card-hover);
    border-color: var(--border-color-hover);
  }

  .search-input-field:focus {
    background: var(--bg-card-hover);
    border-color: var(--border-color-focus);
    box-shadow: 0 0 0 2px var(--accent-glow);
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

  .search-action-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: var(--text-primary);
  }
</style>
