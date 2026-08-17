export function portal(node: HTMLElement, target: string | HTMLElement = 'body') {
  let targetEl: HTMLElement | null = null;

  if (typeof target === 'string') {
    targetEl = document.querySelector(target);
  } else {
    targetEl = target;
  }

  if (targetEl) {
    targetEl.appendChild(node);
  }

  return {
    destroy() {
      if (node.parentNode) {
        node.parentNode.removeChild(node);
      }
    }
  };
}
