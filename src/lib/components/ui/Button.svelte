<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import { ripple, tooltip } from '$lib/motion';

  interface Props extends HTMLButtonAttributes {
    type?: 'button' | 'submit' | 'reset';
    variant?: 'primary' | 'accent' | 'ghost' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
    onclick?: (e: MouseEvent) => void;
    children?: Snippet;
    class?: string;
    ref?: HTMLButtonElement;
    title?: string;
    tooltip?: string;
  }

  let {
    type = 'button',
    variant = 'primary',
    size = 'md',
    disabled = false,
    onclick,
    children,
    class: extraClass = '',
    ref = $bindable(),
    title,
    tooltip: tooltipProp,
    ...restProps
  }: Props = $props();

  let effectiveTooltip = $derived(tooltipProp ?? title);
</script>

<button
  bind:this={ref}
  {type}
  {disabled}
  {onclick}
  use:ripple
  use:tooltip={effectiveTooltip}
  aria-label={restProps['aria-label'] || effectiveTooltip}
  class="btn btn-{variant} btn-{size} {extraClass}"
  {...restProps}
>
  {#if children}
    {@render children()}
  {/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-sans);
    font-weight: 500;
    cursor: pointer;
    border-radius: var(--radius-full);
    transition: all var(--duration-fast) var(--ease-expo);
    border: var(--border-width) solid transparent;
    outline: none;
    box-sizing: border-box;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .btn:active {
    transform: scale(0.98);
  }

  .btn:disabled {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
    pointer-events: none;
  }

  .btn-primary {
    background: var(--bg-card);
    border-color: var(--border-color);
    color: var(--text-primary);
  }

  .btn-primary:hover {
    background: var(--bg-card-hover);
    border-color: var(--border-color-hover);
  }

  .btn-accent {
    background: color-mix(in srgb, var(--accent-primary) 90%, transparent);
    border-color: transparent;
    color: var(--text-on-accent, var(--text-primary));
  }

  .btn-accent:hover {
    background: var(--accent-primary);
    border-color: transparent;
    color: var(--text-on-accent, var(--text-primary));
  }

  .btn-ghost {
    background: transparent;
    color: color-mix(in srgb, var(--text-primary) 70%, var(--text-secondary));
  }

  .btn-ghost:hover {
    background: var(--bg-card-hover);
    color: var(--text-primary);
  }

  .btn-danger {
    background: rgba(224, 60, 60, 0.1);
    border-color: rgba(224, 60, 60, 0.2);
    color: #f87171;
  }

  .btn-danger:hover {
    background: rgba(224, 60, 60, 0.2);
    border-color: rgba(224, 60, 60, 0.3);
  }

  .btn-sm {
    height: 34px;
    padding: 0 14px;
    font-size: 12px;
    gap: 6px;
  }

  .btn-sm :global(svg) {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  .btn-md {
    height: 46px;
    padding: 0 20px;
    font-size: 14px;
    gap: 8px;
  }

  .btn-md :global(svg) {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }

  .btn-lg {
    height: 52px;
    padding: 0 24px;
    font-size: 16px;
    gap: 10px;
  }

  .btn-lg :global(svg) {
    width: 24px;
    height: 24px;
    flex-shrink: 0;
  }
</style>
