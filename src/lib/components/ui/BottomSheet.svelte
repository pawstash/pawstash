<script lang="ts">
  import { onDestroy, untrack, type Component, type Snippet } from 'svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { portal } from '$lib/actions/portal';
  import { scrollable } from '$lib/actions/scrollable';
  import { dialogFocus } from '$lib/actions/dialogFocus';
  import { i18n } from '$lib/i18n';
  import MaterialShape from '$lib/components/ui/MaterialShape.svelte';
  import { heroShapeCycle } from '$lib/utils/materialShapes';

  interface Props {
    isOpen?: boolean;
    title: string;
    description?: string;
    closeLabel?: string;
    icon?: Component;
    tone?: 'accent' | 'danger';
    flush?: boolean;
    scrollable?: boolean;
    tall?: boolean;
    onclose: () => void;
    children?: Snippet;
    footer?: Snippet;
  }

  let {
    isOpen = false,
    title,
    description,
    closeLabel,
    icon: Icon,
    tone = 'accent',
    flush = false,
    scrollable: isScrollable = true,
    tall = false,
    onclose,
    children,
    footer
  }: Props = $props();

  let heroNonce = $state(Math.random());
  let wasOpen = untrack(() => isOpen);
  $effect.pre(() => {
    if (isOpen && !wasOpen) heroNonce = Math.random();
    wasOpen = isOpen;
  });
  const heroCycle = $derived(heroShapeCycle(`${title}\u0001${heroNonce}`, tone));
  const heroShape = $derived(heroCycle[0] ?? 'circle');

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
  let isScrolled = $state(false);
  let hasMoreBelow = $state(false);
  let bodyViewport: HTMLElement | null = null;
  let contentEl = $state<HTMLElement | null>(null);

  function measureEdges() {
    if (!bodyViewport) return;
    isScrolled = bodyViewport.scrollTop > 2;
    hasMoreBelow = bodyViewport.scrollHeight - bodyViewport.scrollTop - bodyViewport.clientHeight > 2;
  }

  $effect(() => {
    if (!contentEl) return;
    const observer = new ResizeObserver(measureEdges);
    observer.observe(contentEl);
    return () => observer.disconnect();
  });
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

  $effect(() => {
    if (!isOpen) return;
    return untrack(() =>
      navigationState.registerBackHandler(() => {
        requestClose();
        return true;
      })
    );
  });

  export function close() {
    requestClose();
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
    if (!isOpen) return;

    isEntering = !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    dragOffset = 0;
    isClosing = false;
    isDragging = false;
  });
</script>

{#if isOpen}
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
      aria-label={closeLabel ?? i18n.t('common.close')}
      onclick={requestClose}
    ></button>

    <div
      class="bottom-sheet__assembly"
      onanimationend={handleAssemblyAnimationEnd}
    >
      <div
        bind:this={panel}
        class="bottom-sheet__panel"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabindex="-1"
        use:dialogFocus={{ onEscape: requestClose }}
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

        <header class="bottom-sheet__header" class:is-hero={Boolean(Icon)}>
          {#if Icon}
            <span class="bottom-sheet__hero" data-tone={tone} aria-hidden="true">
              <MaterialShape
                name={heroShape}
                size={56}
                duration={700}
                cycle={heroCycle}
                spin
                class="bottom-sheet__hero-shape"
              />
              <span class="bottom-sheet__hero-glyph"><Icon /></span>
            </span>
          {/if}
          <h2 class="bottom-sheet__title">{title}</h2>
          {#if description}
            <p class="bottom-sheet__description">{description}</p>
          {/if}
        </header>

        {#if children}
          {#if isScrollable}
            <div
              class="bottom-sheet__body"
              class:is-scrolled={isScrolled}
              class:has-more={hasMoreBelow}
              use:scrollable={{
                overflowX: 'hidden',
                onReady: (viewport) => (bodyViewport = viewport),
                onScroll: measureEdges
              }}
            >
              <div class="bottom-sheet__content" class:is-flush={flush} bind:this={contentEl}>
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
    padding-bottom: max(var(--bottom-sheet-gap), var(--safe-bottom));
    background: var(--bottom-sheet-surface);
    border: 0;
    border-radius: var(--bottom-sheet-radius) var(--bottom-sheet-radius) 0 0;
    box-shadow: var(--bottom-sheet-shadow);
    box-sizing: border-box;
    overflow: hidden;
  }

  .bottom-sheet__drag-zone {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
    height: var(--bottom-sheet-handle-zone);
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
    width: var(--bottom-sheet-handle-width);
    height: var(--bottom-sheet-handle-height);
    border-radius: var(--radius-full);
    background: var(--bottom-sheet-handle-color);
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
    --fade-top: 0px;
    --fade-bottom: 0px;
  }

  .bottom-sheet__body.is-scrolled {
    --fade-top: 24px;
  }

  .bottom-sheet__body.has-more {
    --fade-bottom: 24px;
  }

  .bottom-sheet__body.is-scrolled,
  .bottom-sheet__body.has-more {
    -webkit-mask-image: linear-gradient(to bottom, transparent 0, black var(--fade-top), black calc(100% - var(--fade-bottom)), transparent 100%);
    mask-image: linear-gradient(to bottom, transparent 0, black var(--fade-top), black calc(100% - var(--fade-bottom)), transparent 100%);
  }

  :global(html.webkitgtk) .bottom-sheet__body.is-scrolled,
  :global(html.webkitgtk) .bottom-sheet__body.has-more {
    -webkit-mask-image: none;
    mask-image: none;
  }

  .bottom-sheet__body.is-unscrollable {
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .bottom-sheet__header {
    position: relative;
    z-index: 2;
    flex: none;
    padding: 0 var(--bottom-sheet-inset) var(--bottom-sheet-gap);
  }

  .bottom-sheet__header.is-hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding-top: 2px;
    text-align: center;
  }

  .bottom-sheet__hero {
    position: relative;
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    color: var(--bottom-sheet-hero-tone);
  }

  .bottom-sheet__hero :global(.bottom-sheet__hero-shape) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    color: color-mix(in srgb, var(--bottom-sheet-hero-tone) 20%, transparent);
  }

  .bottom-sheet__hero[data-tone='accent'] {
    --bottom-sheet-hero-tone: var(--accent-primary);
  }

  .bottom-sheet__hero[data-tone='danger'] {
    --bottom-sheet-hero-tone: var(--status-error);
  }

  .bottom-sheet__hero-glyph {
    position: relative;
    display: grid;
    place-items: center;
  }

  .bottom-sheet__hero-glyph :global(svg) {
    width: 26px;
    height: 26px;
  }

  .bottom-sheet__title {
    margin: 0;
    color: var(--text-primary);
    font-family: var(--font-display);
    font-size: var(--modal-title-size);
    line-height: var(--type-title-lg-line);
    font-weight: 400;
    overflow-wrap: anywhere;
  }

  .bottom-sheet__description {
    margin: 4px 0 0;
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
  }

  .bottom-sheet__content {
    --menu-inset: 0px;
    padding: 0 var(--bottom-sheet-inset) 8px;
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
    padding: var(--bottom-sheet-gap) var(--bottom-sheet-inset) 0;
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
