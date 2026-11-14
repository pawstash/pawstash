import { FLOATING_EDGE } from '$lib/utils/floating';

export type TooltipPlacement = 'top' | 'bottom' | 'left' | 'right';

type FloatingModule = typeof import('@floating-ui/dom');

let floatingPromise: Promise<FloatingModule> | null = null;

function loadFloating(): Promise<FloatingModule> {
  return (floatingPromise ??= import('@floating-ui/dom'));
}

export type TooltipOptions = {
  text?: string;
  placement?: TooltipPlacement;
  delay?: number;
  open?: boolean;
  variant?: 'default' | 'accent';
};

export type TooltipParam = string | undefined | null | TooltipOptions;

const GAP = 6;
const HIDE_MS = 120;
const WARM_MS = 600;
const ALLOW_SIDE_PLACEMENTS = false;

const FALLBACKS: Record<TooltipPlacement, TooltipPlacement[]> = {
  bottom: ['top', 'right', 'left'],
  top: ['bottom', 'right', 'left'],
  left: ['right', 'bottom', 'top'],
  right: ['left', 'bottom', 'top']
};

let lastHiddenAt = 0;

const canHover = () => typeof window !== 'undefined' && window.matchMedia('(hover: hover)').matches;

function normalizePlacement(placement: TooltipPlacement): TooltipPlacement {
  if (ALLOW_SIDE_PLACEMENTS) return placement;
  return placement === 'top' ? 'top' : 'bottom';
}

function parseParam(param: TooltipParam, defaultDelay: number) {
  const options: TooltipOptions = typeof param === 'string' ? { text: param } : (param ?? {});
  return {
    text: options.text?.trim() || undefined,
    placement: normalizePlacement(options.placement ?? 'bottom'),
    delay: options.delay ?? defaultDelay,
    open: options.open,
    variant: options.variant ?? ('default' as const)
  };
}

type ActiveTooltip = {
  token: symbol;
  anchor: HTMLElement;
  text: string;
  variant: 'default' | 'accent';
  placement: TooltipPlacement;
};

let tooltipEl: HTMLElement | null = null;
let active: ActiveTooltip | null = null;
let stopAutoUpdate: (() => void) | undefined;
let hideTimer: ReturnType<typeof setTimeout> | undefined;
let placed = false;

function element() {
  if (!tooltipEl) {
    tooltipEl = document.createElement('div');
    tooltipEl.className = 'app-tooltip';
    tooltipEl.setAttribute('role', 'tooltip');
    document.body.appendChild(tooltipEl);
  }
  return tooltipEl;
}

async function place(token: symbol) {
  const current = active;
  if (!current || current.token !== token) return;
  const { computePosition, flip, shift, offset } = await loadFloating();
  if (active?.token !== token) return;

  const fallbacks = ALLOW_SIDE_PLACEMENTS
    ? FALLBACKS[current.placement]
    : FALLBACKS[current.placement].filter((item) => item === 'top' || item === 'bottom');
  const { x, y, placement } = await computePosition(current.anchor, element(), {
    placement: current.placement,
    strategy: 'fixed',
    middleware: [
      offset(GAP),
      flip({ fallbackPlacements: fallbacks, padding: FLOATING_EDGE }),
      shift({ padding: FLOATING_EDGE })
    ]
  });
  if (active?.token !== token) return;

  const el = element();
  el.dataset.side = placement.split('-')[0];
  el.style.left = `${Math.round(x)}px`;
  el.style.top = `${Math.round(y)}px`;
  placed = true;
}

