<script lang="ts">
  import { tick } from 'svelte';
  import MediaViewer from './MediaViewer.svelte';
  import MediaDock from './MediaDock.svelte';
  import { viewerState } from '$lib/state/viewerState.svelte';
  import { mediaDockState } from '$lib/state/mediaDockState.svelte';
  import { mediaRegistry } from '$lib/media/mediaRegistry';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';

  let item = $derived(viewerState.current);
  let callbacks = $derived(viewerState.callbacks);

  /**
   * Windowed viewer fills the whole height (covering the title bar; only its controls float
   * above) and starts to the right of the sidebar. OS fullscreen and mobile use no inset.
   */
  function measureShell() {
    const root = document.documentElement;
    if (layoutState.isMobile) {
      root.style.setProperty('--viewer-shell-left', '0px');
      return;
    }
    const side = document.querySelector('.sidebar-aside')?.getBoundingClientRect();
    root.style.setProperty('--viewer-shell-left', `${Math.round(side?.width ?? 0)}px`);
  }

  $effect(() => {
    if (viewerState.mode !== 'full') {
      document.documentElement.style.removeProperty('--viewer-shell-left');
      return;
    }

    let observer: ResizeObserver | undefined;
    let frame = 0;

    const attach = () => {
      const side = document.querySelector('.sidebar-aside');
      if (!side) {
        frame = requestAnimationFrame(attach);
        return;
      }
      measureShell();
      observer = new ResizeObserver(() => measureShell());
      observer.observe(side);
    };

    frame = requestAnimationFrame(attach);
    const remeasure = () => requestAnimationFrame(measureShell);
    window.addEventListener('resize', remeasure);

    return () => {
      cancelAnimationFrame(frame);
      observer?.disconnect();
      window.removeEventListener('resize', remeasure);
    };
  });

  let lastEntryKey = navigationState.entryKey;
  $effect(() => {
    const entryKey = navigationState.entryKey;
    if (entryKey === lastEntryKey) return;
    lastEntryKey = entryKey;
    // Leaving the page while the viewer is open always collapses it to the dock.
    const touchBlocked = layoutState.isMobile && !(configState.settings.viewer_dock_on_touch ?? true);
    if (viewerState.mode === 'full' && !touchBlocked) {
      mediaDockState.setPeeked(false);
      viewerState.dock();
    }
  });

  function handleClose(finalIndex?: number, _finalTime?: number) {
    if (finalIndex !== undefined) viewerState.setIndex(finalIndex);
    viewerState.close();
  }

  function handleDock(finalIndex?: number, _finalTime?: number) {
    if (finalIndex !== undefined) viewerState.setIndex(finalIndex);
    mediaDockState.setPeeked(false);
    viewerState.dock();
  }

  async function expand() {
    const resume = !mediaDockState.keepPlaying;
    viewerState.expand();
    if (!resume) return;
    await tick();
    const element = mediaRegistry.element(viewerState.currentKey);
    void element?.play().catch(() => {});
  }

  function previous() {
    viewerState.setIndex(viewerState.index - 1);
  }

  function next() {
    viewerState.setIndex(viewerState.index + 1);
  }
</script>

{#if viewerState.mode === 'full' && item}
  <MediaViewer
    items={viewerState.items}
    initialIndex={viewerState.index}
    initialTime={viewerState.initialTime}
    originRect={viewerState.originRect}
    onclose={handleClose}
    ondock={handleDock}
    onindexchange={(index) => viewerState.setIndex(index)}
    ondownload={callbacks.ondownload
      ? (mediaItem, index) => callbacks.ondownload?.(mediaItem, index)
      : undefined}
    onopenpost={callbacks.onopenpost
      ? (mediaItem, index) => callbacks.onopenpost?.(mediaItem, index)
      : undefined}
    ondelete={callbacks.ondelete
      ? (mediaItem, index) => callbacks.ondelete?.(mediaItem, index)
      : undefined}
  />
{:else if viewerState.mode === 'docked' && item}
  <MediaDock
    item={item}
    index={viewerState.index}
    count={viewerState.items.length}
    onexpand={expand}
    onclose={() => viewerState.close()}
    onprevious={previous}
    onnext={next}
  />
{/if}
