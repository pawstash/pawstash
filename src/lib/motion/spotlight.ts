export interface SpotlightOptions {
  glowColor?: string;
  radius?: number;
  disabled?: boolean;
}

export function spotlight(node: HTMLElement, options: SpotlightOptions = {}) {
  let { glowColor = 'rgba(255, 255, 255, 0.08)', radius = 120, disabled = false } = options;

  node.style.position = node.style.position || 'relative';

  function handlePointerMove(e: MouseEvent) {
    if (disabled) return;
    const rect = node.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    node.style.backgroundImage = `radial-gradient(circle ${radius}px at ${x}px ${y}px, ${glowColor}, transparent 70%)`;
  }

  function handlePointerLeave() {
    node.style.backgroundImage = 'none';
  }

  node.addEventListener('pointermove', handlePointerMove);
  node.addEventListener('pointerleave', handlePointerLeave);

  return {
    update(newOptions: SpotlightOptions = {}) {
      glowColor = newOptions.glowColor ?? 'rgba(255, 255, 255, 0.08)';
      radius = newOptions.radius ?? 120;
      disabled = newOptions.disabled ?? false;
    },
    destroy() {
      node.removeEventListener('pointermove', handlePointerMove);
      node.removeEventListener('pointerleave', handlePointerLeave);
      node.style.backgroundImage = 'none';
    }
  };
}
