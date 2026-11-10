export interface MarqueeOptions {
  speed?: number;
  gap?: number;
  delay?: number;
  active?: boolean;
  disabled?: boolean;
  trigger?: string;
}

const DEFAULTS = {
  speed: 32,
  gap: 24,
  delay: 1200,
  active: false,
  disabled: false
};

export function marquee(node: HTMLElement, initial: MarqueeOptions = {}) {
  let options = { ...DEFAULTS, ...initial };
  let track: HTMLSpanElement | null = null;
  let originals: ChildNode[] = [];
  let frame = 0;
  let startTime = 0;
  let distance = 0;
  let cycle = 0;
  let hovered = false;
  let focused = false;
  let resizeTimer: ReturnType<typeof setTimeout> | undefined;

  const saved = {
    overflow: node.style.overflow,
    whiteSpace: node.style.whiteSpace,
    textOverflow: node.style.textOverflow
  };
  const reduced =
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  node.style.overflow = 'hidden';
  node.style.whiteSpace = 'nowrap';
  node.style.textOverflow = 'ellipsis';

  function wants() {
    return !reduced && !options.disabled && (hovered || focused || options.active);
  }

  function build(): boolean {
    if (track) return true;
    if (!wants()) return false;
    if (node.clientWidth < 8) return false;

    originals = Array.from(node.childNodes);
    if (originals.length === 0) return false;

    const element = document.createElement('span');
    element.className = 'marquee-track';
    element.style.cssText =
      'display:inline-flex;align-items:baseline;width:max-content;will-change:transform;';
    for (const child of originals) element.appendChild(child);
    node.appendChild(element);

    const width = element.getBoundingClientRect().width;
    if (width <= node.clientWidth + 1) {
      for (const child of originals) node.appendChild(child);
      element.remove();
      originals = [];
      return false;
    }

    const clone = element.cloneNode(true) as HTMLSpanElement;
    clone.setAttribute('aria-hidden', 'true');
    clone.style.marginLeft = `${options.gap}px`;
    element.appendChild(clone);

    node.style.textOverflow = 'clip';
    track = element;
    distance = width + options.gap;
    cycle = (distance / Math.max(1, options.speed)) * 1000;
    return true;
  }

  function destroyTrack() {
    if (!track) return;
    for (const child of originals) node.appendChild(child);
    track.remove();
    track = null;
    originals = [];
    node.style.textOverflow = 'ellipsis';
  }

  function stop() {
    cancelAnimationFrame(frame);
    frame = 0;
    if (track) track.style.transform = '';
  }

  function step(now: number) {
    frame = 0;
    if (!track || !wants()) return;
    const phase = (now - startTime) % (cycle + options.delay);
    const progress = phase < cycle ? phase / cycle : 0;
    track.style.transform = progress === 0 ? '' : `translateX(${(-progress * distance).toFixed(2)}px)`;
    frame = requestAnimationFrame(step);
  }

  function start() {
    if (frame || !track) return;
    startTime = performance.now();
    frame = requestAnimationFrame(step);
  }

  function refresh() {
    if (wants()) {
      if (build()) start();
      return;
    }
    stop();
    destroyTrack();
  }

  const onEnter = () => {
    hovered = true;
    refresh();
  };
  const onLeave = () => {
    hovered = false;
    refresh();
  };
  const onFocusIn = () => {
    focused = true;
    refresh();
  };
  const onFocusOut = () => {
    focused = false;
    refresh();
  };

  const trigger =
    (initial.trigger ? node.closest<HTMLElement>(initial.trigger) : null) ?? node;

  trigger.addEventListener('pointerenter', onEnter);
  trigger.addEventListener('pointerleave', onLeave);
  trigger.addEventListener('focusin', onFocusIn);
  trigger.addEventListener('focusout', onFocusOut);

  const observer = new ResizeObserver(() => {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
      stop();
      destroyTrack();
      refresh();
    }, 120);
  });
  observer.observe(node);

  refresh();

  return {
    update(next: MarqueeOptions = {}) {
      const gapChanged = next.gap !== undefined && next.gap !== options.gap;
      const speedChanged = next.speed !== undefined && next.speed !== options.speed;
      options = { ...options, ...next };
      if (gapChanged) {
        stop();
        destroyTrack();
      } else if (speedChanged && track) {
        cycle = (distance / Math.max(1, options.speed)) * 1000;
        startTime = performance.now();
      }
      refresh();
    },
    destroy() {
      stop();
      destroyTrack();
      observer.disconnect();
      clearTimeout(resizeTimer);
      trigger.removeEventListener('pointerenter', onEnter);
      trigger.removeEventListener('pointerleave', onLeave);
      trigger.removeEventListener('focusin', onFocusIn);
      trigger.removeEventListener('focusout', onFocusOut);
      node.style.overflow = saved.overflow;
      node.style.whiteSpace = saved.whiteSpace;
      node.style.textOverflow = saved.textOverflow;
    }
  };
}
