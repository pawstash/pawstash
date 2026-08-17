let _rippleStyle: HTMLStyleElement | null = null;

export function ripple(node: HTMLElement, enabled = true) {
  if (!enabled || typeof window === 'undefined') return { destroy() {} };

  if (!_rippleStyle) {
    _rippleStyle = document.createElement('style');
    _rippleStyle.textContent = `
      @keyframes _rpl-expand {
        to { transform: translate(-50%,-50%) scale(1); }
      }
      @keyframes _rpl-fade {
        to { opacity: 0; }
      }
    `;
    document.head.appendChild(_rippleStyle);
  }

  node.style.position = node.style.position || 'relative';
  node.style.overflow = 'hidden';

  function spawn(e: PointerEvent) {
    const rect = node.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const size = Math.hypot(
      Math.max(x, rect.width - x),
      Math.max(y, rect.height - y),
    ) * 2;

    const el = document.createElement('span');
    el.style.cssText = `
      position:absolute;left:${x}px;top:${y}px;
      width:${size}px;height:${size}px;border-radius:50%;
      background:var(--ripple-color, rgba(255, 255, 255, 0.15));opacity:0.6;pointer-events:none;
      transform:translate(-50%,-50%) scale(0);
      animation:_rpl-expand 0.45s cubic-bezier(0.2,0.9,0.3,1) forwards,
                _rpl-fade 0.4s 0.2s ease-out forwards;
    `;
    node.appendChild(el);
    el.addEventListener('animationend', (ev) => {
      if (ev.animationName === '_rpl-fade') el.remove();
    }, { once: true });
  }

  node.addEventListener('pointerdown', spawn);
  return {
    destroy() {
      node.removeEventListener('pointerdown', spawn);
    },
  };
}
