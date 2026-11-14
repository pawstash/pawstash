import { layoutState } from '$lib/state/layoutState.svelte';

export interface ScrollableOptions {
  initialScrollTop?: number;
  overflowX?: 'hidden' | 'scroll' | 'visible-hidden' | 'visible-scroll';
  overflowY?: 'hidden' | 'scroll' | 'visible-hidden' | 'visible-scroll';
  onScroll?: (scrollTop: number) => void;
  onReady?: (viewport: HTMLElement | null) => void;
  nativeOnTouch?: boolean;
}

export interface ScrollableContext {
  viewport: HTMLElement | null;
}

export const SCROLLABLE_CONTEXT = Symbol('pawstash-scrollable');

type OverlayScrollbarsModule = typeof import('overlayscrollbars');

let modulePromise: Promise<OverlayScrollbarsModule> | null = null;
let moduleRef: OverlayScrollbarsModule | null = null;
const waiting = new Set<() => void>();

export function warmOverlayScrollbars(): Promise<OverlayScrollbarsModule> {
  if (!modulePromise) {
    modulePromise = import('overlayscrollbars').then((module) => {
      moduleRef = module;
      for (const upgrade of [...waiting]) upgrade();
      waiting.clear();
      return module;
    });
  }
  return modulePromise;
}

function whenReady(upgrade: () => void) {
  if (moduleRef) {
    upgrade();
    return;
  }
  waiting.add(upgrade);
}

export function scrollable(node: HTMLElement, options: ScrollableOptions = {}) {
  if (options.nativeOnTouch && layoutState.isMobile) {
    return nativeScrollable(node, options);
  }

  let destroyed = false;
  let instance: ReturnType<OverlayScrollbarsModule['OverlayScrollbars']> | null = null;
  let viewport = node;
  let observer: ResizeObserver | undefined;

  const handleScroll = () => options.onScroll?.(viewport.scrollTop);

  node.classList.add('scroll-pending');
  node.style.overflowX = options.overflowX === 'hidden' ? 'hidden' : 'auto';
  node.style.overflowY = options.overflowY === 'hidden' ? 'hidden' : 'auto';
  node.addEventListener('scroll', handleScroll, { passive: true });
  observer = new ResizeObserver(handleScroll);
  observer.observe(node);
  options.onReady?.(node);

  requestAnimationFrame(() => {
    if (destroyed) return;
    node.scrollTop = options.initialScrollTop ?? 0;
    handleScroll();
  });

  const upgrade = () => {
    if (destroyed) return;
    const startTop = node.scrollTop;
    node.removeEventListener('scroll', handleScroll);
    observer?.disconnect();
    node.classList.remove('scroll-pending');
    node.style.removeProperty('overflow-x');
    node.style.removeProperty('overflow-y');

    const module = moduleRef!;
    instance = module.OverlayScrollbars(node, {
      overflow: {
        x: options.overflowX ?? 'scroll',
        y: options.overflowY ?? 'scroll'
      },
      scrollbars: {
        theme: 'os-theme-pawstash',
        autoHide: 'leave',
        autoHideDelay: 600,
        clickScroll: true
      }
    });
    const elements = instance.elements();
    viewport = elements.viewport;
    elements.scrollbarHorizontal?.scrollbar?.setAttribute('data-tauri-drag-region', 'false');
    elements.scrollbarVertical?.scrollbar?.setAttribute('data-tauri-drag-region', 'false');
    options.onReady?.(viewport);
    viewport.addEventListener('scroll', handleScroll, { passive: true });
    observer = new ResizeObserver(handleScroll);
    observer.observe(viewport);
    requestAnimationFrame(() => {
      if (destroyed) return;
      viewport.scrollTop = startTop;
      handleScroll();
    });
  };

  whenReady(upgrade);

  return {
    destroy() {
      destroyed = true;
      waiting.delete(upgrade);
      observer?.disconnect();
      node.removeEventListener('scroll', handleScroll);
      viewport.removeEventListener('scroll', handleScroll);
      node.classList.remove('scroll-pending');
      options.onReady?.(null);
      instance?.destroy();
      instance = null;
    }
  };
}

function nativeScrollable(node: HTMLElement, options: ScrollableOptions) {
  const viewport = node;
  viewport.classList.add('native-scroll');
  viewport.style.overflowX = options.overflowX === 'hidden' ? 'hidden' : 'auto';
  viewport.style.overflowY = options.overflowY === 'hidden' ? 'hidden' : 'auto';

  const handleScroll = () => options.onScroll?.(viewport.scrollTop);
  viewport.addEventListener('scroll', handleScroll, { passive: true });
  const resizeObserver = new ResizeObserver(() => handleScroll());
  resizeObserver.observe(viewport);
  options.onReady?.(viewport);

  requestAnimationFrame(() => {
    viewport.scrollTop = options.initialScrollTop ?? 0;
    handleScroll();
  });

  return {
    destroy() {
      resizeObserver.disconnect();
      viewport.removeEventListener('scroll', handleScroll);
      viewport.classList.remove('native-scroll');
      options.onReady?.(null);
    }
  };
}
