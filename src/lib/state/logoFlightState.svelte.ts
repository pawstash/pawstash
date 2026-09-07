function solveCubicBezier(x1: number, y1: number, x2: number, y2: number) {
  return function (t: number): number {
    if (t <= 0) return 0;
    if (t >= 1) return 1;
    let u = t;
    for (let i = 0; i < 6; i++) {
      const currentX = 3 * (1 - u) * (1 - u) * u * x1 + 3 * (1 - u) * u * u * x2 + u * u * u;
      const dx = 3 * (1 - u) * (1 - u) * x1 + 6 * (1 - u) * u * (x2 - x1) + 3 * u * u * (1 - x2);
      if (Math.abs(currentX - t) < 1e-4) break;
      if (Math.abs(dx) < 1e-6) break;
      u -= (currentX - t) / dx;
    }
    u = Math.max(0, Math.min(1, u));
    return 3 * (1 - u) * (1 - u) * u * y1 + 3 * (1 - u) * u * u * y2 + u * u * u;
  };
}

const easeSpring = solveCubicBezier(0.16, 1, 0.3, 1);

export type FlightDirection = 'toHero' | 'toSidebar' | null;

class LogoFlightState {
  isFlying = $state(false);
  direction = $state<FlightDirection>(null);

  private sidebarEl: HTMLElement | null = null;
  private heroEl: HTMLElement | null = null;
  private flightEl: HTMLElement | null = null;
  private lastHeroRect: DOMRect | null = null;
  private rafId: number | null = null;

  registerFlightElement(el: HTMLElement | null) {
    this.flightEl = el;
  }

  registerSidebar(el: HTMLElement | null) {
    this.sidebarEl = el;
  }

  registerHero(el: HTMLElement | null) {
    this.heroEl = el;
    if (el) {
      this.lastHeroRect = el.getBoundingClientRect();
    }
  }

  unregisterHero() {
    if (this.heroEl) {
      this.lastHeroRect = this.heroEl.getBoundingClientRect();
    }
    this.heroEl = null;
  }

  cancel() {
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
    if (this.flightEl) {
      this.flightEl.style.display = 'none';
      this.flightEl.style.transform = '';
    }
    this.isFlying = false;
    this.direction = null;
  }

  flyToHero() {
    this.cancel();

    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      return;
    }

    if (!this.sidebarEl || !this.heroEl || !this.flightEl) return;

    const fromRect = this.sidebarEl.getBoundingClientRect();
    const toRect = this.heroEl.getBoundingClientRect();

    if (fromRect.width < 10 || fromRect.height < 10 || toRect.width < 10 || toRect.height < 10) return;

    const flight = this.flightEl;
    this.isFlying = true;
    this.direction = 'toHero';

    flight.style.display = 'block';
    flight.style.left = `${fromRect.left}px`;
    flight.style.top = `${fromRect.top}px`;
    flight.style.width = `${fromRect.width}px`;
    flight.style.height = `${fromRect.height}px`;
    flight.style.transformOrigin = 'top left';

    const startTime = performance.now();
    const duration = 360;

    const step = (now: number) => {
      const elapsed = now - startTime;
      const t = Math.min(1, elapsed / duration);
      const p = easeSpring(t);

      const currentTo = this.heroEl
        ? this.heroEl.getBoundingClientRect()
        : (this.lastHeroRect || toRect);

      const dx = currentTo.left - fromRect.left;
      const dy = currentTo.top - fromRect.top;
      const targetScaleX = currentTo.width / fromRect.width;
      const targetScaleY = currentTo.height / fromRect.height;

      const curDx = dx * p;
      const curDy = dy * p;
      const curScaleX = 1 + (targetScaleX - 1) * p;
      const curScaleY = 1 + (targetScaleY - 1) * p;

      flight.style.transform = `translate3d(${curDx}px, ${curDy}px, 0) scale(${curScaleX}, ${curScaleY})`;
      flight.style.filter = `drop-shadow(0 ${2 + 6 * p}px ${8 + 28 * p}px rgba(254, 184, 173, ${0.15 + 0.13 * p}))`;

      if (t < 1) {
        this.rafId = requestAnimationFrame(step);
      } else {
        flight.style.display = 'none';
        flight.style.transform = '';
        this.isFlying = false;
        this.direction = null;
        this.rafId = null;
        this.lastHeroRect = currentTo;
      }
    };

    this.rafId = requestAnimationFrame(step);
  }

  flyToSidebar() {
    this.cancel();

    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      return;
    }

    if (!this.sidebarEl || !this.flightEl || !this.lastHeroRect) return;

    const fromRect = this.lastHeroRect;
    const toRect = this.sidebarEl.getBoundingClientRect();

    if (fromRect.bottom < 50 || fromRect.top > window.innerHeight) {
      return;
    }

    if (toRect.width < 10 || toRect.height < 10 || fromRect.width < 10 || fromRect.height < 10) return;

    const flight = this.flightEl;
    this.isFlying = true;
    this.direction = 'toSidebar';

    flight.style.display = 'block';
    flight.style.left = `${fromRect.left}px`;
    flight.style.top = `${fromRect.top}px`;
    flight.style.width = `${fromRect.width}px`;
    flight.style.height = `${fromRect.height}px`;
    flight.style.transformOrigin = 'top left';

    const startTime = performance.now();
    const duration = 320;

    const step = (now: number) => {
      const elapsed = now - startTime;
      const t = Math.min(1, elapsed / duration);
      const p = easeSpring(t);

      const currentTo = this.sidebarEl
        ? this.sidebarEl.getBoundingClientRect()
        : toRect;

      const dx = currentTo.left - fromRect.left;
      const dy = currentTo.top - fromRect.top;
      const targetScaleX = currentTo.width / fromRect.width;
      const targetScaleY = currentTo.height / fromRect.height;

      const curDx = dx * p;
      const curDy = dy * p;
      const curScaleX = 1 + (targetScaleX - 1) * p;
      const curScaleY = 1 + (targetScaleY - 1) * p;

      flight.style.transform = `translate3d(${curDx}px, ${curDy}px, 0) scale(${curScaleX}, ${curScaleY})`;
      flight.style.filter = `drop-shadow(0 ${8 - 6 * p}px ${36 - 28 * p}px rgba(254, 184, 173, ${0.28 - 0.13 * p}))`;

      if (t < 1) {
        this.rafId = requestAnimationFrame(step);
      } else {
        flight.style.display = 'none';
        flight.style.transform = '';
        this.isFlying = false;
        this.direction = null;
        this.rafId = null;
      }
    };

    this.rafId = requestAnimationFrame(step);
  }
}

export const logoFlightState = new LogoFlightState();
