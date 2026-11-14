<script lang="ts">
  import { edgeFade } from '$lib/actions/edgeFade';
  import { ripple, tooltip } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import IconChevronDown from '~icons/fluent/chevron-down-20-regular';
  import IconChevronUp from '~icons/fluent/chevron-up-20-regular';
  import StableWeightLabel from './StableWeightLabel.svelte';

  export interface TagItem {
    name: string;
    count?: number;
  }

  interface Props {
    tags: Array<string | TagItem>;
    activeTag?: string | null;
    onclick?: (tag: string) => void;
    onclear?: () => void;
    size?: 'sm' | 'md' | 'lg';
    showAll?: boolean;
    allLabel?: string;
    allCount?: number;
    expandable?: boolean;
    class?: string;
  }

  let {
    tags = [],
    activeTag = null,
    onclick: handleClick,
    onclear: handleClear,
    size = 'md',
    showAll = false,
    allLabel = undefined,
    allCount = undefined,
    expandable = true,
    class: extraClass = ''
  }: Props = $props();

  let expanded = $state(false);

  let effectiveAllLabel = $derived(allLabel ?? i18n.t('common.all'));
  let expandBtnLabel = $derived(i18n.t('common.expand'));
  let collapseBtnLabel = $derived(i18n.t('common.collapse'));

  interface NormalizedTag {
    name: string;
    count?: number;
    key: string;
  }

  let normalizedTags = $derived.by<NormalizedTag[]>(() => {
    const seen = new Set<string>();
    const result: NormalizedTag[] = [];

    for (const item of tags) {
      const name = typeof item === 'string' ? item.replace(/^#+/, '').trim() : item.name.replace(/^#+/, '').trim();
      const count = typeof item === 'object' && item !== null ? item.count : undefined;
      if (!name) continue;
      const key = name.toLowerCase();
      if (seen.has(key)) continue;
      seen.add(key);
      result.push({ name, count, key });
    }
    return result;
  });

  let scrollEl = $state<HTMLDivElement | null>(null);
  let hasOverflow = $state(false);

  function checkOverflow() {
    if (!scrollEl) {
      hasOverflow = false;
      return;
    }
    hasOverflow = scrollEl.scrollWidth > scrollEl.clientWidth + 4;
  }

  $effect(() => {
    void normalizedTags;
    if (expanded) return;
    const rafId = requestAnimationFrame(checkOverflow);
    return () => cancelAnimationFrame(rafId);
  });

  $effect(() => {
    if (!scrollEl) return;
    const ro = new ResizeObserver(() => {
      checkOverflow();
    });
    ro.observe(scrollEl);
    return () => ro.disconnect();
  });

  let canExpand = $derived(expandable && hasOverflow);

  let isAllActive = $derived(!activeTag);

  function isTagActive(tag: NormalizedTag): boolean {
    if (!activeTag) return false;
    const cleanActive = activeTag.trim().toLowerCase().replace(/^#+/, '');
    return cleanActive === tag.key || cleanActive.replace(/_/g, ' ') === tag.key.replace(/_/g, ' ');
  }

  let needsTwoRows = $derived(!expanded && (normalizedTags.length > 3 || (showAll && normalizedTags.length > 2)));
  let firstRowParity = $derived(showAll ? 1 : 0);

  let row1Tags = $derived.by<NormalizedTag[]>(() => {
    if (expanded) return [];
    if (!needsTwoRows) return normalizedTags;
    return normalizedTags.filter((_, i) => i % 2 === firstRowParity);
  });

  let row2Tags = $derived.by<NormalizedTag[]>(() => {
    if (expanded || !needsTwoRows) return [];
    return normalizedTags.filter((_, i) => i % 2 !== firstRowParity);
  });
</script>

{#snippet tagBody(tag: NormalizedTag)}
  <span class="tag-hash">#</span>
  <span class="tag-label">
    <StableWeightLabel text={tag.name} reserveWeight="var(--font-weight-semibold)" />
  </span>
  {#if tag.count !== undefined}
    <span class="tag-count-badge">{tag.count}</span>
  {/if}
{/snippet}

{#snippet renderTag(tag: NormalizedTag)}
  {@const isActive = isTagActive(tag)}
  {#if handleClick}
    <button
      type="button"
      class="tag-chip is-clickable"
      class:is-active={isActive}
      use:ripple
      onclick={() => {
        if (isActive && handleClear) {
          handleClear();
        } else {
          handleClick(tag.name);
        }
      }}
      use:tooltip={`#${tag.name}${tag.count !== undefined ? ` (${tag.count})` : ''}`}
    >
      {@render tagBody(tag)}
    </button>
  {:else}
    <span class="tag-chip" class:is-active={isActive}>
      {@render tagBody(tag)}
    </span>
  {/if}
{/snippet}

{#snippet renderAllButton()}
  {#if showAll}
    <button
      type="button"
      class="tag-chip tag-all-chip is-clickable"
      class:is-active={isAllActive}
      use:ripple
      onclick={() => {
        if (handleClear) {
          handleClear();
        } else if (handleClick) {
          handleClick('');
        }
      }}
      use:tooltip={effectiveAllLabel}
    >
      <span class="tag-label">
        <StableWeightLabel text={effectiveAllLabel} reserveWeight="var(--font-weight-semibold)" />
      </span>
      {#if allCount !== undefined}
        <span class="tag-count-badge">{allCount}</span>
      {/if}
    </button>
  {/if}
{/snippet}

{#if normalizedTags.length > 0 || showAll}
  {#if !expanded}
    <div class="tag-bar-shell size-{size} {extraClass}">
      <div class="tag-scroll-container" bind:this={scrollEl} use:edgeFade>
        <div class="tag-rows-wrapper">
          <div class="tag-flex-row">
            {@render renderAllButton()}
            {#each row1Tags as tag (tag.key)}
              {@render renderTag(tag)}
            {/each}
          </div>

          {#if needsTwoRows && row2Tags.length > 0}
            <div class="tag-flex-row">
              {#each row2Tags as tag (tag.key)}
                {@render renderTag(tag)}
              {/each}
            </div>
          {/if}
        </div>
      </div>

      {#if canExpand}
        <div class="tag-fixed-right-action">
          <button
            type="button"
            class="tag-chip tag-more-btn"
            onclick={() => expanded = true}
            aria-label={expandBtnLabel}
            use:tooltip={expandBtnLabel}
            use:ripple
          >
            <IconChevronDown class="chevron-icon" />
          </button>
        </div>
      {/if}
    </div>
  {:else}
    <div class="tag-wrap-container size-{size} {extraClass}">
      <div class="tag-flex-wrap">
        {@render renderAllButton()}
        {#each normalizedTags as tag (tag.key)}
          {@render renderTag(tag)}
        {/each}
        <button
          type="button"
          class="tag-chip tag-more-btn"
          onclick={() => expanded = false}
          aria-label={collapseBtnLabel}
          use:tooltip={collapseBtnLabel}
          use:ripple
        >
          <IconChevronUp class="chevron-icon" />
        </button>
      </div>
    </div>
  {/if}
{/if}

<style>
  .tag-bar-shell {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
  }

  .tag-scroll-container {
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    -ms-overflow-style: none;
    -webkit-overflow-scrolling: touch;
  }

  .tag-scroll-container::-webkit-scrollbar {
    display: none;
  }

  .tag-bar-shell,
  .tag-wrap-container {
    --tag-chip-height: 32px;
    --tag-chip-px: 12px;
    --tag-gap: 8px;
    --tag-font: var(--type-label-lg-size);
  }

  .size-sm {
    --tag-chip-height: 28px;
    --tag-chip-px: 10px;
    --tag-gap: 6px;
    --tag-font: var(--type-label-md-size);
  }

  .size-lg {
    --tag-chip-height: 36px;
    --tag-chip-px: 14px;
  }

  .tag-rows-wrapper {
    display: flex;
    flex-direction: column;
    gap: var(--tag-gap);
    width: max-content;
    padding-inline-end: 24px;
  }

  .tag-flex-row,
  .tag-flex-wrap {
    display: flex;
    align-items: center;
    gap: var(--tag-gap);
  }

  .tag-flex-row {
    flex-wrap: nowrap;
    width: max-content;
  }

  .tag-flex-wrap {
    flex-wrap: wrap;
    width: 100%;
  }

  .tag-fixed-right-action {
    display: flex;
    flex-shrink: 0;
    margin-inline-start: var(--tag-gap);
  }

  .tag-wrap-container {
    display: block;
    width: 100%;
  }

  .tag-chip {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    gap: 2px;
    height: var(--tag-chip-height);
    padding: 0 var(--tag-chip-px);
    border: 0;
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--tag-font);
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
    user-select: none;
    box-sizing: border-box;
    transition: background-color var(--duration-fast) var(--ease-standard),
                color var(--duration-fast) var(--ease-standard);
  }

  .tag-chip.is-clickable {
    cursor: pointer;
    outline: none;
  }

  @media (hover: hover) {
    .tag-chip.is-clickable:hover {
      background: var(--surface-container-highest);
    }
  }

  .tag-chip.is-clickable:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: 2px;
  }

  .tag-chip.is-active {
    background: var(--accent-container);
    color: var(--accent-on-container);
  }

  .tag-hash {
    color: var(--text-secondary);
  }

  .tag-chip.is-active .tag-hash {
    color: inherit;
  }

  .tag-count-badge {
    margin-inline-start: 6px;
    color: var(--text-secondary);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }

  .tag-chip.is-active .tag-count-badge {
    color: inherit;
  }

  .tag-more-btn {
    justify-content: center;
    width: var(--tag-chip-height);
    padding: 0;
    color: var(--text-secondary);
  }

  .tag-more-btn :global(.chevron-icon) {
    width: 18px;
    height: 18px;
  }
</style>
