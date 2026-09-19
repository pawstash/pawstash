<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { ripple } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import IconSuccess from '~icons/fluent/checkmark-24-filled';
  import IconError from '~icons/fluent/error-circle-24-filled';
  import IconWarning from '~icons/fluent/warning-24-filled';
  import IconInfo from '~icons/fluent/info-24-filled';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconClose from '~icons/fluent/dismiss-20-regular';
  import IconFavorited from '~icons/fluent/heart-24-filled';
  import IconUnfavorited from '~icons/fluent/heart-broken-24-regular';
  import IconRemoved from '~icons/fluent/folder-dismiss-24-regular';
  import IconDeleted from '~icons/fluent/delete-24-filled';
  import IconCopied from '~icons/fluent/copy-24-filled';
  import IconDownload from '~icons/fluent/arrow-download-24-filled';
  import IconCleared from '~icons/fluent/broom-24-regular';

  export type ToastVariant = 'success' | 'error' | 'warning' | 'info' | 'loading';

  export type ToastGlyph =
    | 'favorited'
    | 'unfavorited'
    | 'removed'
    | 'deleted'
    | 'copied'
    | 'download'
    | 'cleared';

  interface Props {
    id: number | string;
    variant: ToastVariant;
    title: string;
    description?: string;
    swatch?: string;
    glyph?: ToastGlyph;
    actionLabel?: string;
    onaction?: () => void;
    ondismiss?: () => void;
    dismissible?: boolean;
    duration?: number;
    repeat?: number;
  }

  let {
    id,
    variant,
    title,
    description,
    swatch,
    glyph,
    actionLabel,
    onaction,
    ondismiss,
    dismissible = true,
    duration = Number.POSITIVE_INFINITY,
    repeat = 1
  }: Props = $props();

  const countsDown = $derived(Number.isFinite(duration) && duration > 0);
  const runKey = $derived(`${variant}|${title}|${description ?? ''}|${duration}`);

  let mounted = $state(false);
  onMount(() => {
    mounted = true;
  });

  const GLYPHS = {
    success: IconSuccess,
    error: IconError,
    warning: IconWarning,
    info: IconInfo,
    loading: IconLoading
  };

  const ACTION_GLYPHS: Record<ToastGlyph, typeof IconSuccess> = {
    favorited: IconFavorited,
    unfavorited: IconUnfavorited,
    removed: IconRemoved,
    deleted: IconDeleted,
    copied: IconCopied,
    download: IconDownload,
    cleared: IconCleared
  };

  const Glyph = $derived(glyph ? ACTION_GLYPHS[glyph] : GLYPHS[variant]);

  function dismiss() {
    if (ondismiss) ondismiss();
    else toast.dismiss(id);
  }
</script>

<div
  class="toast"
  class:has-close={dismissible}
  class:is-stacked={Boolean(description) || Boolean(actionLabel)}
  class:is-swapped={mounted}
  data-variant={variant}
  style:--toast-duration={countsDown ? `${duration}ms` : undefined}
