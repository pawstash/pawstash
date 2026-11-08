<script lang="ts">
  import { getContext, type Snippet } from 'svelte';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { i18n } from '$lib/i18n';
  import Button from '$lib/components/ui/Button.svelte';
  import WindowControls from './WindowControls.svelte';
  import OverflowMenu, { type MenuItem } from './OverflowMenu.svelte';
  import SelectionBar from './SelectionBar.svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
  import IconSettings from '~icons/fluent/settings-24-regular';

  interface Props {
    title?: string;
    heading?: Snippet<[boolean]>;
    titleAfter?: number;
    titleTrigger?: HTMLElement | null;
    back?: boolean | (() => void);
    views?: Snippet;
    filter?: Snippet;
    actions?: Snippet;
    menu?: MenuItem[];
    selection?: { total: number; onSelectAll?: () => void; actions?: Snippet };
    hideOnScroll?: boolean;
    solid?: boolean;
    height?: number;
    hidden?: boolean;
  }

  let {
    title,
    heading,
    titleAfter = 0,
    titleTrigger = null,
    back = false,
    views,
    filter,
    actions,
    menu = [],
    selection,
    hideOnScroll = true,
    solid = false,
    height = $bindable(0),
    hidden = $bindable(false)
  }: Props = $props();

  const ROOT_CONTENT = new Set(['feed', 'creators', 'favorites', 'library', 'downloads']);
  const HIDE_DELTA = 6;

  const scrollContext = getContext<ScrollableContext | undefined>(SCROLLABLE_CONTEXT);

  let barEl = $state<HTMLElement | null>(null);
  let scrollTop = $state(0);
  let isOffline = $state(typeof navigator !== 'undefined' && !navigator.onLine);

  let isTouch = $derived(layoutState.isMobile);
  let selecting = $derived(Boolean(selection) && selectionState.active);
  let showWindowControls = $derived(!isTouch && layoutState.effectiveTitlebarStyle === 'windows');
  let showSettings = $derived(isTouch && ROOT_CONTENT.has(navigationState.route.name));
  let triggerPassed = $state(false);
  let titleShown = $derived(titleTrigger ? triggerPassed : scrollTop >= titleAfter);
  let condensed = $derived(Boolean(titleTrigger && triggerPassed && title));
  let hasHeading = $derived(Boolean(heading || title));
  let viewsRow = $derived(isTouch && hasHeading && Boolean(views));

  $effect(() => {
    const viewport = scrollContext?.viewport;
    if (!viewport) return;
    let last = viewport.scrollTop;
    scrollTop = last;

    let frame = 0;
    const measureTrigger = () => {
      frame = 0;
      if (!titleTrigger || !barEl) return;
      triggerPassed = titleTrigger.getBoundingClientRect().bottom <= barEl.getBoundingClientRect().bottom;
    };
    void height;
    frame = requestAnimationFrame(measureTrigger);
    const resize = titleTrigger ? new ResizeObserver(() => (frame ||= requestAnimationFrame(measureTrigger))) : null;
    if (titleTrigger) resize?.observe(titleTrigger);

    const onScroll = () => {
      if (titleTrigger && !frame) frame = requestAnimationFrame(measureTrigger);
      const top = viewport.scrollTop;
      const delta = top - last;
      last = top;
      scrollTop = top;
      if (!isTouch || !hideOnScroll || selecting) return;
      if (top <= height || barEl?.contains(document.activeElement) || barEl?.querySelector('[aria-expanded="true"]')) hidden = false;
      else if (delta > HIDE_DELTA) hidden = true;
      else if (delta < -HIDE_DELTA) hidden = false;
    };

    viewport.addEventListener('scroll', onScroll, { passive: true });
    return () => {
      viewport.removeEventListener('scroll', onScroll);
      resize?.disconnect();
      if (frame) cancelAnimationFrame(frame);
    };
  });

  $effect(() => {
    if (!isTouch || !hideOnScroll || selecting) hidden = false;
  });

  $effect(() => {
    const update = () => (isOffline = !navigator.onLine);
    window.addEventListener('online', update);
    window.addEventListener('offline', update);
    return () => {
      window.removeEventListener('online', update);
      window.removeEventListener('offline', update);
    };
  });

  function handleBack() {
    if (typeof back === 'function') back();
    else navigationState.back();
  }