function openTooltip(token: symbol, anchor: HTMLElement, config: ReturnType<typeof parseParam>) {
  clearTimeout(hideTimer);
  hideTimer = undefined;
  stopAutoUpdate?.();
  stopAutoUpdate = undefined;

  const el = element();
  const wasPlaced = placed;
  active = {
    token,
    anchor,
    text: config.text ?? '',
    variant: config.variant,
    placement: config.placement
  };
  el.textContent = config.text ?? '';
  el.classList.toggle('is-accent', config.variant === 'accent');
  el.classList.toggle('is-moving', wasPlaced);

  void place(token).then(async () => {
    if (active?.token !== token) return;
    const { autoUpdate } = await loadFloating();
    if (active?.token !== token) return;
    stopAutoUpdate?.();
    stopAutoUpdate = autoUpdate(anchor, el, () => void place(token).catch(() => {}));
    requestAnimationFrame(() => {
      if (active?.token === token && !hideTimer) el.classList.add('is-visible');
    });
  }).catch(() => {
    if (active?.token === token) closeTooltip(token);
  });
}

function closeTooltip(token: symbol) {
  if (active?.token !== token) return;
  stopAutoUpdate?.();
  stopAutoUpdate = undefined;

  const el = tooltipEl;
  if (!el) return;
  if (el.classList.contains('is-visible')) lastHiddenAt = performance.now();
  el.classList.remove('is-visible');
  clearTimeout(hideTimer);
  hideTimer = setTimeout(() => {
    if (active?.token === token) active = null;
    hideTimer = undefined;
  }, HIDE_MS);
}

function updateTooltip(token: symbol, config: ReturnType<typeof parseParam>) {
  if (active?.token !== token) return;
  active = {
    ...active,
    text: config.text ?? '',
    variant: config.variant,
    placement: config.placement
  };
  const el = element();
  el.textContent = config.text ?? '';
  el.classList.toggle('is-accent', config.variant === 'accent');
  el.classList.toggle('is-moving', placed);
  void place(token).catch(() => {});
}

export function tooltip(node: HTMLElement, param: TooltipParam, defaultDelay = 400) {
  const token = Symbol('tooltip');
  let showTimer: ReturnType<typeof setTimeout> | undefined;
  let config = parseParam(param, defaultDelay);

  function scheduleShow() {
    if (!config.text) return;
    clearTimeout(showTimer);
    const warm = performance.now() - lastHiddenAt < WARM_MS;
    showTimer = setTimeout(() => {
      if (config.text) openTooltip(token, node, config);
    }, warm || config.open ? 0 : config.delay);
  }

  function hide() {
    clearTimeout(showTimer);
    showTimer = undefined;
    closeTooltip(token);
  }

  const onEnter = () => {
    if (config.open !== undefined || !canHover()) return;
    void loadFloating();
    scheduleShow();
  };
  const onFocus = () => {
    if (config.open !== undefined || !node.matches(':focus-visible')) return;
    void loadFloating();
    scheduleShow();
  };
  const onLeave = () => {
    if (config.open === undefined) hide();
  };

  node.addEventListener('mouseenter', onEnter);
  node.addEventListener('mouseleave', onLeave);
  node.addEventListener('focusin', onFocus);
  node.addEventListener('focusout', onLeave);
  node.addEventListener('click', onLeave);
  node.addEventListener('pointerdown', onLeave);

  if (config.open) scheduleShow();

  return {
    update(next: TooltipParam) {
      config = parseParam(next, defaultDelay);
      if (!config.text) {
        hide();
        return;
      }
      if (active?.token === token) updateTooltip(token, config);
      if (config.open === true && active?.token !== token) scheduleShow();
      else if (config.open === false) hide();
    },
    destroy() {
      clearTimeout(showTimer);
      node.removeEventListener('mouseenter', onEnter);
      node.removeEventListener('mouseleave', onLeave);
      node.removeEventListener('focusin', onFocus);
      node.removeEventListener('focusout', onLeave);
      node.removeEventListener('click', onLeave);
      node.removeEventListener('pointerdown', onLeave);
      if (active?.token !== token) return;
      active = null;
      clearTimeout(hideTimer);
      hideTimer = undefined;
      stopAutoUpdate?.();
      stopAutoUpdate = undefined;
      tooltipEl?.classList.remove('is-visible');
    }
  };
}
