<script lang="ts">
  import { tick, onDestroy, type Snippet } from 'svelte';
  import { portal } from '$lib/actions/portal';
  import { scrollable } from '$lib/actions/scrollable';
  import Button from '$lib/components/ui/Button.svelte';

  interface Props {
    isOpen?: boolean;
    open?: boolean;
    title: string;
    closeLabel?: string;
    actionLabel?: string;
    flush?: boolean;
    scrollable?: boolean;
    tall?: boolean;
    onclose: () => void;
    children?: Snippet;
    footer?: Snippet;
    floating?: Snippet;
  }

  let {
    isOpen = false,
    open = false,
    title,
    closeLabel = '',
    actionLabel = closeLabel,
    flush = false,
    scrollable: isScrollable = true,
    tall = false,
    onclose,
    children,
    footer,
    floating
  }: Props = $props();

  const isVisible = $derived(isOpen || open);

  const DISMISS_DISTANCE_RATIO = 0.25;
  const DISMISS_FLICK_DISTANCE_RATIO = 0.12;
  const DISMISS_VELOCITY = 0.65;
  const DRAG_START_THRESHOLD = 6;
  const FLICK_VELOCITY_MAX_IDLE = 80;

  let panel = $state<HTMLElement | null>(null);
  let dragZone = $state<HTMLButtonElement | null>(null);
  let activePointerId = $state<number | null>(null);
  let pointerStartY = 0;
  let lastPointerY = 0;
  let lastPointerTime = 0;
  let dragVelocity = 0;
  let dragOffset = $state(0);
  let isEntering = $state(true);
  let isDragging = $state(false);
  let isClosing = $state(false);
  let closeTimer: ReturnType<typeof setTimeout> | null = null;

  onDestroy(() => {
    if (closeTimer) {
      clearTimeout(closeTimer);
      closeTimer = null;
    }
  });

  let dragProgress = $derived.by(() => {
    const panelHeight = panel?.clientHeight ?? 1;
    return Math.min(dragOffset / panelHeight, 1);
  });

  const focusableSelector = [
    'button:not([disabled])',
    '[href]',
    'input:not([disabled])',
    'select:not([disabled])',
    'textarea:not([disabled])',
    '[tabindex]:not([tabindex="-1"])'
  ].join(',');

  function focusableElements() {
    return panel
      ? Array.from(panel.querySelectorAll<HTMLElement>(focusableSelector)).filter(
          (element) =>
            element.tabIndex >= 0 &&
            !element.hasAttribute('hidden') &&
            element.getAttribute('aria-hidden') !== 'true'
        )
      : [];
  }

  function requestClose() {
    if (isClosing) return;
    if (activePointerId !== null && dragZone?.hasPointerCapture(activePointerId)) {
      dragZone.releasePointerCapture(activePointerId);
    }
    activePointerId = null;
    isEntering = false;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      onclose();
      return;
    }
    isDragging = false;
    isClosing = true;

    if (closeTimer) clearTimeout(closeTimer);
    closeTimer = setTimeout(() => {
      closeTimer = null;
      if (isClosing) {
        isClosing = false;
        dragOffset = 0;
        onclose();
      }
    }, 300);
  }

  function handleAssemblyAnimationEnd(event: AnimationEvent) {
    if (event.currentTarget !== event.target) return;
    if (event.animationName.endsWith('bottom-sheet-in')) {
      isEntering = false;
      return;
    }
    if (!isClosing || !event.animationName.endsWith('bottom-sheet-out')) return;
    if (closeTimer) {
      clearTimeout(closeTimer);
      closeTimer = null;
    }
    isClosing = false;
    dragOffset = 0;
    onclose();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      requestClose();
      return;
    }
    if (event.key !== 'Tab') return;

    const elements = focusableElements();
    if (elements.length === 0) {
      event.preventDefault();
      panel?.focus();
      return;
    }

    const first = elements[0];
    const last = elements[elements.length - 1];
    if (event.shiftKey && (document.activeElement === first || document.activeElement === panel)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && (document.activeElement === last || !panel?.contains(document.activeElement))) {
      event.preventDefault();
      first.focus();
    }
  }

  function handlePointerDown(event: PointerEvent) {
    if (event.button !== 0 || isClosing || activePointerId !== null) return;
    isEntering = false;
    activePointerId = event.pointerId;
    pointerStartY = event.clientY;
    lastPointerY = event.clientY;
    lastPointerTime = event.timeStamp;
    dragVelocity = 0;
    dragOffset = 0;
    (event.currentTarget as HTMLButtonElement).setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent) {
    if (activePointerId !== event.pointerId) return;

    const elapsed = Math.max(event.timeStamp - lastPointerTime, 1);
    dragVelocity = (event.clientY - lastPointerY) / elapsed;
    const nextDragOffset = Math.max(0, event.clientY - pointerStartY);
    lastPointerY = event.clientY;
    lastPointerTime = event.timeStamp;

    if (!isDragging && nextDragOffset < DRAG_START_THRESHOLD) return;
    isDragging = true;
    dragOffset = nextDragOffset;
  }

  function finishPointer(event: PointerEvent, cancelled = false) {
    if (activePointerId !== event.pointerId) return;

    const wasDragging = isDragging;
    const target = event.currentTarget as HTMLButtonElement;
    activePointerId = null;
    if (target.hasPointerCapture(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }
    isDragging = false;

    const panelHeight = panel?.clientHeight ?? 0;
    const dismissDistance = panelHeight * DISMISS_DISTANCE_RATIO;
    const flickDistance = panelHeight * DISMISS_FLICK_DISTANCE_RATIO;
    const velocityIsFresh = event.timeStamp - lastPointerTime <= FLICK_VELOCITY_MAX_IDLE;
    const releaseVelocity = velocityIsFresh ? dragVelocity : 0;
    const isDismissFlick = dragOffset >= flickDistance && releaseVelocity >= DISMISS_VELOCITY;
    if (!cancelled && wasDragging && (dragOffset >= dismissDistance || isDismissFlick)) {
      requestClose();
      return;
    }

    dragOffset = 0;
    dragVelocity = 0;
  }

  function handleLostPointerCapture(event: PointerEvent) {
    if (activePointerId !== event.pointerId) return;
    activePointerId = null;
    isDragging = false;
    dragOffset = 0;
    dragVelocity = 0;
  }

  $effect(() => {
    if (!isVisible) return;

    isEntering = !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const previouslyFocused = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    const previousBodyOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    dragOffset = 0;
    isClosing = false;
    isDragging = false;

    void tick().then(() => {
      const firstFocusable = focusableElements()[0];
      (firstFocusable ?? panel)?.focus();
    });

    return () => {
      document.body.style.overflow = previousBodyOverflow;
      if (previouslyFocused?.isConnected) {
        requestAnimationFrame(() => previouslyFocused.focus());
      }
    };
  });
</script>

{#if isVisible}
  <div
    use:portal={'body'}
    class="bottom-sheet"
    class:is-entering={isEntering}
    class:is-dragging={isDragging}
    class:is-closing={isClosing}
    class:is-tall={tall}
    style={`--bottom-sheet-drag-y: ${dragOffset}px; --bottom-sheet-drag-progress: ${dragProgress};`}
  >
    <button
      type="button"
      class="bottom-sheet__backdrop"
      tabindex="-1"
      aria-label={closeLabel}
      onclick={requestClose}
    ></button>

    <div
      class="bottom-sheet__assembly"
      onanimationend={handleAssemblyAnimationEnd}
    >
      <button
        bind:this={dragZone}
        type="button"
        class="bottom-sheet__drag-zone"
        tabindex="-1"
        aria-hidden="true"
        onpointerdown={handlePointerDown}
        onpointermove={handlePointerMove}
        onpointerup={(event) => finishPointer(event)}
        onpointercancel={(event) => finishPointer(event, true)}
        onlostpointercapture={handleLostPointerCapture}
      >
        <span class="bottom-sheet__handle" aria-hidden="true"></span>
      </button>

      <div
        bind:this={panel}
        class="bottom-sheet__panel"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabindex="-1"
        onkeydown={handleKeydown}
      >
        <header class="bottom-sheet__header">
          <h2 class="bottom-sheet__title">{title}</h2>
        </header>

        {#if children}
          {#if isScrollable}
            <div class="bottom-sheet__body" use:scrollable={{ overflowX: 'hidden' }}>
              <div class="bottom-sheet__content" class:is-flush={flush}>
                {@render children()}
              </div>
            </div>
          {:else}
            <div class="bottom-sheet__body is-unscrollable">
              <div class="bottom-sheet__content is-unscrollable" class:is-flush={flush}>
                {@render children()}
              </div>
            </div>
          {/if}
        {/if}

        {#if footer}
          <footer class="bottom-sheet__footer">
            {@render footer()}
          </footer>
        {:else if actionLabel || closeLabel}
          <footer class="bottom-sheet__footer">
            <Button class="bottom-sheet__close" variant="primary" size="md" onclick={requestClose}>
              {actionLabel || closeLabel}
            </Button>
          </footer>
        {/if}

        {#if floating}
          {@render floating()}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .bottom-sheet {
    position: fixed;
    inset: 0;
    z-index: var(--z-bottom-sheet);
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-top: var(--mobile-status-bar-height);
    box-sizing: border-box;
    isolation: isolate;
  }

  .bottom-sheet__backdrop {
    position: absolute;
    inset: 0;
    z-index: -1;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: var(--bottom-sheet-backdrop);
    opacity: calc(1 - (var(--bottom-sheet-drag-progress) * 0.72));
    transition: opacity var(--duration-normal) var(--ease-expo);
  }

  .bottom-sheet.is-entering .bottom-sheet__backdrop {
    animation: bottom-sheet-backdrop-in var(--duration-normal) var(--ease-expo) both;
  }

  .bottom-sheet.is-dragging .bottom-sheet__backdrop {
    transition: none;
  }

  .bottom-sheet.is-closing .bottom-sheet__backdrop {
    animation: bottom-sheet-backdrop-out var(--duration-normal) var(--ease-expo) both;
  }

  .bottom-sheet__assembly {
    width: min(100%, calc(var(--bottom-sheet-width) * var(--ui-scale, 1)));
    max-height: min(
      var(--bottom-sheet-max-height),
      calc(100dvh - var(--mobile-status-bar-height))
    );
    display: flex;
    flex-direction: column;
    align-items: stretch;
    transform: translate3d(0, var(--bottom-sheet-drag-y), 0);
    transition: transform var(--duration-normal) var(--ease-expo);
    will-change: transform;
  }

  .bottom-sheet.is-entering .bottom-sheet__assembly {
    animation: bottom-sheet-in var(--duration-normal) var(--ease-expo) both;
  }

  .bottom-sheet.is-dragging .bottom-sheet__assembly {
    transition: none;
  }

  .bottom-sheet.is-closing .bottom-sheet__assembly {
    animation: bottom-sheet-out var(--duration-normal) var(--ease-expo) both;
  }

  .bottom-sheet__panel {
    position: relative;
    min-height: 0;
    display: flex;
    flex: 0 1 auto;
    flex-direction: column;
    color: var(--text-primary);
    background: var(--bottom-sheet-surface);
    border: 0;
    border-radius:
      calc(var(--bottom-sheet-radius) * var(--ui-scale, 1))
      calc(var(--bottom-sheet-radius) * var(--ui-scale, 1))
      0 0;
    box-shadow: var(--bottom-sheet-shadow);
    box-sizing: border-box;
    overflow: hidden;
  }

  .bottom-sheet__drag-zone {
    position: relative;
    flex: none;
    min-height: calc(var(--bottom-sheet-drag-target-height) * var(--ui-scale, 1));
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    touch-action: none;
    cursor: grab;
  }

  .bottom-sheet__drag-zone:active {
    cursor: grabbing;
  }

  .bottom-sheet__handle {
    position: absolute;
    bottom: calc(var(--space-2) * var(--ui-scale, 1));
    left: 50%;
    width: calc(var(--bottom-sheet-handle-width) * var(--ui-scale, 1));
    height: calc(var(--bottom-sheet-handle-height) * var(--ui-scale, 1));
    border-radius: var(--radius-full);
    background: var(--bottom-sheet-handle-color);
    transform: translateX(-50%);
  }

  .bottom-sheet.is-tall .bottom-sheet__assembly {
    height: calc(100dvh - var(--mobile-status-bar-height) - 8px);
    max-height: calc(100dvh - var(--mobile-status-bar-height) - 8px);
  }

  .bottom-sheet.is-tall .bottom-sheet__panel {
    flex: 1 1 100%;
    height: 100%;
  }

  .bottom-sheet__body {
    min-height: 0;
    max-width: 100%;
    flex: 1 1 auto;
  }

  .bottom-sheet__body.is-unscrollable {
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .bottom-sheet__header {
    flex: none;
    padding:
      calc(var(--bottom-sheet-padding-top) * var(--ui-scale, 1))
      calc(var(--bottom-sheet-padding-x) * var(--ui-scale, 1))
      0;
  }

  .bottom-sheet__title {
    margin: 0;
    color: var(--text-primary);
    font-family: var(--font-display);
    font-size: calc(var(--text-base) * var(--ui-scale, 1));
    font-weight: var(--font-weight-semibold);
    line-height: var(--leading-tight);
    overflow-wrap: anywhere;
  }

  .bottom-sheet__content {
    padding:
      calc(var(--bottom-sheet-title-content-gap) * var(--ui-scale, 1))
      calc(var(--bottom-sheet-padding-x) * var(--ui-scale, 1));
    box-sizing: border-box;
  }

  .bottom-sheet__content.is-unscrollable {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .bottom-sheet__content.is-flush {
    padding: 0;
  }

  .bottom-sheet__footer {
    flex: none;
    padding:
      calc(var(--bottom-sheet-content-action-gap) * var(--ui-scale, 1))
      calc(var(--bottom-sheet-padding-x) * var(--ui-scale, 1))
      max(
        calc(var(--bottom-sheet-padding-bottom) * var(--ui-scale, 1)),
        var(--safe-bottom)
      );
  }

  .bottom-sheet__footer :global(.bottom-sheet__close) {
    width: 100%;
  }

  @keyframes bottom-sheet-in {
    from {
      transform: translate3d(0, 100%, 0);
    }
    to {
      transform: translate3d(0, 0, 0);
    }
  }

  @keyframes bottom-sheet-out {
    from {
      transform: translate3d(0, var(--bottom-sheet-drag-y), 0);
    }
    to {
      transform: translate3d(0, 110%, 0);
    }
  }

  @keyframes bottom-sheet-backdrop-in {
    from {
      opacity: 0;
    }
  }

  @keyframes bottom-sheet-backdrop-out {
    to {
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .bottom-sheet__assembly,
    .bottom-sheet__backdrop {
      animation: none;
      transition: none;
    }
  }
</style>
