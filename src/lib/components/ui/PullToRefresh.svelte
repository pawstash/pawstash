<script lang="ts">
  import { onMount } from 'svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  interface Props {
    onrefresh?: () => Promise<void> | void;
    scrollContainer?: HTMLElement | null;
    disabled?: boolean;
    class?: string;
  }

  let {
    onrefresh,
    scrollContainer,
    disabled = false,
    class: extraClass = ''
  }: Props = $props();

  const THRESHOLD = 68;
  const MAX_PULL = 104;
  const RESTING_OFFSET = 58;
  const RESISTANCE = 0.44;

  let pullDistance = $state(0);
  let pulling = $state(false);
  let refreshing = $state(false);
  let exiting = $state(false);
  let thresholdPassed = $state(false);

  let startY = 0;
  let startX = 0;
  let isTracking = false;

  let pullProgress = $derived(Math.min(1, pullDistance / THRESHOLD));
  let isVisible = $derived(pullDistance > 4 || refreshing || exiting);

  function getScrollTop(): number {
    if (!scrollContainer) return 0;
    return scrollContainer.scrollTop;
  }

  function handleTouchStart(e: TouchEvent) {
    if (!layoutState.isMobile || disabled || refreshing || !onrefresh || e.touches.length !== 1) {
      return;
    }

    if (getScrollTop() > 2) {
      isTracking = false;
      return;
    }

    startY = e.touches[0].clientY;
    startX = e.touches[0].clientX;
    isTracking = true;
    pulling = false;
    thresholdPassed = false;
  }

  function handleTouchMove(e: TouchEvent) {
    if (!isTracking || refreshing || disabled || !onrefresh || e.touches.length !== 1) {
      return;
    }

    if (getScrollTop() > 1) {
      isTracking = false;
      if (pullDistance > 0) {
        pullDistance = 0;
        pulling = false;
      }
      return;
    }

    const currentY = e.touches[0].clientY;
    const currentX = e.touches[0].clientX;
    const diffY = currentY - startY;
    const diffX = Math.abs(currentX - startX);

    // Prevent trigger on horizontal swipe gestures
    if (diffX > diffY && diffX > 8) {
      isTracking = false;
      return;
    }

    if (diffY > 0 && getScrollTop() <= 0) {
      pulling = true;
      const raw = diffY * RESISTANCE;
      pullDistance = Math.min(MAX_PULL, raw);

      if (pullDistance >= THRESHOLD && !thresholdPassed) {
        thresholdPassed = true;
        if (typeof navigator !== 'undefined' && navigator.vibrate) {
          try { navigator.vibrate(10); } catch {}
        }
      } else if (pullDistance < THRESHOLD && thresholdPassed) {
        thresholdPassed = false;
      }

      if (pullDistance > 6 && e.cancelable) {
        e.preventDefault();
      }
    }
  }

  async function handleTouchEnd() {
    if (!isTracking && !pulling) return;
    isTracking = false;

    if (pullDistance >= THRESHOLD && onrefresh && !refreshing) {
      refreshing = true;
      pulling = false;
      pullDistance = RESTING_OFFSET;

      try {
        await onrefresh();
      } catch (err) {
        console.warn('Pull-to-refresh action failed:', err);
      } finally {
        refreshing = false;
        exiting = true;
        pullDistance = 0;
        setTimeout(() => {
          exiting = false;
          thresholdPassed = false;
        }, 250);
      }
    } else {
      pulling = false;
      exiting = true;
      pullDistance = 0;
      setTimeout(() => {
        exiting = false;
        thresholdPassed = false;
      }, 250);
    }
  }
  onMount(() => {
    window.addEventListener('touchstart', handleTouchStart, { passive: true });
    window.addEventListener('touchmove', handleTouchMove, { passive: false });
    window.addEventListener('touchend', handleTouchEnd, { passive: true });
    window.addEventListener('touchcancel', handleTouchEnd, { passive: true });

    return () => {
      window.removeEventListener('touchstart', handleTouchStart);
      window.removeEventListener('touchmove', handleTouchMove);
      window.removeEventListener('touchend', handleTouchEnd);
      window.removeEventListener('touchcancel', handleTouchEnd);
    };
  });
</script>

{#if isVisible && layoutState.isMobile}
  <div
    class="ptr-indicator-wrapper {extraClass}"
    class:refreshing
    class:ready={thresholdPassed}
    class:exiting
    style:--ptr-offset="{pullDistance}px"
    style:--ptr-progress={pullProgress}
    aria-hidden="true"
  >
    <div class="ptr-badge">
      {#if refreshing}
        <IconLoading class="ptr-icon text-white" />
      {:else}
        <span
          class="ptr-icon-wrap"
          style:transform={`rotate(${pullProgress * 300}deg) scale(${0.8 + pullProgress * 0.2})`}
        >
          <IconArrowClockwise class="ptr-icon text-white" />
        </span>
      {/if}
    </div>
  </div>
{/if}

<style>
  .ptr-indicator-wrapper {
    position: absolute;
    top: calc(var(--mobile-status-bar-height, 0px) + 8px);
    left: 50%;
    transform: translate(-50%, var(--ptr-offset, 0px));
    z-index: 1000;
    pointer-events: none;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: var(--ptr-progress, 0);
    transition: transform 250ms cubic-bezier(0.16, 1, 0.3, 1), opacity 200ms ease;
    will-change: transform, opacity;
  }

  .ptr-indicator-wrapper.refreshing {
    opacity: 1;
    transform: translate(-50%, var(--ptr-offset, 58px));
  }

  .ptr-indicator-wrapper.exiting {
    opacity: 0;
    transform: translate(-50%, 0px);
  }

  .ptr-badge {
    width: 52px;
    height: 52px;
    border-radius: 50%;
    background: rgba(16, 18, 24, 0.94);
    backdrop-filter: blur(28px) saturate(200%);
    -webkit-backdrop-filter: blur(28px) saturate(200%);
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.75);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ffffff;
    transition: box-shadow 180ms ease, transform 180ms ease;
  }

  .ptr-indicator-wrapper.ready .ptr-badge {
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.85), 0 0 24px var(--accent-glow, rgba(168, 85, 247, 0.5));
    transform: scale(1.08);
  }

  .ptr-icon-wrap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    will-change: transform;
    transition: transform 120ms linear;
  }

  :global(.ptr-icon) {
    width: 26px !important;
    height: 26px !important;
    color: #ffffff !important;
  }
</style>
