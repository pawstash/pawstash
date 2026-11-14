<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { computePosition, autoUpdate, flip, shift, offset, size } from '@floating-ui/dom';
  import { FLOATING_EDGE, FLOATING_OFFSET } from '$lib/utils/floating';
  import { portal } from '$lib/actions/portal';
  import { scrollable } from '$lib/actions/scrollable';
  import Button from '$lib/components/ui/Button.svelte';
  import IconFilter from '~icons/fluent/filter-24-regular';

  interface Props {
    open?: boolean;
    active?: boolean;
    badge?: number | string;
    title?: string;
    icon?: Component;
    triggerVariant?: 'ghost' | 'accent' | 'primary';
    align?: 'right' | 'left';
    width?: string;
    class?: string;
    menuClass?: string;
    keepOpen?: boolean;
    anchorEl?: HTMLElement | null;
    trigger?: Snippet<[{ toggle: (e: MouseEvent) => void; open: boolean; active: boolean }]>;
    children?: Snippet<[{ close: () => void }]>;
  }

  let {
    open = $bindable(false),
    active = false,
    badge,
    title,
    icon,
    triggerVariant,
    align = 'right',
    width = 'min(330px, calc(100vw - 32px))',
    class: extraClass = '',
    menuClass = '',
    keepOpen = false,
    anchorEl,
    trigger,
    children
  }: Props = $props();

  let rootEl = $state<HTMLDivElement>();
  let dropdownEl = $state<HTMLDivElement | null>(null);
  let cleanupAutoUpdate: (() => void) | null = null;
  let wasOpen = false;

  $effect(() => {
    if (open) {
      wasOpen = true;
      return;
    }
    if (!wasOpen) return;
    wasOpen = false;
    const trigger = rootEl?.querySelector<HTMLElement>('button');
    if (trigger?.isConnected) trigger.focus();
  });

  function toggle(event: MouseEvent) {
    event.stopPropagation();
    open = !open;
  }

  function close() {
    open = false;
  }

  async function updatePosition() {
    const referenceEl = anchorEl ?? rootEl;
    if (!referenceEl || !dropdownEl) return;

    const placement = align === 'left' ? 'bottom-start' : 'bottom-end';

    const { x, y } = await computePosition(referenceEl, dropdownEl, {
      placement,
      strategy: 'fixed',
      middleware: [
        offset(FLOATING_OFFSET),
        flip({
          fallbackPlacements: ['top-end', 'top-start', 'bottom', 'top'],
          padding: FLOATING_EDGE
        }),
        shift({
          padding: FLOATING_EDGE
        }),
        size({
          padding: FLOATING_EDGE,
          apply({ availableHeight, elements }) {
            Object.assign(elements.floating.style, {
              maxHeight: `${Math.min(availableHeight, 600)}px`
            });
          }
        })
      ]
    });

    if (!dropdownEl) return;
    dropdownEl.style.left = `${x}px`;
    dropdownEl.style.top = `${y}px`;
    dropdownEl.style.visibility = 'visible';
  }

  $effect(() => {
    const referenceEl = anchorEl ?? rootEl;
    if (open && referenceEl && dropdownEl) {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = autoUpdate(referenceEl, dropdownEl, () => {
        void updatePosition();
      });
    } else {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = null;
    }

    return () => {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = null;
    };
  });

  function handleWindowPointerDown(event: PointerEvent) {
    if (!open) return;
    const target = event.target as HTMLElement;
    if (target.closest('.select-dropdown-portal') || target.closest('[data-portal-keep-open]')) return;
    const referenceEl = anchorEl ?? rootEl;
    if (referenceEl?.contains(target) || rootEl?.contains(target) || dropdownEl?.contains(target)) return;
    open = false;
  }

  function handleWindowKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape' && open) {
      open = false;
    }
  }
</script>

<svelte:window onpointerdown={handleWindowPointerDown} onkeydown={handleWindowKeyDown} />

<div
  bind:this={rootEl}
  class="popover-menu-root {extraClass}"
  data-popover-menu
  style={anchorEl ? 'display: contents;' : undefined}
>
  {#if trigger}
    {@render trigger({ toggle, open, active })}
  {:else if !anchorEl}
    <div class="popover-trigger-wrapper">
      <Button
        variant={triggerVariant ?? 'ghost'}
        class="btn-icon popover-trigger-btn {active ? 'is-on' : ''}"
        onclick={toggle}
        {title}
        aria-expanded={open}
        aria-haspopup="menu"
      >
        {#if icon}
          {@const IconComp = icon}
          <IconComp />
        {:else}
          <IconFilter />
        {/if}
      </Button>

      {#if badge !== undefined && (typeof badge === 'number' ? badge > 0 : !!badge)}
        <span class="popover-badge">{badge}</span>
      {/if}
    </div>
  {/if}

  {#if open}
    <div
      use:portal={'body'}
      use:scrollable={{ overflowX: 'hidden' }}
      bind:this={dropdownEl}
      class="popover-menu-dropdown floating-surface popover-portal {menuClass || 'unified-filter-menu'}"
      data-portal-keep-open={keepOpen ? '' : undefined}
      style:--popover-menu-width={width}
      style="position: fixed; visibility: hidden; z-index: 10000;"
      role="menu"
    >
      {#if children}
        {@render children({ close })}
      {/if}
    </div>
  {/if}
</div>

<style>
  .popover-menu-root {
    position: relative;
    display: inline-flex;
    flex-shrink: 0;
  }

  .popover-trigger-wrapper {
    position: relative;
    display: inline-flex;
    flex-shrink: 0;
  }

  .popover-badge {
    position: absolute;
    top: 4px;
    right: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: var(--radius-full);
    background: var(--accent-primary);
    color: var(--accent-on-primary);
    font-size: var(--type-label-sm-size);
    font-weight: 600;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    pointer-events: none;
    box-shadow: 0 0 0 2px var(--surface);
  }

  :global(.popover-menu-dropdown.popover-portal) {
    position: fixed;
    width: var(--popover-menu-width, max-content);
    min-width: min(220px, calc(100vw - 32px));
    max-width: min(420px, calc(100vw - 32px));
    max-height: min(600px, calc(100vh - 100px));
    overflow-x: hidden !important;
    animation: popoverMenuIn var(--duration-fast) var(--ease-standard);
  }

  @keyframes popoverMenuIn {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
</style>
