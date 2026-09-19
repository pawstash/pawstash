const ENTER_STEP = 22;
const ENTER_MAX_DELAY = 420;
const ENTER_DURATION = 300;
const SKELETON_FADE = 220;

export class GridHandoff {
  skeletonMounted = $state(false);
  skeletonFading = $state(false);
  entering = $state(false);

  #awaitingContent = false;
  #fadeTimer: ReturnType<typeof setTimeout> | undefined;
  #enterTimer: ReturnType<typeof setTimeout> | undefined;

  sync(hasContent: boolean, loading: boolean) {
    if (!hasContent && loading) {
      clearTimeout(this.#fadeTimer);
      this.#awaitingContent = true;
      this.skeletonMounted = true;
      this.skeletonFading = false;
      return;
    }

    if (hasContent && this.#awaitingContent) {
      this.#awaitingContent = false;
      this.skeletonFading = true;
      this.entering = true;
      clearTimeout(this.#fadeTimer);
      clearTimeout(this.#enterTimer);
      this.#fadeTimer = setTimeout(() => {
        this.skeletonMounted = false;
        this.skeletonFading = false;
      }, SKELETON_FADE);
      this.#enterTimer = setTimeout(() => {
        this.entering = false;
      }, ENTER_MAX_DELAY + ENTER_DURATION + 80);
      return;
    }

    if (!hasContent && !loading) {
      this.#awaitingContent = false;
      this.skeletonMounted = false;
      this.skeletonFading = false;
    }
  }

  delayFor(index: number): number | null {
    if (!this.entering) return null;
    return Math.min(index * ENTER_STEP, ENTER_MAX_DELAY);
  }

  destroy() {
    clearTimeout(this.#fadeTimer);
    clearTimeout(this.#enterTimer);
  }
}
