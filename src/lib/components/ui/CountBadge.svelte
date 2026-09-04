<script lang="ts">
  interface Props {
    count?: number | string | null;
    variant?: 'tab' | 'header' | 'muted' | 'pill';
    showZero?: boolean;
    max?: number;
    class?: string;
  }

  let {
    count = 0,
    variant = 'tab',
    showZero = false,
    max,
    class: extraClass = ''
  }: Props = $props();

  let numericCount = $derived(
    typeof count === 'number'
      ? count
      : (typeof count === 'string' && count.trim() !== '' && !isNaN(Number(count))
        ? Number(count)
        : null)
  );

  let shouldRender = $derived.by(() => {
    if (count === null || count === undefined || count === '') return false;
    if (numericCount !== null) {
      if (numericCount === 0 && !showZero) return false;
      return true;
    }
    return true;
  });

  let displayValue = $derived.by(() => {
    if (numericCount !== null) {
      if (max !== undefined && numericCount > max) {
        return `${max.toLocaleString()}+`;
      }
      return numericCount.toLocaleString();
    }
    return String(count);
  });
</script>

{#if shouldRender}
  <span class="count-badge variant-{variant} {extraClass}">
    {displayValue}
  </span>
{/if}

<style>
  .count-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
    line-height: 1;
    letter-spacing: -0.01em;
    user-select: none;
    pointer-events: none;
    box-sizing: border-box;
    transition: opacity var(--duration-fast) var(--ease-expo), background var(--duration-fast) var(--ease-expo), color var(--duration-fast) var(--ease-expo);
  }

  .variant-tab,
  .variant-pill {
    padding: 1px 6.5px 2px 6.5px;
    border-radius: var(--radius-full, 9999px);
    background: rgba(255, 255, 255, 0.12);
    color: inherit;
    font-size: 11px;
    font-weight: 600;
  }

  /* When inside an active accent button, boost background contrast */
  :global(.btn-accent) .variant-tab,
  :global(.btn-accent) .variant-pill,
  :global(.btn.btn-accent) .variant-tab,
  :global(.btn.btn-accent) .variant-pill,
  :global(.btn.variant-accent) .variant-tab,
  :global(.btn.variant-accent) .variant-pill {
    background: rgba(255, 255, 255, 0.22);
    color: inherit;
  }

  .variant-header {
    font-size: 13.5px;
    font-weight: 500;
    opacity: 0.55;
  }

  .variant-muted {
    font-size: 12px;
    font-weight: 500;
    opacity: 0.45;
  }
</style>
