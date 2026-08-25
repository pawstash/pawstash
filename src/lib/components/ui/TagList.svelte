<script lang="ts">
  import { ripple } from '$lib/motion';
  import IconChevronDown from '~icons/fluent/chevron-down-20-regular';
  import IconChevronUp from '~icons/fluent/chevron-up-20-regular';

  interface Props {
    tags: string[];
    activeTag?: string | null;
    onclick?: (tag: string) => void;
    maxVisible?: number;
    size?: 'sm' | 'md' | 'lg';
    class?: string;
  }

  let {
    tags = [],
    activeTag = null,
    onclick: handleClick,
    maxVisible = 0,
    size = 'md',
    class: extraClass = ''
  }: Props = $props();

  let expanded = $state(false);

  let cleanTags = $derived(
    Array.from(new Set(tags.map((t) => t.trim()).filter(Boolean)))
  );

  let visibleTags = $derived.by(() => {
    if (maxVisible > 0 && !expanded && cleanTags.length > maxVisible) {
      return cleanTags.slice(0, maxVisible);
    }
    return cleanTags;
  });

  let hiddenCount = $derived(
    maxVisible > 0 && !expanded && cleanTags.length > maxVisible
      ? cleanTags.length - maxVisible
      : 0
  );
</script>

{#if cleanTags.length > 0}
  <div class="tag-chips-wrap size-{size} {extraClass}">
    {#each visibleTags as tag}
      {@const isActive = activeTag === tag}
      {#if handleClick}
        <button
          type="button"
          class="tag-chip is-clickable"
          class:is-active={isActive}
          use:ripple
          onclick={() => handleClick(tag)}
          title={`#${tag}`}
        >
          <span class="tag-hash">#</span>
          <span class="tag-label">{tag}</span>
        </button>
      {:else}
        <span class="tag-chip" class:is-active={isActive}>
          <span class="tag-hash">#</span>
          <span class="tag-label">{tag}</span>
        </span>
      {/if}
    {/each}

    {#if maxVisible > 0 && cleanTags.length > maxVisible}
      <button
        type="button"
        class="tag-chip tag-more-btn"
        onclick={() => expanded = !expanded}
      >
        {#if expanded}
          <span>Less</span>
          <IconChevronUp class="chevron-icon" />
        {:else}
          <span>+{hiddenCount}</span>
          <IconChevronDown class="chevron-icon" />
        {/if}
      </button>
    {/if}
  </div>
{/if}

<style>
  .tag-chips-wrap {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 8px;
    width: 100%;
  }

  .tag-chips-wrap.size-sm {
    gap: 4px 6px;
  }

  .tag-chips-wrap.size-lg {
    gap: 8px 10px;
  }

  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 28px;
    padding: 0 10px;
    border-radius: var(--radius-md, 8px);
    background: transparent;
    border: var(--border-width, 1px) solid transparent;
    color: var(--text-secondary);
    font-size: 13px;
    font-family: var(--font-sans);
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
    user-select: none;
    box-sizing: border-box;
    transition: background var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo),
                color var(--duration-fast) var(--ease-expo);
  }

  .size-sm .tag-chip {
    height: 22px;
    padding: 0 7px;
    font-size: 12px;
    border-radius: var(--radius-sm, 6px);
  }

  .size-lg .tag-chip {
    height: 32px;
    padding: 0 12px;
    font-size: 14px;
    border-radius: var(--radius-md, 8px);
  }

  .tag-chip.is-clickable {
    cursor: pointer;
    outline: none;
  }

  .tag-chip.is-clickable:hover {
    background: var(--bg-card-hover);
    color: var(--text-primary);
  }

  .tag-chip.is-clickable:hover .tag-hash {
    opacity: 0.9;
    color: var(--accent-primary);
  }

  .tag-chip.is-clickable:focus-visible {
    border-color: var(--border-color-focus, var(--accent-primary));
    box-shadow: 0 0 0 2px var(--accent-glow, rgba(255, 255, 255, 0.15));
  }

  .tag-chip.is-active {
    background: color-mix(in srgb, var(--accent-primary) 14%, transparent);
    border-color: color-mix(in srgb, var(--accent-primary) 28%, transparent);
    color: var(--accent-primary);
    font-weight: 600;
  }

  .tag-chip.is-active .tag-hash {
    color: var(--accent-primary);
    opacity: 1;
  }

  .tag-hash {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--accent-primary);
    opacity: 0.55;
    margin-right: 1px;
    transition: opacity var(--duration-fast) var(--ease-expo), color var(--duration-fast) var(--ease-expo);
  }

  .size-sm .tag-hash {
    font-size: 11px;
  }

  .size-lg .tag-hash {
    font-size: 13.5px;
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
    background: transparent;
    border-color: transparent;
    color: var(--text-muted);
    font-size: 12px;
    gap: 3px;
  }

  .tag-more-btn:hover {
    background: var(--bg-card-hover);
    color: var(--text-secondary);
  }
</style>
