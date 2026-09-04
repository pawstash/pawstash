import { computePosition, autoUpdate, flip, shift, offset } from '@floating-ui/dom';

export type TooltipPlacement = 'top' | 'bottom' | 'left' | 'right';

export type TooltipOptions = {
  text?: string;
  placement?: TooltipPlacement;
  delay?: number;
};

export type TooltipParam = string | undefined | null | TooltipOptions;

const OPPOSITE_SIDES: Record<TooltipPlacement, readonly TooltipPlacement[]> = {
  top: ['bottom', 'right', 'left'],
  bottom: ['top', 'right', 'left'],
  left: ['right', 'top', 'bottom'],
  right: ['left', 'top', 'bottom']
};

const SIDE_TO_ARROW: Record<string, TooltipPlacement> = {
  top: 'bottom',
  bottom: 'top',
  left: 'right',
  right: 'left'
};

function parseParam(param: TooltipParam, defaultDelay: number) {
  if (typeof param === 'string') {
    return { text: param.trim() || undefined, placement: undefined, delay: defaultDelay };
  }
  if (param && typeof param === 'object') {
    return {
      text: param.text?.trim() || undefined,
      placement: param.placement,
      delay: param.delay ?? defaultDelay
    };
  }
  return { text: undefined, placement: undefined, delay: defaultDelay };
}

export function tooltip(node: HTMLElement, param: TooltipParam, defaultDelay = 120) {
  let activeEl: HTMLElement | null = null;
  let textEl: HTMLElement | null = null;
  let showTimer: ReturnType<typeof setTimeout> | undefined;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let cleanupAutoUpdate: (() => void) | undefined;

  let config = parseParam(param, defaultDelay);

  async function updatePosition() {
    if (!activeEl || !config.text) return;

    const initialPlacement = config.placement || 'top';
    const fallbackPlacements = OPPOSITE_SIDES[initialPlacement] ?? OPPOSITE_SIDES.top;

    const { x, y, placement } = await computePosition(node, activeEl, {
      placement: initialPlacement,
      strategy: 'fixed',
      middleware: [
        offset(6),
        flip({
          fallbackPlacements: [...fallbackPlacements],
          padding: 8
        }),
        shift({
          padding: 8
        })
      ]
    });

    if (!activeEl) return;

    const side = placement.split('-')[0];
    const arrowSide = SIDE_TO_ARROW[side] ?? 'bottom';
    const nodeRect = node.getBoundingClientRect();
    const tooltipRect = activeEl.getBoundingClientRect();

    if (arrowSide === 'top' || arrowSide === 'bottom') {
      const relX = nodeRect.left + nodeRect.width / 2 - x;
      const clampedX = Math.max(14, Math.min(tooltipRect.width - 14, relX));
      activeEl.style.setProperty('--arrow-x', `${Math.round(clampedX)}px`);
    } else {
      const relY = nodeRect.top + nodeRect.height / 2 - y;
      const clampedY = Math.max(10, Math.min(tooltipRect.height - 10, relY));
      activeEl.style.setProperty('--arrow-y', `${Math.round(clampedY)}px`);
    }

    activeEl.setAttribute('data-side', arrowSide);
    activeEl.style.left = `${Math.round(x)}px`;
    activeEl.style.top = `${Math.round(y)}px`;
  }

  function show() {
    if (!config.text) return;
    if (showTimer) clearTimeout(showTimer);

    showTimer = setTimeout(async () => {
      const textToShow = config.text;
      if (!textToShow) return;

      if (hideTimer) {
        clearTimeout(hideTimer);
        hideTimer = undefined;
      }

      if (!activeEl) {
        activeEl = document.createElement('div');
        activeEl.className = 'app-tooltip';

        textEl = document.createElement('span');
        textEl.className = 'app-tooltip-text';

        const arrow = document.createElement('div');
        arrow.className = 'app-tooltip-arrow';
        arrow.innerHTML = '<svg viewBox="0 0 14 5" aria-hidden="true"><path d="M 0 0 L 5.6 4.1 Q 7 5 8.4 4.1 L 14 0 Z" /></svg>';

        activeEl.append(textEl, arrow);
        document.body.appendChild(activeEl);
      }

      if (textEl) {
        textEl.textContent = textToShow;
      }

      await updatePosition();
      if (!activeEl) return;

      cleanupAutoUpdate?.();
      cleanupAutoUpdate = autoUpdate(node, activeEl, () => {
        void updatePosition();
      });

      requestAnimationFrame(() => {
        activeEl?.classList.add('is-visible');
      });
    }, config.delay);
  }

  function hide() {
    if (showTimer) {
      clearTimeout(showTimer);
      showTimer = undefined;
    }
    if (cleanupAutoUpdate) {
      cleanupAutoUpdate();
      cleanupAutoUpdate = undefined;
    }
    if (activeEl) {
      const el = activeEl;
      el.classList.remove('is-visible');
      if (hideTimer) clearTimeout(hideTimer);
      hideTimer = setTimeout(() => {
        el.remove();
        if (activeEl === el) {
          activeEl = null;
          textEl = null;
        }
        hideTimer = undefined;
      }, 130);
    }
  }

  node.addEventListener('mouseenter', show);
  node.addEventListener('mouseleave', hide);
  node.addEventListener('click', hide);
  node.addEventListener('pointerdown', hide);

  return {
    update(newParam: TooltipParam) {
      config = parseParam(newParam, defaultDelay);
      if (!config.text) {
        hide();
        return;
      }
      if (textEl) {
        textEl.textContent = config.text;
      }
      if (activeEl) {
        void updatePosition();
      }
    },
    destroy() {
      hide();
      if (hideTimer) {
        clearTimeout(hideTimer);
        hideTimer = undefined;
      }
      if (activeEl) {
        activeEl.remove();
        activeEl = null;
        textEl = null;
      }
      node.removeEventListener('mouseenter', show);
      node.removeEventListener('mouseleave', hide);
      node.removeEventListener('click', hide);
      node.removeEventListener('pointerdown', hide);
    }
  };
}
