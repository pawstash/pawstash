export function tooltip(node: HTMLElement, text: string | undefined) {
  let tooltipEl: HTMLElement | null = null;
  let currentText = text;

  function show() {
    if (!currentText) return;

    tooltipEl = document.createElement('div');
    tooltipEl.textContent = currentText;
    tooltipEl.className = 'fixed z-[10001] px-2 py-1 text-[11px] font-medium font-outfit text-gray-200 bg-gray-900/95 border border-white/10 rounded-md shadow-xl backdrop-blur-md pointer-events-none transition-opacity duration-150 opacity-0';

    document.body.appendChild(tooltipEl);

    const rect = node.getBoundingClientRect();
    const tooltipRect = tooltipEl.getBoundingClientRect();

    let top = rect.bottom + 6;
    let left = rect.left + (rect.width - tooltipRect.width) / 2;

    if (top + tooltipRect.height > window.innerHeight) {
      top = rect.top - tooltipRect.height - 6;
    }
    if (left < 6) left = 6;
    if (left + tooltipRect.width > window.innerWidth - 6) {
      left = window.innerWidth - tooltipRect.width - 6;
    }

    tooltipEl.style.top = `${top}px`;
    tooltipEl.style.left = `${left}px`;

    requestAnimationFrame(() => {
      if (tooltipEl) tooltipEl.style.opacity = '1';
    });
  }

  function hide() {
    if (tooltipEl) {
      tooltipEl.remove();
      tooltipEl = null;
    }
  }

  node.addEventListener('mouseenter', show);
  node.addEventListener('mouseleave', hide);
  node.addEventListener('click', hide);

  return {
    update(newText: string | undefined) {
      currentText = newText;
      if (tooltipEl && currentText) {
        tooltipEl.textContent = currentText;
      }
    },
    destroy() {
      hide();
      node.removeEventListener('mouseenter', show);
      node.removeEventListener('mouseleave', hide);
      node.removeEventListener('click', hide);
    }
  };
}
