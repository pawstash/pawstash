<script lang="ts">
  import { ripple } from '$lib/motion';
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
    maxVisible?: number;
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
    maxVisible = 0,
    size = 'md',
    showAll = false,
    allLabel = undefined,
    allCount = undefined,
    expandable = true,
    class: extraClass = ''
  }: Props = $props();

  let expanded = $state(false);

  let effectiveAllLabel = $derived(allLabel ?? (i18n.t('common.all') || 'All'));
  let expandBtnLabel = $derived(i18n.t('common.expand') || 'Expand');
  let collapseBtnLabel = $derived(i18n.t('common.collapse') || 'Less');

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

  let displayTags = $derived(normalizedTags);

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
    const _ = displayTags;
    const _expanded = expanded;
    if (_expanded) return;
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

  let needsTwoRows = $derived(!expanded && (displayTags.length > 3 || (showAll && displayTags.length > 2)));

  let row1Tags = $derived.by<NormalizedTag[]>(() => {
    if (expanded) return [];
    if (!needsTwoRows) return displayTags;
    const list: NormalizedTag[] = [];
    for (let i = 0; i < displayTags.length; i++) {
      if (showAll) {
        if (i % 2 === 1) list.push(displayTags[i]);
      } else {
        if (i % 2 === 0) list.push(displayTags[i]);
      }
    }
    return list;
  });

  let row2Tags = $derived.by<NormalizedTag[]>(() => {
    if (expanded || !needsTwoRows) return [];
    const list: NormalizedTag[] = [];
    for (let i = 0; i < displayTags.length; i++) {
      if (showAll) {
        if (i % 2 === 0) list.push(displayTags[i]);
      } else {
        if (i % 2 === 1) list.push(displayTags[i]);
      }
    }
    return list;
  });
