<script lang="ts">
  import type { Snippet } from 'svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import SearchBar from '$lib/components/ui/SearchBar.svelte';

  interface Props {
    searchOpen?: boolean;
    searchQuery?: string;
    searchPlaceholder?: string;
    showSearchButton?: boolean;
    onsearchtoggle?: (open: boolean) => void;
    children?: Snippet;
    class?: string;
  }

  let {
    searchOpen = $bindable(false),
    searchQuery = $bindable(''),
    searchPlaceholder,
    showSearchButton = true,
    onsearchtoggle,
    children,
    class: extraClass = ''
  }: Props = $props();
</script>

<div
  class="header-actions-root {extraClass}"
  class:is-mobile={layoutState.isMobile}
  class:search-active={searchOpen}
>
  {#if showSearchButton}
    <SearchBar
      bind:value={searchQuery}
      bind:open={searchOpen}
      placeholder={searchPlaceholder}
      onclose={() => onsearchtoggle?.(false)}
    />
  {/if}

  {#if !layoutState.isMobile || !searchOpen}
    <div class="action-buttons-cluster">
      {#if children}
        {@render children()}
      {/if}
    </div>
  {/if}
</div>

<style>
  .header-actions-root {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .header-actions-root.is-mobile.search-active {
    width: 100%;
    flex: 1;
    display: flex;
    align-items: center;
    background: transparent;
  }

  .action-buttons-cluster {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
