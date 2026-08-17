import type { ThemeTokens, FontSizeScale, RadiusScale, SurfaceStyle, AccentColor, MotionSpeed } from './tokens';
import { FONT_SCALE_MAP, RADIUS_SCALE_MAP, ACCENT_COLOR_MAP, MOTION_SPEED_MAP } from './tokens';

export class ThemeState {
  tokens = $state<ThemeTokens>({
    fontScale: 'standard',
    radiusScale: 'smooth',
    surfaceStyle: 'glass',
    accent: 'rose',
    motionSpeed: 'smooth',
    backdropBlurPx: 24,
    borderWidthPx: 1,
    titlebarHeightPx: 30,
    sidebarWidthPx: 208
  });

  reset() {
    Object.assign(this.tokens, {
      fontScale: 'standard',
      radiusScale: 'smooth',
      surfaceStyle: 'glass',
      accent: 'rose',
      motionSpeed: 'smooth',
      backdropBlurPx: 24,
      borderWidthPx: 1,
      titlebarHeightPx: 30,
      sidebarWidthPx: 208
    } satisfies ThemeTokens);
    this.applyCssTokens();
  }

  init() {
    if (typeof localStorage !== 'undefined') {
      const saved = localStorage.getItem('pawstash_theme_settings');
      if (saved) {
        try {
          Object.assign(this.tokens, JSON.parse(saved));
        } catch (e) {}
      }
    }
    if (typeof window !== 'undefined') {
      this.applyCssTokens();
    }
  }

  setFontScale(fontScale: FontSizeScale) {
    this.tokens.fontScale = fontScale;
    this.applyCssTokens();
  }

  setRadiusScale(radiusScale: RadiusScale) {
    this.tokens.radiusScale = radiusScale;
    this.applyCssTokens();
  }

  setAccent(accent: AccentColor) {
    this.tokens.accent = accent;
    this.applyCssTokens();
  }

  setSurfaceStyle(surfaceStyle: SurfaceStyle) {
    this.tokens.surfaceStyle = surfaceStyle;
    this.applyCssTokens();
  }

  setMotionSpeed(motionSpeed: MotionSpeed) {
    this.tokens.motionSpeed = motionSpeed;
    this.applyCssTokens();
  }

  setBackdropBlur(blurPx: number) {
    this.tokens.backdropBlurPx = blurPx;
    this.applyCssTokens();
  }

  setBorderWidth(widthPx: number) {
    this.tokens.borderWidthPx = widthPx;
    this.applyCssTokens();
  }

  setTitlebarHeight(heightPx: number) {
    this.tokens.titlebarHeightPx = heightPx;
    this.applyCssTokens();
  }

  setSidebarWidth(widthPx: number) {
    this.tokens.sidebarWidthPx = widthPx;
    this.applyCssTokens();
  }

  applyCssTokens() {
    if (typeof document === 'undefined') return;
    this.save();

    const root = document.documentElement;
    const font = FONT_SCALE_MAP[this.tokens.fontScale];
    const radius = RADIUS_SCALE_MAP[this.tokens.radiusScale];
    const accent = ACCENT_COLOR_MAP[this.tokens.accent];
    const motion = MOTION_SPEED_MAP[this.tokens.motionSpeed];

    root.style.setProperty('--titlebar-height', `${this.tokens.titlebarHeightPx}px`);
    root.style.setProperty('--sidebar-width-expanded', `${this.tokens.sidebarWidthPx}px`);

    root.style.setProperty('--text-xs', font.xs);
    root.style.setProperty('--text-sm', font.sm);
    root.style.setProperty('--text-base', font.base);
    root.style.setProperty('--text-lg', font.lg);
    root.style.setProperty('--text-xl', font.xl);

    root.style.setProperty('--radius-sm', radius.sm);
    root.style.setProperty('--radius-md', radius.md);
    root.style.setProperty('--radius-lg', radius.lg);
    root.style.setProperty('--radius-xl', radius.xl);

    let accentPrimary = '';
    let accentHover = '';
    let accentGlow = '';

    if (this.tokens.accent.startsWith('#')) {
      accentPrimary = this.tokens.accent;
      accentHover = darkenColor(this.tokens.accent, 0.12);
      accentGlow = hexToRgba(this.tokens.accent, 0.35);
    } else {
      const accent = ACCENT_COLOR_MAP[this.tokens.accent as any] || ACCENT_COLOR_MAP.rose;
      accentPrimary = accent.primary;
      accentHover = accent.hover;
      accentGlow = accent.glow;
    }

    root.style.setProperty('--accent-primary', accentPrimary);
    root.style.setProperty('--accent-primary-hover', accentHover);
    root.style.setProperty('--accent-glow', accentGlow);
    root.style.setProperty('--text-on-accent', getContrastColor(accentPrimary));

    if (this.tokens.accent === 'rgb') {
      root.classList.add('accent-rgb');
    } else {
      root.classList.remove('accent-rgb');
    }

    root.style.setProperty('--duration-fast', motion.fast);
    root.style.setProperty('--duration-normal', motion.normal);
    root.style.setProperty('--duration-slow', motion.slow);

    root.style.setProperty('--border-width', `${this.tokens.borderWidthPx}px`);
    root.style.setProperty('--backdrop-blur', `${this.tokens.backdropBlurPx}px`);

    if (this.tokens.surfaceStyle === 'oled') {
      root.style.setProperty('--bg-base', '#000000');
      root.style.setProperty('--bg-surface', 'rgba(14, 16, 20, 0.95)');
    } else {
      root.style.setProperty('--bg-base', '#0c0e14');
      root.style.setProperty('--bg-surface', 'rgba(22, 26, 38, 0.7)');
    }
  }

