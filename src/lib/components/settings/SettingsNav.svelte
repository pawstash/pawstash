<script lang="ts">
  import { onMount } from 'svelte';
  import { ripple } from '$lib/motion';
  import StableWeightLabel from '$lib/components/ui/StableWeightLabel.svelte';

  export interface NavCategory {
    id: string;
    label: string;
    icon?: any;
  }

  interface Props {
    categories: NavCategory[];
    activeCategory: string;
    onselect: (id: string) => void;
    class?: string;
  }

  let {
    categories,
    activeCategory,
    onselect,
    class: extraClass = ''
  }: Props = $props();

  let containerEl = $state<HTMLElement | null>(null);
  let buttonEls: Record<string, HTMLButtonElement | null> = {};

  let isPointerDown = false;
  let isDragging = $state(false);
  let pointerStartX = 0;
  let pointerStartY = 0;
  let scrollStartLeft = 0;
  let suppressClick = false;
  let suppressTimeout: ReturnType<typeof setTimeout> | undefined;

  export function revealCategory(id: string, smooth = true) {
    if (!containerEl || isPointerDown || isDragging) return;
    if (containerEl.clientWidth === 0 || containerEl.closest('[inert]')) return;
    const button = buttonEls[id];
    if (!button) return;

    const containerWidth = containerEl.clientWidth;
    const buttonLeft = button.offsetLeft;
    const buttonWidth = button.offsetWidth;
    const currentScroll = containerEl.scrollLeft;

    const edgePadding = 28;
    const isComfortablyVisible =
      buttonLeft >= currentScroll + edgePadding &&
      buttonLeft + buttonWidth <= currentScroll + containerWidth - edgePadding;

    // During scroll-spy (smooth === false), don't jitter if already comfortably visible
    if (!smooth && isComfortablyVisible) {
      return;
    }

    const targetLeft = Math.max(0, buttonLeft - (containerWidth - buttonWidth) / 2);
    const prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const behavior: ScrollBehavior = prefersReduced || !smooth ? 'auto' : 'smooth';

    containerEl.scrollTo({ left: targetLeft, behavior });
  }

  $effect(() => {
    if (activeCategory) {
      revealCategory(activeCategory, false);
    }
  });

  function handlePointerDown(e: PointerEvent) {
    if (!containerEl || e.button !== 0) return;
    isPointerDown = true;
    isDragging = false;
    pointerStartX = e.clientX;
    pointerStartY = e.clientY;
    scrollStartLeft = containerEl.scrollLeft;
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isPointerDown || !containerEl) return;
    const dx = e.clientX - pointerStartX;
    const dy = e.clientY - pointerStartY;

    if (!isDragging) {
      if (Math.abs(dx) > 5 && Math.abs(dx) >= Math.abs(dy)) {
        isDragging = true;
        suppressClick = true;
        containerEl.setPointerCapture?.(e.pointerId);
      }
    }

    if (isDragging) {
      containerEl.scrollLeft = scrollStartLeft - dx;
      e.preventDefault();
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (!isPointerDown) return;
    isPointerDown = false;
    if (isDragging) {
      try {
        containerEl?.releasePointerCapture?.(e.pointerId);
      } catch {
        // ignore
      }
      isDragging = false;
      suppressClick = true;
      if (suppressTimeout) clearTimeout(suppressTimeout);
      suppressTimeout = setTimeout(() => {
        suppressClick = false;
      }, 120);
    }
  }

  function handleClickCapture(e: MouseEvent) {
    if (suppressClick || isDragging) {
      e.preventDefault();
      e.stopPropagation();
      e.stopImmediatePropagation();
      suppressClick = false;
    }
  }

  function handleTabClick(id: string) {
    if (suppressClick || isDragging) return;
    onselect(id);
    revealCategory(id, true);
  }

  function handleKeydown(e: KeyboardEvent, index: number) {
    let nextIndex = index;
    if (e.key === 'ArrowRight') {
      nextIndex = (index + 1) % categories.length;
    } else if (e.key === 'ArrowLeft') {
      nextIndex = (index - 1 + categories.length) % categories.length;
    } else if (e.key === 'Home') {
      nextIndex = 0;
    } else if (e.key === 'End') {
      nextIndex = categories.length - 1;
    } else {
      return;
    }

    e.preventDefault();
    const nextCat = categories[nextIndex];
    if (!nextCat) return;

    onselect(nextCat.id);
    revealCategory(nextCat.id, true);
    buttonEls[nextCat.id]?.focus({ preventScroll: true });
  }

  onMount(() => {
    const el = containerEl;
    if (!el) return;

    function handleWheel(e: WheelEvent) {
      if (el && el.scrollWidth > el.clientWidth) {
        const delta = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
        if (delta !== 0) {
          const prev = el.scrollLeft;
          el.scrollLeft += delta;
          if (el.scrollLeft !== prev) {
            e.preventDefault();
          }
        }
      }
    }

    el.addEventListener('wheel', handleWheel, { passive: false });
    return () => {
      el.removeEventListener('wheel', handleWheel);
      if (suppressTimeout) clearTimeout(suppressTimeout);
    };
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  bind:this={containerEl}
  class="settings-nav {extraClass}"
  class:is-dragging={isDragging}
  role="tablist"
  tabindex="-1"
  aria-label="Settings categories"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerUp}
  onclickcapture={handleClickCapture}
>
  {#each categories as category, index (category.id)}
    {@const isActive = category.id === activeCategory}
    <button
      bind:this={buttonEls[category.id]}
      type="button"
      role="tab"
      aria-selected={isActive}
      tabindex={isActive ? 0 : -1}
      data-settings-category={category.id}
      class="settings-nav__item"
      class:is-active={isActive}
      use:ripple
      onclick={() => handleTabClick(category.id)}
      onkeydown={(e) => handleKeydown(e, index)}
    >
      {#if category.icon}
        <category.icon class="settings-nav__icon" />
      {/if}
      <StableWeightLabel text={category.label} reserveWeight="var(--font-weight-semibold, 600)" />
    </button>
  {/each}
</div>

<style>
  .settings-nav {
    display: flex;
    align-items: center;
    flex: 1 1 auto;
    min-width: 0;
    gap: calc(4px * var(--ui-scale, 1));
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    -ms-overflow-style: none;
    -webkit-overflow-scrolling: touch;
    touch-action: pan-x pan-y;
    user-select: none;
    white-space: nowrap;
    -webkit-mask-image: linear-gradient(to right, black calc(100% - 24px), transparent 100%);
    mask-image: linear-gradient(to right, black calc(100% - 24px), transparent 100%);
    padding-right: calc(20px * var(--ui-scale, 1));
    cursor: grab;
  }

  .settings-nav::-webkit-scrollbar {
    display: none;
  }

  .settings-nav.is-dragging {
    cursor: grabbing;
  }

  .settings-nav.is-dragging * {
    cursor: grabbing !important;
    user-select: none !important;
  }

  :global(.page-shell.mobile) .settings-nav,
  :global([data-layout='mobile']) .settings-nav {
    margin-left: -12px;
    padding-left: 12px;
    cursor: auto;
  }

  @media (max-width: 768px) {
    .settings-nav {
      margin-left: -12px;
      padding-left: 12px;
      cursor: auto;
    }
  }

  .settings-nav__item {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    height: calc(var(--control-height, var(--control-height-md, 46px)) * var(--ui-scale, 1));
    min-width: calc(var(--control-height, var(--control-height-md, 46px)) * var(--ui-scale, 1));
    padding: 0 calc(var(--control-padding-x, var(--control-padding-md, 20px)) * var(--ui-scale, 1));
    gap: calc(6px * var(--ui-scale, 1));
    border: 0;
    border-radius: calc(var(--radius-sm, 6px) * var(--ui-scale, 1));
    background: var(--input-bg, rgba(255, 255, 255, 0.06));
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-size: calc(var(--control-font-size, var(--control-font-md, 14px)) * var(--ui-scale, 1));
    font-weight: var(--font-weight-normal, 400);
    line-height: normal;
    white-space: nowrap;
    cursor: pointer;
    box-sizing: border-box;
    transition:
      background var(--duration-fast, 150ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
      color var(--duration-fast, 150ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
      border-radius var(--duration-normal, 240ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
      transform var(--duration-fast, 150ms) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1));
  }

  .settings-nav__item:hover:not(.is-active) {
    background: var(--input-bg-hover, rgba(255, 255, 255, 0.095));
    color: var(--text-primary);
  }

  .settings-nav__item:active {
    transform: scale(0.98);
  }

  .settings-nav__item:focus-visible {
    outline: calc(1.5px * var(--ui-scale, 1)) solid var(--accent-primary) !important;
    outline-offset: calc(-1.5px * var(--ui-scale, 1)) !important;
    box-shadow: none !important;
    z-index: 1;
  }

  .settings-nav__item.is-active {
    border-radius: min(
      calc(var(--radius-full, 9999px) * var(--ui-scale, 1)),
      calc(var(--control-height, 46px) / 2)
    );
    background: var(--choice-active-bg, var(--accent-primary));
    color: var(--choice-active-text, var(--text-on-accent, #ffffff));
    font-weight: var(--font-weight-semibold, 600);
  }

  .settings-nav__item :global(svg) {
    width: calc(var(--control-icon-size, var(--control-icon-md, 20px)) * var(--ui-scale, 1));
    height: calc(var(--control-icon-size, var(--control-icon-md, 20px)) * var(--ui-scale, 1));
    flex-shrink: 0;
    opacity: 0.72;
    transition: opacity var(--duration-fast, 150ms) var(--ease-expo);
  }

  .settings-nav__item:hover:not(.is-active) :global(svg) {
    opacity: 1;
  }

  .settings-nav__item.is-active :global(svg) {
    color: var(--choice-active-text, currentColor);
    opacity: 1;
  }

  @media (prefers-reduced-motion: reduce) {
    .settings-nav__item {
      transition: none !important;
    }
  }
</style>
