let active: HTMLVideoElement | null = null;

/** One hover preview decodes at a time: a grid of them is a grid of decoders. */
export function claimHoverVideo(element: HTMLVideoElement) {
  if (active && active !== element) {
    release(active);
  }
  active = element;
}

export function releaseHoverVideo(element: HTMLVideoElement) {
  if (active === element) active = null;
  release(element);
}

function release(element: HTMLVideoElement) {
  element.pause();
  element.removeAttribute('src');
  element.load();
}
