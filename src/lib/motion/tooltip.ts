import { computePosition, autoUpdate, flip, shift, offset } from '@floating-ui/dom';

export type TooltipPlacement = 'top' | 'bottom' | 'left' | 'right';

export type TooltipOptions = {
  text?: string;
  placement?: TooltipPlacement;
  delay?: number;
  open?: boolean;
  variant?: 'default' | 'accent';
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
    return {
      text: param.trim() || undefined,
      placement: undefined,
      delay: defaultDelay,
      open: undefined as boolean | undefined,
      variant: 'default' as 'default' | 'accent'
    };
  }
  if (param && typeof param === 'object') {
    return {
      text: param.text?.trim() || undefined,
      placement: param.placement,
      delay: param.delay ?? defaultDelay,
      open: param.open,
      variant: param.variant ?? ('default' as const)
    };
  }
  return {
    text: undefined,
    placement: undefined,
    delay: defaultDelay,
    open: undefined as boolean | undefined,
    variant: 'default' as 'default' | 'accent'
  };
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
      activeEl.classList.toggle('is-accent', config.variant === 'accent');

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

  function showOnTrigger() {
    if (config.open === undefined) show();
  }

  function hideOnTrigger() {
    if (config.open === undefined) hide();
  }

  node.addEventListener('mouseenter', showOnTrigger);
  node.addEventListener('mouseleave', hideOnTrigger);
  node.addEventListener('focusin', showOnTrigger);
  node.addEventListener('focusout', hideOnTrigger);
  node.addEventListener('click', hideOnTrigger);
  node.addEventListener('pointerdown', hideOnTrigger);

  if (config.open) show();

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
      activeEl?.classList.toggle('is-accent', config.variant === 'accent');
      if (activeEl) {
        void updatePosition();
      }
      if (config.open === true) {
        if (!activeEl || hideTimer) show();
      } else if (config.open === false) {
        hide();
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
      node.removeEventListener('mouseenter', showOnTrigger);
      node.removeEventListener('mouseleave', hideOnTrigger);
      node.removeEventListener('focusin', showOnTrigger);
      node.removeEventListener('focusout', hideOnTrigger);
      node.removeEventListener('click', hideOnTrigger);
      node.removeEventListener('pointerdown', hideOnTrigger);
    }
  };
}