  private async transitionTheme(fn: () => void, event?: MouseEvent) {
    if (typeof document === 'undefined' || !document.startViewTransition) {
      fn();
      return;
    }

    const x = event?.clientX ?? window.innerWidth / 2;
    const y = event?.clientY ?? window.innerHeight / 2;

    const maxRadius = Math.hypot(
      Math.max(x, window.innerWidth - x),
      Math.max(y, window.innerHeight - y)
    );

    const transition = document.startViewTransition(() => {
      fn();
    });

    await transition.ready;

    document.documentElement.animate(
      {
        clipPath: [
          `circle(0px at ${x}px ${y}px)`,
          `circle(${maxRadius}px at ${x}px ${y}px)`
        ]
      },
      {
        duration: 450,
        easing: 'cubic-bezier(0.16, 1, 0.3, 1)',
        fill: 'forwards',
        pseudoElement: '::view-transition-new(root)'
      }
    );
  }

  private save() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('pawstash_theme_settings', JSON.stringify(this.tokens));
    }
  }
}

export const themeState = new ThemeState();

function hexToRgba(hex: string, alpha: number): string {
  const cleanHex = hex.replace('#', '');
  const r = parseInt(cleanHex.substring(0, 2), 16);
  const g = parseInt(cleanHex.substring(2, 4), 16);
  const b = parseInt(cleanHex.substring(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function darkenColor(hex: string, percent: number): string {
  const cleanHex = hex.replace('#', '');
  let r = parseInt(cleanHex.substring(0, 2), 16);
  let g = parseInt(cleanHex.substring(2, 4), 16);
  let b = parseInt(cleanHex.substring(4, 6), 16);

  r = Math.max(0, Math.floor(r * (1 - percent)));
  g = Math.max(0, Math.floor(g * (1 - percent)));
  b = Math.max(0, Math.floor(b * (1 - percent)));

  const rHex = r.toString(16).padStart(2, '0');
  const gHex = g.toString(16).padStart(2, '0');
  const bHex = b.toString(16).padStart(2, '0');
  return `#${rHex}${gHex}${bHex}`;
}

export function getContrastColor(color: string): string {
  let r = 255, g = 255, b = 255;
  
  if (color.startsWith('#')) {
    const cleanHex = color.replace('#', '');
    if (cleanHex.length === 3) {
      r = parseInt(cleanHex[0] + cleanHex[0], 16);
      g = parseInt(cleanHex[1] + cleanHex[1], 16);
      b = parseInt(cleanHex[2] + cleanHex[2], 16);
    } else if (cleanHex.length === 6) {
      r = parseInt(cleanHex.substring(0, 2), 16);
      g = parseInt(cleanHex.substring(2, 4), 16);
      b = parseInt(cleanHex.substring(4, 6), 16);
    }
  } else if (color.startsWith('rgb')) {
    const matches = color.match(/\d+/g);
    if (matches && matches.length >= 3) {
      r = parseInt(matches[0], 10);
      g = parseInt(matches[1], 10);
      b = parseInt(matches[2], 10);
    }
  }
  
  const yiq = (r * 299 + g * 587 + b * 114) / 1000;
  return yiq >= 128 ? '#111111' : '#ffffff';
}
