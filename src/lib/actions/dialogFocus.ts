export interface DialogFocusOptions {
  onEscape?: () => void;
}

const FOCUSABLE_SELECTOR = [
  'button:not([disabled])',
  '[href]',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])'
].join(',');

export function dialogFocus(node: HTMLElement, options: DialogFocusOptions = {}) {
  let current = options;

  const previouslyFocused =
    document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const previousBodyOverflow = document.body.style.overflow;
  document.body.style.overflow = 'hidden';

  function focusableElements() {
    return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
      (element) =>
        element.tabIndex >= 0 &&
        !element.hasAttribute('hidden') &&
        element.getAttribute('aria-hidden') !== 'true'
    );
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      current.onEscape?.();
      return;
    }
    if (event.key !== 'Tab') return;

    const elements = focusableElements();
    if (elements.length === 0) {
      event.preventDefault();
      node.focus();
      return;
    }

    const first = elements[0];
    const last = elements[elements.length - 1];
    if (event.shiftKey && (document.activeElement === first || document.activeElement === node)) {
      event.preventDefault();
      last.focus();
    } else if (
      !event.shiftKey &&
      (document.activeElement === last || !node.contains(document.activeElement))
    ) {
      event.preventDefault();
      first.focus();
    }
  }

  node.addEventListener('keydown', handleKeydown);
  const initialFocus = requestAnimationFrame(() => {
    (focusableElements()[0] ?? node).focus();
  });

  return {
    update(next: DialogFocusOptions = {}) {
      current = next;
    },
    destroy() {
      cancelAnimationFrame(initialFocus);
      node.removeEventListener('keydown', handleKeydown);
      document.body.style.overflow = previousBodyOverflow;
      if (previouslyFocused?.isConnected) {
        requestAnimationFrame(() => previouslyFocused.focus());
      }
    }
  };
}
