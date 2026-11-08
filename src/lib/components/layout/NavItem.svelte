<script lang="ts">
  import type { Component } from 'svelte';
  import { ripple, tooltip } from '$lib/motion';
  import ProgressRing from '$lib/components/ui/ProgressRing.svelte';

  interface Props {
    label: string;
    subtitle?: string;
    icon: Component;
    iconActive?: Component;
    active?: boolean;
    compact?: boolean;
    badge?: number;
    progress?: number | null;
    progressVariant?: 'standard' | 'segmented';
    onclick: () => void;
  }

  let {
    label,
    subtitle,
    icon: Icon,
    iconActive: IconActive,
    active = false,
    compact = false,
    badge,
    progress,
    progressVariant = 'standard',
    onclick
  }: Props = $props();

  let hasProgress = $derived(progress !== undefined);
</script>

<button
  type="button"
  class="nav-item"
  class:is-active={active}
  class:is-compact={compact}
  class:has-subtitle={Boolean(subtitle) && !compact}
  aria-current={active ? 'page' : undefined}
  aria-label={compact ? label : undefined}
  use:ripple
  use:tooltip={compact ? { text: subtitle ? `${label} · ${subtitle}` : label, placement: 'right' } : undefined}
  {onclick}
>
  <span class="nav-item__icon">
    {#if hasProgress}
      <ProgressRing value={progress ?? null} variant={progressVariant} size={30} />
    {/if}
    {#if active && IconActive}<IconActive />{:else}<Icon />{/if}
    {#if compact && badge}
      <span class="nav-item__dot" aria-hidden="true"></span>
    {/if}
  </span>
  {#if !compact}
    <span class="nav-item__text">
      <span class="nav-item__label">{label}</span>
      {#if subtitle}<span class="nav-item__subtitle">{subtitle}</span>{/if}
    </span>
    {#if badge}
      <span class="nav-item__badge">{badge}</span>
    {/if}
  {/if}
</button>

<style>
  .nav-item {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 11px;
    width: max-content;
    max-width: 100%;
    height: 44px;
    padding: 0 20px 0 11px;
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--text-secondary);
    text-align: start;
    overflow: hidden;
    box-sizing: border-box;
    flex-shrink: 0;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
  }

  .nav-item.has-subtitle {
    height: 52px;
  }

  .nav-item.is-compact {
    width: 44px;
    justify-content: center;
    padding: 0;
  }

  @media (hover: hover) {
    .nav-item:not(.is-active):hover {
      background: var(--state-layer-hover);
      color: var(--text-primary);
    }
  }

  .nav-item:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .nav-item.is-active {
    background: var(--state-layer-pressed);
    color: var(--text-primary);
  }

  .nav-item__icon {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 22px;
    height: 22px;
    opacity: 0.65;
    transition: opacity 200ms ease;
  }

  .nav-item:hover .nav-item__icon,
  .nav-item.is-active .nav-item__icon {
    opacity: 1;
  }

  .nav-item__icon > :global(svg) {
    width: 22px;
    height: 22px;
  }

  .nav-item__icon :global(.progress-ring) {
    position: absolute;
    inset: -4px;
  }

  .nav-item__dot {
    position: absolute;
    top: -1px;
    right: -2px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-primary);
  }

  .nav-item__text {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    min-width: 0;
  }

  .nav-item__label,
  .nav-item__subtitle {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .nav-item__label {
    font-size: 14px;
    line-height: normal;
    font-weight: 475;
    letter-spacing: 0.015em;
    opacity: 0.65;
    transition: opacity 200ms ease;
  }

  .nav-item:hover .nav-item__label,
  .nav-item.is-active .nav-item__label {
    opacity: 1;
  }

  .nav-item.is-active .nav-item__label {
    font-weight: 500;
  }

  .nav-item__subtitle {
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
  }

  .nav-item.is-active .nav-item__subtitle {
    color: inherit;
  }

  .nav-item__badge {
    flex: none;
    color: var(--text-secondary);
    font-size: var(--type-label-md-size);
    font-variant-numeric: tabular-nums;
  }

  .nav-item.is-active .nav-item__badge {
    color: inherit;
  }
</style>
