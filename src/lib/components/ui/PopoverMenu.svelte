<script lang="ts">
  import type { Snippet } from 'svelte';
  import { computePosition, autoUpdate, flip, shift, offset, size } from '@floating-ui/dom';
  import { portal } from '$lib/actions/portal';
  import { scrollable } from '$lib/actions/scrollable';
  import Button from '$lib/components/ui/Button.svelte';
  import IconFilter from '~icons/fluent/filter-24-regular';

  interface Props {
    open?: boolean;
    active?: boolean;
    badge?: number | string;
    title?: string;
    icon?: any;
    triggerVariant?: 'ghost' | 'accent' | 'primary';
    align?: 'right' | 'left';
    width?: string;
    class?: string;
    menuClass?: string;
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
    trigger,
    children
  }: Props = $props();

  let rootEl = $state<HTMLDivElement>();
  let dropdownEl = $state<HTMLDivElement | null>(null);
  let cleanupAutoUpdate: (() => void) | null = null;

  function toggle(event: MouseEvent) {
    event.stopPropagation();
    open = !open;
  }

  function close() {
    open = false;
  }

  async function updatePosition() {
    if (!rootEl || !dropdownEl) return;

    const placement = align === 'left' ? 'bottom-start' : 'bottom-end';

    const { x, y } = await computePosition(rootEl, dropdownEl, {
      placement,
      strategy: 'fixed',
      middleware: [
        offset(8),
        flip({
          fallbackPlacements: ['top-end', 'top-start', 'bottom', 'top'],
          padding: 12
        }),
        shift({
          padding: 12
        }),
        size({
          padding: 12,
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
    if (open && rootEl && dropdownEl) {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = autoUpdate(rootEl, dropdownEl, () => {
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
    if (!open || !rootEl) return;
    const target = event.target as HTMLElement;
    if (target.closest('.select-dropdown-portal') || target.closest('[data-portal-keep-open]')) return;
    if (!rootEl.contains(target) && !dropdownEl?.contains(target)) {
      open = false;
    }
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
>
  {#if trigger}
    {@render trigger({ toggle, open, active })}
  {:else}
    <div class="popover-trigger-wrapper">
      <Button
        variant={triggerVariant ?? (open || active ? 'accent' : 'ghost')}
        class="btn-icon popover-trigger-btn"
        onclick={toggle}
        {title}
        aria-label={title || 'Menu'}
        aria-expanded={open}
        aria-haspopup="menu"
      >
        {#if icon}
          {#if typeof icon === 'function'}
            {@const IconComp = icon}
            <IconComp class="w-5 h-5" />
          {:else}
            {@render icon()}
          {/if}
        {:else}
          <IconFilter class="w-5 h-5" />
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
      use:scrollable
      bind:this={dropdownEl}
      class="popover-menu-dropdown unified-filter-menu popover-portal {menuClass}"
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
    top: -3px;
    right: -3px;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 17px;
    height: 17px;
    padding: 0 4px;
    border-radius: var(--radius-full);
    background: var(--accent-primary);
    color: var(--text-on-accent, #fff);
    font-size: 10.5px;
    font-weight: 700;
    line-height: 1;
    pointer-events: none;
    box-shadow: 0 0 0 2px var(--bg-surface);
  }

  :global(.popover-menu-dropdown.popover-portal) {
    position: fixed;
    width: var(--popover-menu-width, min(330px, calc(100vw - 32px)));
    max-height: min(600px, calc(100vh - 100px));
    padding: 10px;
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-xl);
    background: var(--bg-dropdown);
    box-shadow: 0 18px 55px rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    box-sizing: border-box;
    animation: popoverMenuIn 0.18s var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1));
  }

  @keyframes popoverMenuIn {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
</style>
