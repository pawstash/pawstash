export function portal(node: HTMLElement, target: string | HTMLElement = 'body') {
  let targetEl: HTMLElement | null = null;

  if (typeof target === 'string') {
    // Menus and other overlays must live inside the fullscreen element, otherwise the
    // top layer paints them underneath it and they become invisible.
    const fullscreen = document.fullscreenElement;
    targetEl = target === 'body' && fullscreen instanceof HTMLElement
      ? fullscreen
      : document.querySelector(target);
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
