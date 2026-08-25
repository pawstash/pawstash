import { OverlayScrollbars } from 'overlayscrollbars';

export interface ScrollableOptions {
  initialScrollTop?: number;
  overflowX?: 'hidden' | 'scroll' | 'visible-hidden' | 'visible-scroll';
  overflowY?: 'hidden' | 'scroll' | 'visible-hidden' | 'visible-scroll';
  onScroll?: (scrollTop: number) => void;
  onReady?: (viewport: HTMLElement | null) => void;
}

export interface ScrollableContext {
  viewport: HTMLElement | null;
}

export const SCROLLABLE_CONTEXT = Symbol('pawstash-scrollable');

export function scrollable(node: HTMLElement, options: ScrollableOptions = {}) {
  const instance = OverlayScrollbars(node, {
    overflow: {
      x: options.overflowX ?? 'scroll',
      y: options.overflowY ?? 'scroll'
    },
    scrollbars: {
      autoHide: 'leave',
      autoHideDelay: 600,
      clickScroll: true
    }
  });
  const viewport = instance.elements().viewport;
  options.onReady?.(viewport);
  const handleScroll = () => options.onScroll?.(viewport.scrollTop);
  viewport.addEventListener('scroll', handleScroll, { passive: true });
  const resizeObserver = new ResizeObserver(() => {
    handleScroll();
  });
  resizeObserver.observe(viewport);
  requestAnimationFrame(() => {
    viewport.scrollTop = options.initialScrollTop ?? 0;
    handleScroll();
  });

  return {
    destroy() {
      resizeObserver.disconnect();
      viewport.removeEventListener('scroll', handleScroll);
      options.onReady?.(null);
      instance.destroy();
    }
  };
}
