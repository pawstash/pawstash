import { getCurrentWindow } from '@tauri-apps/api/window';
import { configState } from '$lib/state/configState.svelte';
import { apiHideToTray } from '$lib/utils/ipc';
import { matchesShortcut } from '$lib/utils/shortcuts';

export function isTextEditable(target: EventTarget | null): boolean {
  if (!target || !(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName.toUpperCase();
  if (tag === 'TEXTAREA') return true;
  if (tag === 'INPUT') {
    const type = (target as HTMLInputElement).type?.toLowerCase() || 'text';
    return !['range', 'checkbox', 'radio', 'button', 'submit', 'reset', 'file', 'color', 'image'].includes(type);
  }
  return false;
}

export function triggerPanic(): void {
  try {
    document.querySelectorAll('video, audio').forEach((el) => {
      try {
        (el as HTMLMediaElement).pause();
      } catch {}
    });
  } catch {}

  try {
    if (document.fullscreenElement) {
      void document.exitFullscreen().catch(() => {});
    }
  } catch {}

  try {
    void apiHideToTray();
  } catch {}
  try {
    void getCurrentWindow().hide();
  } catch {}
}

export function handleGlobalPanicKey(event: KeyboardEvent): boolean {
  const enabled = configState.settings.panic_button_enabled ?? configState.settings.boss_key_enabled ?? true;
  if (!enabled) return false;
  const shortcut = (configState.settings.panic_button_shortcut || configState.settings.boss_key_shortcut || 'H').trim();
  if (!shortcut) return false;

  const target = event.target;
  if (target instanceof Element && target.closest('.shortcut-box')) return false;

  if (matchesShortcut(event, shortcut)) {
    const hasModifier = event.altKey || event.ctrlKey || event.metaKey;
    const isEscOrFn = event.key === 'Escape' || /^f\d{1,2}$/i.test(event.key || '');
    if (isTextEditable(target) && !hasModifier && !isEscOrFn) {
      return false;
    }

    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();

    triggerPanic();
    return true;
  }

  return false;
}

export function panicCapture(node: HTMLElement): { destroy: () => void } {
  const onKey = (e: KeyboardEvent) => handleGlobalPanicKey(e);
  const releaseMediaFocus = () => {
    setTimeout(() => {
      if (document.activeElement === node || (document.activeElement && node.contains(document.activeElement))) {
        (document.activeElement as HTMLElement)?.blur();
      }
    }, 10);
  };

  node.addEventListener('keydown', onKey, { capture: true });
  node.addEventListener('keyup', onKey, { capture: true });
  node.addEventListener('pointerup', releaseMediaFocus);
  node.addEventListener('click', releaseMediaFocus);
  node.addEventListener('focus', releaseMediaFocus);
  node.addEventListener('focusin', releaseMediaFocus);

  return {
    destroy() {
      node.removeEventListener('keydown', onKey, { capture: true });
      node.removeEventListener('keyup', onKey, { capture: true });
      node.removeEventListener('pointerup', releaseMediaFocus);
      node.removeEventListener('click', releaseMediaFocus);
      node.removeEventListener('focus', releaseMediaFocus);
      node.removeEventListener('focusin', releaseMediaFocus);
    }
  };
}

let isInitialized = false;
export function initPanicListener(): () => void {
  if (typeof window === 'undefined' || isInitialized) return () => {};
  isInitialized = true;

  const releaseIfMedia = (e: Event) => {
    const target = e.target;
    if (target instanceof HTMLVideoElement || target instanceof HTMLAudioElement) {
      setTimeout(() => {
        target.blur();
      }, 10);
    }
  };

  window.addEventListener('keydown', handleGlobalPanicKey, { capture: true, passive: false });
  document.addEventListener('keydown', handleGlobalPanicKey, { capture: true, passive: false });
  document.documentElement?.addEventListener('keydown', handleGlobalPanicKey, { capture: true, passive: false });

  document.addEventListener('focusin', releaseIfMedia, { capture: true });
  document.addEventListener('pointerup', releaseIfMedia, { capture: true });
  document.addEventListener('click', releaseIfMedia, { capture: true });

  return () => {
    window.removeEventListener('keydown', handleGlobalPanicKey, { capture: true });
    document.removeEventListener('keydown', handleGlobalPanicKey, { capture: true });
    document.documentElement?.removeEventListener('keydown', handleGlobalPanicKey, { capture: true });
    document.removeEventListener('focusin', releaseIfMedia, { capture: true });
    document.removeEventListener('pointerup', releaseIfMedia, { capture: true });
    document.removeEventListener('click', releaseIfMedia, { capture: true });
    isInitialized = false;
  };
}

if (typeof window !== 'undefined') {
  initPanicListener();
}