</script>

{#snippet leading()}
  {#if back}
    <Button variant="ghost" class="btn-icon bar-back" aria-label={i18n.t('nav.back')} onclick={handleBack}>
      <IconArrowLeft />
    </Button>
  {/if}
  {#if heading}
    <div class="bar-heading">{@render heading(condensed)}</div>
    {#if condensed}
      <h1 class="bar-title is-swapped">{title}</h1>
    {/if}
  {:else if title}
    <h1 class="bar-title" class:is-hidden={!titleShown}>{title}</h1>
  {/if}
{/snippet}

{#snippet trailing(withFilter: boolean)}
  {#if isOffline}
    <span class="offline-chip" role="status">{i18n.t('nav.offline')}</span>
  {/if}
  {#if actions || (withFilter && filter) || menu.length > 0}
    <div class="bar-actions">
      {#if actions}{@render actions()}{/if}
      {#if withFilter && filter}{@render filter()}{/if}
      <OverflowMenu items={menu} />
    </div>
  {/if}
{/snippet}

{#if !isTouch}
  <header
    class="top-app-bar is-pointer"
    class:has-window-controls={showWindowControls}
    class:has-back={Boolean(back)}
    class:is-selecting={selecting}
    data-control-size="base"
    data-tauri-drag-region="deep"
    bind:this={barEl}
    bind:clientHeight={height}
  >
    {#if selecting && selection}
      <SelectionBar total={selection.total} onSelectAll={selection.onSelectAll} actions={selection.actions} />
    {:else}
      {@render leading()}
      {#if views}
        <div class="bar-views">{@render views()}</div>
      {/if}
      <div class="bar-spacer"></div>
      {@render trailing(true)}
    {/if}
    {#if showWindowControls}
      <WindowControls variant="windows" />
    {/if}
  </header>
{:else}
  <header
    class="top-app-bar is-touch"
    class:is-solid={(solid && scrollTop > 0) || selecting}
    class:is-selecting={selecting}
    class:is-hidden={hidden}
    data-control-size="md"
    bind:this={barEl}
    bind:clientHeight={height}
  >
    {#if selecting && selection}
      <div class="bar-row has-back is-selecting">
        <SelectionBar total={selection.total} onSelectAll={selection.onSelectAll} actions={selection.actions} />
      </div>
    {:else}
      <div class="bar-row" class:has-back={Boolean(back)}>
        {#if hasHeading}
          {@render leading()}
        {:else if views}
          <div class="bar-views">{@render views()}</div>
        {/if}
        <div class="bar-spacer"></div>
        {@render trailing(!viewsRow)}
        {#if showSettings}
          <Button variant="ghost" class="btn-icon" aria-label={i18n.t('nav.settings')} onclick={() => navigationState.openSettings()}>
            <IconSettings />
          </Button>
        {/if}
      </div>
      {#if viewsRow}
        <div class="bar-row is-views" data-control-size="base">
          <div class="bar-views">{@render views?.()}</div>
          {#if filter}{@render filter()}{/if}
        </div>
      {/if}
    {/if}
  </header>
{/if}

<style>
  .top-app-bar {
    display: flex;
    min-width: 0;
    box-sizing: border-box;
    color: var(--text-primary);
    --control-icon-size: 20px;
  }

  .top-app-bar.is-touch {
    --control-icon-size: 24px;
  }

  .bar-title {
    flex: 0 1 auto;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    color: var(--text-primary);
    font-family: var(--font-display);
    font-size: var(--type-title-lg-size);
    line-height: var(--type-title-lg-line);
    font-weight: 400;
    white-space: nowrap;
    text-overflow: ellipsis;
    transition: opacity var(--duration-normal) var(--ease-standard);
  }

  .bar-title.is-hidden {
    opacity: 0;
  }

  .bar-title.is-swapped {
    animation: bar-title-in var(--duration-normal) var(--ease-expo) var(--duration-fast) both;
  }

  @keyframes bar-title-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }

  .bar-heading {
    display: flex;
    align-items: center;
    min-width: 0;
    transition: margin var(--duration-fast) var(--ease-standard);
  }

  .bar-views {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .bar-spacer {
    flex: 1 1 0;
    min-width: 0;
  }

  .bar-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .offline-chip {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    height: 24px;
    padding: 0 10px;
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--text-secondary);
    font-size: var(--type-label-md-size);
    line-height: var(--type-label-md-line);
    font-weight: 500;
    white-space: nowrap;
  }

  .top-app-bar.is-pointer {
    align-items: center;
    gap: 8px;
    height: calc(var(--toolbar-height) + var(--edge-top, 0px));
    flex-shrink: 0;
    padding-top: var(--edge-top, 0px);
    padding-inline: var(--gutter);
    transition: padding-inline-start 300ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .top-app-bar.is-pointer:not(.has-window-controls) {
    padding-inline-end: calc(var(--gutter) + var(--edge-right, 0px));
  }

  :global(html.mac-header-overflow) .top-app-bar.is-pointer {
    padding-inline-start: calc(var(--mac-header-width) - var(--sidebar-width-collapsed) + 8px);
  }

  .top-app-bar.is-pointer .bar-title,
  .top-app-bar.is-pointer .bar-heading {
    margin-inline-end: 16px;
  }

  .top-app-bar .bar-heading:has(+ .bar-title) {
    flex-shrink: 0;
    margin-inline-end: 8px;
  }

  .top-app-bar.is-pointer.has-window-controls {
    padding-inline-end: 0;
  }

  .top-app-bar.is-pointer.is-selecting {
    background: var(--chrome-surface);
  }

  .top-app-bar.is-pointer.has-window-controls .bar-actions {
    margin-inline-end: 12px;
  }

  .top-app-bar.is-pointer :global(.bar-back) {
    margin-inline-start: -10px;
  }

  .top-app-bar.is-pointer:not(.has-window-controls) .bar-actions {
    margin-inline-end: -10px;
  }

  .top-app-bar.is-touch {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: var(--z-sticky);
    flex-direction: column;
    padding-top: var(--mobile-status-bar-height);
    background: transparent;
    transition:
      transform var(--duration-normal) var(--ease-standard),
      background-color var(--duration-normal) var(--ease-standard);
  }

  .top-app-bar.is-touch.is-solid {
    background: var(--surface-container);
  }

  .top-app-bar.is-touch.is-selecting {
    background: var(--chrome-surface);
  }

  .top-app-bar.is-touch.is-hidden {
    transform: translateY(-100%);
  }

  .bar-row {
    display: flex;
    align-items: center;
    gap: 4px;
    height: var(--app-bar-height);
    padding-inline: 16px 4px;
  }

  .bar-row.has-back {
    padding-inline-start: 4px;
  }

  .bar-row.is-views {
    --control-icon-size: 24px;
    gap: 8px;
    height: var(--app-bar-row-height);
    padding-inline: var(--gutter) 4px;
  }

  .bar-row.is-views .bar-views {
    flex: 1 1 auto;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .bar-views::-webkit-scrollbar {
    display: none;
  }

  .bar-row:has(:global(.search-active)) > :global(:not(.bar-actions)),
  .bar-row:has(:global(.search-active)) .bar-actions > :global(:not(.header-actions-root)) {
    /* .btn-icon declares its display with !important. */
    display: none !important;
  }

  .bar-row:has(:global(.search-active)) .bar-actions {
    flex: 1;
    padding-inline: 4px 12px;
  }

  .bar-row:has(:global(.search-active)) .bar-actions > :global(.header-actions-root) {
    flex: 1;
  }
</style>