</script>

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
      title={`#${tag.name}${tag.count !== undefined ? ` (${tag.count})` : ''}`}
    >
      <span class="tag-hash">#</span>
      <span class="tag-label">
        <StableWeightLabel text={tag.name} reserveWeight="var(--font-weight-semibold)" />
      </span>
      {#if tag.count !== undefined}
        <span class="tag-count-badge">{tag.count}</span>
      {/if}
    </button>
  {:else}
    <span class="tag-chip" class:is-active={isActive}>
      <span class="tag-hash">#</span>
      <span class="tag-label">
        <StableWeightLabel text={tag.name} reserveWeight="var(--font-weight-semibold)" />
      </span>
      {#if tag.count !== undefined}
        <span class="tag-count-badge">{tag.count}</span>
      {/if}
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
      title={effectiveAllLabel}
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
      <div class="tag-scroll-container" bind:this={scrollEl} class:has-overflow={hasOverflow}>
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
            class="tag-chip tag-more-btn is-fixed-right"
            onclick={() => expanded = true}
            title={expandBtnLabel}
            use:ripple
          >
            <span>{expandBtnLabel}</span>
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
          title={collapseBtnLabel}
          use:ripple
        >
          <span>{collapseBtnLabel}</span>
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

  .tag-scroll-container.has-overflow {
    mask-image: linear-gradient(to right, black calc(100% - 24px), transparent 100%);
    -webkit-mask-image: linear-gradient(to right, black calc(100% - 24px), transparent 100%);
  }

  .tag-scroll-container::-webkit-scrollbar {
    display: none;
  }

  .tag-rows-wrapper {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: max-content;
    padding: 2px 24px 2px 0;
  }

  .size-sm .tag-rows-wrapper {
    gap: 5px;
  }

  .tag-flex-row {
    display: flex;
    flex-direction: row;
    flex-wrap: nowrap;
    align-items: center;
    gap: 6px;
    width: max-content;
  }

  .size-sm .tag-flex-row {
    gap: 5px;
  }

  .tag-fixed-right-action {
    flex-shrink: 0;
    margin-left: 6px;
    display: flex;
    align-items: center;
    z-index: 2;
  }

  .tag-wrap-container {
    display: block;
    width: 100%;
    box-sizing: border-box;
    padding: 2px 0;
  }

  .tag-flex-wrap {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    width: 100%;
  }

  .size-sm .tag-flex-wrap {
    gap: 5px;
  }

  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 30px;
    padding: 0 11px;
    border-radius: var(--radius-full, 9999px);
    background: var(--bg-card, rgba(255, 255, 255, 0.05));
    border: none;
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    color: var(--text-secondary);
    font-size: 13px;
    font-family: var(--font-sans);
    font-weight: 500;
    line-height: normal;
    white-space: nowrap;
    user-select: none;
    box-sizing: border-box;
    flex-shrink: 0;
    transition: background var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo),
                box-shadow var(--duration-fast) var(--ease-expo),
                transform var(--duration-fast) var(--ease-expo);
  }

  .size-sm .tag-chip {
    height: 25px;
    padding: 0 8px;
    font-size: 12px;
    gap: 2px;
  }

  .size-lg .tag-chip {
    height: 36px;
    padding: 0 14px;
    font-size: 14px;
    gap: 4px;
  }

  .tag-chip.is-clickable {
    cursor: pointer;
    outline: none;
  }

  .tag-chip.is-clickable:hover {
    background: var(--bg-card-hover, rgba(255, 255, 255, 0.1));
    color: var(--text-primary);
  }

  .tag-chip.is-clickable:hover .tag-hash {
    opacity: 0.95;
    color: var(--accent-primary);
  }

  .tag-chip.is-clickable:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: 1px;
  }

  .tag-chip.is-active {
    background: var(--accent-primary) !important;
    color: var(--accent-on-primary, #ffffff) !important;
    font-weight: 600;
  }

  .tag-chip.is-active .tag-hash {
    color: var(--accent-on-primary, #ffffff) !important;
    opacity: 1 !important;
  }

  .tag-chip.is-active .tag-count-badge {
    background: rgba(0, 0, 0, 0.25) !important;
    color: var(--accent-on-primary, #ffffff) !important;
    font-weight: 600;
  }

  .tag-hash {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent-primary);
    opacity: 0.65;
    margin-right: 1px;
    transition: opacity var(--duration-fast) var(--ease-expo), color var(--duration-fast) var(--ease-expo);
  }

  .size-sm .tag-hash {
    font-size: 11px;
  }

  .size-lg .tag-hash {
    font-size: 13.5px;
  }

  .tag-count-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 500;
    line-height: 1;
    padding: 2px 5px;
    margin-left: 3px;
    border-radius: var(--radius-full, 9999px);
    background: rgba(255, 255, 255, 0.07);
    color: var(--text-muted);
    min-width: 14px;
    transition: background var(--duration-fast) var(--ease-expo), color var(--duration-fast) var(--ease-expo);
  }

  .size-sm .tag-count-badge {
    font-size: 10px;
    padding: 1px 4px;
    margin-left: 2px;
  }

  .size-lg .tag-count-badge {
    font-size: 12px;
    padding: 2px 6px;
    margin-left: 4px;
  }

  .chevron-icon {
    width: 14px;
    height: 14px;
  }

  :global(.size-sm .chevron-icon) {
    width: 12px;
    height: 12px;
  }

  .tag-more-btn {
    cursor: pointer;
    background: var(--bg-card, rgba(255, 255, 255, 0.05));
    border: none;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    gap: 3px;
    box-shadow: none !important;
  }

  .size-sm .tag-more-btn {
    font-size: 12px;
  }

  .size-lg .tag-more-btn {
    font-size: 14px;
  }

  .tag-more-btn:hover {
    background: var(--bg-card-hover, rgba(255, 255, 255, 0.1));
    color: var(--text-primary);
  }

  .tag-more-btn.is-fixed-right {
    box-shadow: none !important;
  }
</style>
