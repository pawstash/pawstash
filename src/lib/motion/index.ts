import { ripple } from './ripple';
import { spotlight, type SpotlightOptions } from './spotlight';
import { tooltip } from './tooltip';
import type { MotionOptions, MotionPreset } from './types';

export { ripple, spotlight, tooltip };
export type { MotionOptions, MotionPreset, SpotlightOptions };

export function motion(node: HTMLElement, options: MotionPreset | MotionOptions = {}) {
  const opts: MotionOptions = typeof options === 'string'
    ? getPresetOptions(options)
    : options;

  if (opts.disabled) return { destroy() {} };

  const destroyFns: Array<() => void> = [];

  if (opts.ripple !== false) {
    const r = ripple(node, true);
    if (r.destroy) destroyFns.push(r.destroy);
  }

  if (opts.spotlight) {
    const s = spotlight(node, { glowColor: opts.glowColor });
    if (s.destroy) destroyFns.push(s.destroy);
  }

  return {
    destroy() {
      destroyFns.forEach(fn => fn());
    }
  };
}

function getPresetOptions(preset: MotionPreset): MotionOptions {
  switch (preset) {
    case 'window-control':
      return { ripple: true, spotlight: false };
    case 'sidebar-item':
      return { ripple: true, spotlight: true };
    case 'button':
      return { ripple: true, spotlight: true };
    case 'card':
      return { ripple: false, spotlight: true };
    case 'tab':
      return { ripple: true, spotlight: false };
    default:
      return { ripple: true, spotlight: false };
  }
}
