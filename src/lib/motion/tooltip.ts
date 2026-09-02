export function tooltip(node: HTMLElement, text: string | undefined, delay = 120) {
  let tooltipEl: HTMLElement | null = null;
  let currentText = text;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function show() {
    if (!currentText) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      const textToShow = currentText;
      if (!textToShow) return;

      if (!tooltipEl) {
        tooltipEl = document.createElement('div');
        tooltipEl.className = 'app-tooltip';
        document.body.appendChild(tooltipEl);
      }

      tooltipEl.textContent = textToShow;

      const rect = node.getBoundingClientRect();
      const tooltipRect = tooltipEl.getBoundingClientRect();

      let top = rect.bottom + 6;
      let left = rect.left + (rect.width - tooltipRect.width) / 2;

      if (top + tooltipRect.height > window.innerHeight - 8) {
        top = rect.top - tooltipRect.height - 6;
      }
      if (top < 6) top = 6;
      if (left < 6) left = 6;
      if (left + tooltipRect.width > window.innerWidth - 6) {
        left = window.innerWidth - tooltipRect.width - 6;
      }

      tooltipEl.style.top = `${Math.round(top)}px`;
      tooltipEl.style.left = `${Math.round(left)}px`;

      requestAnimationFrame(() => {
        if (tooltipEl) {
          tooltipEl.classList.add('is-visible');
        }
      });
    }, delay);
  }

  function hide() {
    if (timer) {
      clearTimeout(timer);
      timer = undefined;
    }
    if (tooltipEl) {
      tooltipEl.remove();
      tooltipEl = null;
    }
  }

  node.addEventListener('mouseenter', show);
  node.addEventListener('mouseleave', hide);
  node.addEventListener('click', hide);
  node.addEventListener('pointerdown', hide);

  return {
    update(newText: string | undefined) {
      currentText = newText;
      if (!currentText) {
        hide();
        return;
      }
      if (tooltipEl) {
        tooltipEl.textContent = currentText;
      }
    },
    destroy() {
      hide();
      node.removeEventListener('mouseenter', show);
      node.removeEventListener('mouseleave', hide);
      node.removeEventListener('click', hide);
      node.removeEventListener('pointerdown', hide);
    }
  };
}