>
  {#key runKey}
    <span class="toast__glyph" aria-hidden="true">
      <Glyph />
    </span>

    <div class="toast__text">
      <p class="toast__title">
        {title}
        {#if repeat > 1}
          <span class="toast__repeat">&times;{repeat}</span>
        {/if}
      </p>
      {#if description}
        <p class="toast__desc">
          {#if swatch}
            <span class="toast__swatch" style:background={swatch} aria-hidden="true"></span>
          {/if}{description}
        </p>
      {/if}
      {#if actionLabel}
        <button
          type="button"
          class="toast__action"
          use:ripple
          onclick={() => {
            onaction?.();
            dismiss();
          }}
        >
          {actionLabel}
        </button>
      {/if}
    </div>

    {#if dismissible}
      <button
        type="button"
        class="toast__close"
        use:ripple
        aria-label={i18n.t('common.close')}
        onclick={dismiss}
      >
        <IconClose />
        {#if countsDown}
          <svg class="toast__ring" viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="12" cy="12" r="10.5" />
          </svg>
        {/if}
      </button>
    {/if}
  {/key}
</div>

<style>
  .toast {
    --toast-close-slack: calc(
      (var(--toast-close-size) - var(--toast-close-icon)) / 2 * var(--ui-scale, 1)
    );
    position: relative;
    display: flex;
    align-items: center;
    gap: calc(var(--floating-card-gap) * var(--ui-scale, 1));
    width: 100%;
    min-height: calc(
      (var(--border-width) * 2 + var(--toast-padding-y) * 2 + var(--toast-close-size)) * var(--ui-scale, 1)
    );
    padding: calc(var(--toast-padding-y) * var(--ui-scale, 1))
             calc(var(--toast-padding-x) * var(--ui-scale, 1));
    box-sizing: border-box;
    color: var(--text-primary);
    background: var(--floating-bg);
    border: var(--floating-border);
    border-radius: calc(var(--toast-radius) * var(--ui-scale, 1));
    box-shadow: var(--floating-shadow);
    backdrop-filter: var(--floating-backdrop);
    -webkit-backdrop-filter: var(--floating-backdrop);
    overflow: hidden;
  }

  .toast__glyph {
    flex: none;
    display: grid;
    place-items: center;
    width: calc(var(--floating-card-icon-size) * var(--ui-scale, 1));
    height: calc(var(--floating-card-icon-size) * var(--ui-scale, 1));
    color: var(--text-secondary);
  }

  .toast.is-stacked {
    align-items: flex-start;
  }

  .toast.is-stacked .toast__glyph {
    height: calc(
      var(--floating-card-title-size) * var(--floating-card-title-line-height) * var(--ui-scale, 1)
    );
  }

  .toast__glyph :global(svg) {
    width: calc(var(--floating-card-icon-size) * var(--ui-scale, 1));
    height: calc(var(--floating-card-icon-size) * var(--ui-scale, 1));
  }

  .toast__text {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .toast.has-close .toast__title {
    padding-right: calc(
      (var(--toast-close-size) + var(--floating-card-gap)) * var(--ui-scale, 1)
      - var(--toast-close-slack)
    );
  }

  .toast__title {
    margin: 0;
    font-family: var(--font-sans);
    font-size: calc(var(--floating-card-title-size) * var(--ui-scale, 1));
    font-weight: var(--floating-card-title-weight);
    line-height: var(--floating-card-title-line-height);
    letter-spacing: -0.01em;
    color: var(--text-primary);
    overflow-wrap: anywhere;
  }

  .toast__desc {
    margin: calc(var(--space-1) * var(--ui-scale, 1)) 0 0;
    font-size: calc(var(--floating-card-desc-size) * var(--ui-scale, 1));
    font-weight: var(--floating-card-desc-weight);
    line-height: 1.4;
    color: var(--floating-card-desc-color);
    overflow-wrap: anywhere;
  }

  .toast__action {
    align-self: flex-end;
    margin-top: calc(var(--space-1) * var(--ui-scale, 1));
    margin-right: calc(
      (var(--toast-padding-x) - var(--toast-trailing-inset)) * var(--ui-scale, 1) * -1
    );
    margin-bottom: calc(
      (var(--toast-padding-y) - var(--toast-trailing-inset)) * var(--ui-scale, 1) * -1
    );
    height: calc(var(--control-height-sm) * var(--ui-scale, 1));
    padding: 0 calc(var(--control-padding-sm) * var(--ui-scale, 1));
    font-family: var(--font-sans);
    font-size: calc(var(--control-font-sm) * var(--ui-scale, 1));
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    background: transparent;
    border: none;
    border-radius: var(--radius-full);
    cursor: pointer;
    outline: none;
    transition: background var(--duration-fast) var(--ease-expo);
  }

  .toast.has-close .toast__action {
    margin-top: max(
      calc(var(--space-1) * var(--ui-scale, 1)),
      calc(
        (var(--toast-close-size) - var(--floating-card-title-size) * var(--floating-card-title-line-height)) * var(--ui-scale, 1)
        + var(--space-1)
      )
    );
  }

  .toast__action:hover {
    background: var(--bg-card-hover);
  }

  .toast__action:focus-visible {
    outline: 2px solid var(--accent-on-surface);
    outline-offset: -1px;
  }

  .toast__close {
    position: absolute;
    top: calc(50% - (var(--toast-close-size) * var(--ui-scale, 1) / 2));
    right: calc(var(--toast-padding-x) * var(--ui-scale, 1) - var(--toast-close-slack));
    display: grid;
    place-items: center;
    width: calc(var(--toast-close-size) * var(--ui-scale, 1));
    height: calc(var(--toast-close-size) * var(--ui-scale, 1));
    color: var(--text-muted);
    background: transparent;
    border: none;
    border-radius: var(--radius-full);
    cursor: pointer;
    outline: none;
    transition: color var(--duration-fast) var(--ease-expo),
                background var(--duration-fast) var(--ease-expo);
  }

  .toast__close :global(svg:not(.toast__ring)) {
    width: calc(var(--toast-close-icon) * var(--ui-scale, 1));
    height: calc(var(--toast-close-icon) * var(--ui-scale, 1));
  }

  .toast__close:hover {
    color: var(--text-primary);
    background: var(--bg-card-hover);
  }

  .toast.is-stacked .toast__close {
    top: max(
      0px,
      calc(
        var(--toast-padding-y) * var(--ui-scale, 1)
        - (
          var(--toast-close-size) * var(--ui-scale, 1)
          - var(--floating-card-title-size) * var(--floating-card-title-line-height) * var(--ui-scale, 1)
        ) / 2
      )
    );
  }

  .toast__close:focus-visible {
    outline: 2px solid var(--accent-on-surface);
    outline-offset: -1px;
  }

  .toast__swatch {
    display: inline-block;
    width: calc(var(--toast-swatch-size) * var(--ui-scale, 1));
    height: calc(var(--toast-swatch-size) * var(--ui-scale, 1));
    margin-right: calc(var(--space-1) * var(--ui-scale, 1));
    border-radius: var(--radius-full);
    vertical-align: baseline;
    flex: none;
  }

  .toast__repeat {
    margin-left: calc(var(--space-1) * var(--ui-scale, 1));
    font-variant-numeric: tabular-nums;
    font-weight: var(--font-weight-normal);
    color: var(--text-muted);
  }

  .toast.is-swapped .toast__glyph,
  .toast.is-swapped .toast__text,
  .toast.is-swapped .toast__close {
    animation: toast-swap var(--duration-normal) var(--ease-merge) both;
  }

  @keyframes toast-swap {
    from {
      opacity: 0;
      transform: translateX(calc(var(--toast-merge-shift) * -1)) scale(0.94);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast.is-swapped .toast__glyph,
    .toast.is-swapped .toast__text,
    .toast.is-swapped .toast__close {
      animation: none;
    }
  }

  .toast__ring {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    fill: none;
    stroke: currentColor;
    stroke-width: var(--toast-ring-width);
    stroke-linecap: round;
    opacity: 0.9;
    transform: rotate(-90deg);
    stroke-dasharray: var(--toast-ring-length);
    animation: toast-ring var(--toast-duration) linear forwards;
    pointer-events: none;
  }

  :global([data-sonner-toast][data-expanded='true']) .toast__ring,
  :global([data-sonner-toast][data-swiping='true']) .toast__ring {
    animation-play-state: paused;
  }

  @keyframes toast-ring {
    from {
      stroke-dashoffset: 0;
    }
    to {
      stroke-dashoffset: var(--toast-ring-length);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast__ring {
      display: none;
    }
  }

  @media (pointer: coarse) {
    .toast {
      --toast-close-size: var(--tap-target-min);
    }

    .toast__action {
      min-height: calc(var(--tap-target-min) * var(--ui-scale, 1));
    }
  }
</style>
