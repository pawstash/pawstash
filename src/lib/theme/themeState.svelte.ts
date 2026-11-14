import type { ThemeTokens, SurfaceStyle, AccentColor } from './tokens';
import { RADIUS_SCALE_MAP, MOTION_SPEED_MAP, textSizes } from './tokens';
import {
  DEFAULT_COLOR_MODE,
  DEFAULT_CONTRAST_LEVEL,
  DEFAULT_SCHEME_VARIANT,
  FALLBACK_LIGHT_PALETTE,
  FALLBACK_PALETTE,
  type AccentPalette,
  type ColorMode,
  type ContrastLevel,
  type SchemeVariant
} from './palette';
import { apiGetSystemAccentColor, apiSetWindowBackgroundColor } from '$lib/utils/ipc';
import { logger } from '$lib/utils/logger';

const LEGACY_FONT_SCALES: Record<string, number> = { compact: 90, large: 110 };
const PALETTE_CACHE_KEY = 'pawstash_theme_palette';
const SYSTEM_FONT_STACK =
  "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, 'Noto Sans', 'Hiragino Sans', 'Hiragino Kaku Gothic ProN', 'Yu Gothic UI', 'Meiryo UI', 'PingFang SC', 'PingFang TC', 'Microsoft YaHei UI', 'Microsoft JhengHei UI', 'Apple SD Gothic Neo', 'Malgun Gothic', sans-serif";

type PaletteEngine = typeof import('./paletteEngine');

let enginePromise: Promise<PaletteEngine> | null = null;

function loadPaletteEngine(): Promise<PaletteEngine> {
  return (enginePromise ??= import('./paletteEngine'));
}

function readPaletteCache(): AccentPalette | null {
  if (typeof localStorage === 'undefined') return null;
  try {
    const parsed = JSON.parse(localStorage.getItem(PALETTE_CACHE_KEY) || 'null');
    if (
      parsed &&
      typeof parsed.primary === 'string' &&
      typeof parsed.surface === 'string' &&
      Array.isArray(parsed.quadrants) &&
      parsed.quadrants.length === 4
    ) {
      return parsed as AccentPalette;
    }
  } catch {}
  return null;
}

function measureTextZoom(): number {
  const sample = 'MMMMMMMMMM';
  const context = document.createElement('canvas').getContext('2d');
  if (!context) return 1;
  context.font = '100px monospace';
  const expected = context.measureText(sample).width;
  const probe = document.createElement('span');
  probe.style.cssText = 'position:absolute;visibility:hidden;white-space:nowrap;font:100px monospace';
  probe.textContent = sample;
  document.body.appendChild(probe);
  const rendered = probe.getBoundingClientRect().width;
  probe.remove();
  const zoom = rendered / expected;
  return Number.isFinite(zoom) && zoom > 0.5 && zoom < 4 && Math.abs(zoom - 1) > 0.02 ? zoom : 1;
}

export interface SystemMonetPalette {
  primary: string;
  quadrants: [string, string, string, string];
}

export class ThemeState {
  tokens = $state<ThemeTokens>({
    fontFamily: '',
    radiusScale: 'smooth',
    surfaceStyle: 'flat',
    accent: '#D69085',
    colorMode: DEFAULT_COLOR_MODE,
    schemeVariant: DEFAULT_SCHEME_VARIANT,
    contrastLevel: DEFAULT_CONTRAST_LEVEL,
    motionSpeed: 'smooth',
    backdropBlurPx: 24,
    borderWidthPx: 1
  });

  systemPalette = $state<SystemMonetPalette | null>(null);
  overrideAccent = $state<string | null>(null);
  systemPrefersDark = $state(true);
  private textZoom = 1;
  private savedAccent: AccentColor | undefined;
  private legacyUiScale: number | null = null;
  private lastWindowBackground: string | null = null;
  private engine = $state.raw<PaletteEngine | null>(null);
  private cachedPalette: AccentPalette | null = null;
  private paletteMemo: { key: string; palette: AccentPalette } | null = null;

  get isDark(): boolean {
    if (this.tokens.colorMode === 'system') return this.systemPrefersDark;
    return this.tokens.colorMode === 'dark';
  }

  get palette(): AccentPalette {
    if (!this.engine) {
      return this.cachedPalette ?? (this.isDark ? FALLBACK_PALETTE : FALLBACK_LIGHT_PALETTE);
    }

    const { schemeVariant, contrastLevel } = this.tokens;
    const source =
      this.overrideAccent ??
      (this.tokens.accent === 'system' && this.systemPalette
        ? this.systemPalette.primary
        : this.tokens.accent);
    const quadrants =
      !this.overrideAccent && this.tokens.accent === 'system'
        ? this.systemPalette?.quadrants
        : undefined;
    const key = `${source}|${quadrants?.join(',') ?? ''}|${schemeVariant}|${contrastLevel}|${this.isDark}`;
    if (this.paletteMemo?.key === key) return this.paletteMemo.palette;

    const palette = this.engine.generateAccentPalette(
      source,
      quadrants,
      schemeVariant,
      contrastLevel,
      this.isDark
    );
    this.paletteMemo = { key, palette };
    return palette;
  }

  get schemeIsForcedNeutral(): boolean {
    const source = this.overrideAccent ?? this.tokens.accent;
    if (source === 'system' || !this.engine) return false;
    return this.tokens.schemeVariant !== 'monochrome' && this.engine.isAchromatic(source);
  }

  private async ensureEngine(): Promise<PaletteEngine> {
    this.engine ??= await loadPaletteEngine();
    return this.engine;
  }

  reset() {
    Object.assign(this.tokens, {
      fontFamily: '',
      radiusScale: 'smooth',
      surfaceStyle: 'flat',
      accent: this.systemPalette ? 'system' : '#D69085',
      colorMode: DEFAULT_COLOR_MODE,
      schemeVariant: DEFAULT_SCHEME_VARIANT,
      contrastLevel: DEFAULT_CONTRAST_LEVEL,
      motionSpeed: 'smooth',
      backdropBlurPx: 24,
      borderWidthPx: 1
    } satisfies ThemeTokens);
    this.overrideAccent = null;
    this.applyCssTokens();
  }

  takeLegacyUiScale(): number | null {
    const legacy = this.legacyUiScale;
    this.legacyUiScale = null;
    return legacy;
  }

  async fetchSystemPalette() {
    try {
      const engine = await this.ensureEngine();
      const res = await apiGetSystemAccentColor();
      if (res) {
        if (res.startsWith('{')) {
          const parsed = JSON.parse(res);
          const primary = parsed.primary || parsed.c1;
          const quadrants: [string, string, string, string] = [
            parsed.c1 || primary,
            parsed.c2 || primary,
            parsed.c3 || primary,
            parsed.c4 || primary
          ];
          this.systemPalette = { primary, quadrants };
        } else if (res.startsWith('#')) {
          const pal = engine.generateAccentPalette(res);
          this.systemPalette = { primary: res, quadrants: pal.quadrants };
        }
      }
    } catch (e) {
      logger.debug('System accent colour unavailable', e);
    }
  }

  init() {
    if (typeof localStorage !== 'undefined') {
      const saved = localStorage.getItem('pawstash_theme_settings');
      if (saved) {
        try {
          const parsed = JSON.parse(saved);
          this.savedAccent = parsed.accent;
          if ('sidebarWidthPx' in parsed) {
            delete parsed.sidebarWidthPx;
            localStorage.setItem('pawstash_theme_settings', JSON.stringify(parsed));
          }
          delete (this.tokens as any).sidebarWidthPx;
          if ('fontScale' in parsed) {
            const legacy = parsed.fontScale;
            this.legacyUiScale = typeof legacy === 'number' ? legacy : (LEGACY_FONT_SCALES[legacy] ?? null);
            delete parsed.fontScale;
            localStorage.setItem('pawstash_theme_settings', JSON.stringify(parsed));
          }
          Object.assign(this.tokens, parsed);
          if (this.tokens.surfaceStyle !== 'oled') this.tokens.surfaceStyle = 'flat';
        } catch (e) {
          logger.warn('[ThemeState] Failed to parse saved theme settings:', e);
        }
      }
    }

    this.cachedPalette = readPaletteCache();
    this.watchSystemColorScheme();
    if (typeof document !== 'undefined') this.textZoom = measureTextZoom();

    if (typeof window !== 'undefined') this.applyCssTokens();
  }

  async upgradePalette() {
    try {
      await this.ensureEngine();
      await this.fetchSystemPalette();
      if (!this.savedAccent && this.systemPalette) this.tokens.accent = 'system';
      if (typeof window !== 'undefined') this.applyCssTokens();
    } catch (e) {
      logger.warn('Could not load the colour engine, keeping the default palette', e);
    }
  }

  setOverrideAccent(color: string) {
    if (!color || this.overrideAccent === color) return;
    this.overrideAccent = color;
    this.applyCssTokens();
    this.refreshWindowEffect();
  }

  clearOverrideAccent() {
    if (this.overrideAccent !== null) {
      this.overrideAccent = null;
      this.applyCssTokens();
      this.refreshWindowEffect();
    }
  }

  setFontFamily(fontFamily: string) {
    this.tokens.fontFamily = fontFamily;
    this.applyFontTokens();
    this.save();
  }

  private applyFontTokens() {
    const root = document.documentElement;
    const userFont = this.tokens.fontFamily?.trim();
    if (!userFont) {
      root.style.removeProperty('--font-sans');
      root.style.removeProperty('--font-display');
      return;
    }
    const fullStack = `${userFont}, ${SYSTEM_FONT_STACK}`;
    root.style.setProperty('--font-sans', fullStack);
    root.style.setProperty('--font-display', fullStack);
  }

  setAccent(accent: AccentColor) {
    this.tokens.accent = accent;
    this.applyCssTokens();
    this.refreshWindowEffect();
  }

  setColorMode(colorMode: ColorMode) {
    this.tokens.colorMode = colorMode;
    this.applyCssTokens();
    this.refreshWindowEffect();
  }

  private refreshWindowEffect() {
    void import('./backgroundState.svelte').then((m) => m.backgroundState.refreshWindowEffect());
  }

  watchSystemColorScheme() {
    if (typeof window === 'undefined' || !window.matchMedia) return;
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    this.systemPrefersDark = query.matches;
    query.addEventListener('change', (event) => {
      this.systemPrefersDark = event.matches;
      if (this.tokens.colorMode !== 'system') return;
      this.applyCssTokens();
      this.refreshWindowEffect();
    });
  }

  setSchemeVariant(schemeVariant: SchemeVariant) {
    this.tokens.schemeVariant = schemeVariant;
    this.applyCssTokens();
    this.refreshWindowEffect();
  }

  setContrastLevel(contrastLevel: ContrastLevel) {
    this.tokens.contrastLevel = contrastLevel;
    this.applyCssTokens();
    this.refreshWindowEffect();
  }

  setSurfaceStyle(surfaceStyle: SurfaceStyle) {
    if (this.tokens.surfaceStyle === surfaceStyle) return;
    this.tokens.surfaceStyle = surfaceStyle;
    this.applyCssTokens();
  }

  applyCssTokens() {
    if (typeof document === 'undefined') return;
    this.save();

    const root = document.documentElement;
    const textFactor = 1 / this.textZoom;
    const font = textSizes(textFactor);
    const radius = RADIUS_SCALE_MAP[this.tokens.radiusScale];
    const motion = MOTION_SPEED_MAP[this.tokens.motionSpeed];

    root.style.setProperty('--text-xs', font.xs);
    root.style.setProperty('--text-sm', font.sm);
    root.style.setProperty('--text-base', font.base);
    root.style.setProperty('--text-lg', font.lg);
    root.style.setProperty('--text-xl', font.xl);
    root.style.setProperty('--font-scale', String(textFactor));

    root.style.setProperty('--radius-sm', radius.sm);
    root.style.setProperty('--radius-md', radius.md);
    root.style.setProperty('--radius-lg', radius.lg);
    root.style.setProperty('--radius-xl', radius.xl);

    const pal = this.palette;
    if (this.engine) this.cachedPalette = pal;
    this.savePalette(pal);
    root.style.setProperty('--accent-primary', pal.primary);
    root.style.setProperty('--accent-primary-hover', pal.primaryHover);
    root.style.setProperty('--accent-on-primary', pal.onPrimary);
    root.style.setProperty('--accent-container', pal.container);
    root.style.setProperty('--accent-on-container', pal.onContainer);
    root.style.setProperty('--accent-on-surface', pal.onSurface);
    root.style.setProperty('--accent-subtle', pal.subtle);
    root.style.setProperty('--accent-glow', pal.glow);
    root.style.setProperty('--accent-outline', pal.outline);
    root.style.setProperty('--text-on-accent', pal.onPrimary);

    root.style.setProperty('--choice-active-bg', pal.choiceActiveBg);
    root.style.setProperty('--choice-active-text', pal.choiceActiveText);

    root.style.setProperty('--text-primary', pal.textPrimary);
    root.style.setProperty('--text-secondary', pal.textSecondary);
    root.style.setProperty('--text-muted', pal.textMuted);

    root.style.setProperty('--shadow-alpha', this.isDark ? '0.6' : '0.16');
    root.style.setProperty('--shadow-alpha-strong', this.isDark ? '0.78' : '0.22');

    root.classList.toggle('dark', this.isDark);
    root.classList.toggle('light', !this.isDark);
    root.style.setProperty('color-scheme', this.isDark ? 'dark' : 'light');

    const oled = this.tokens.surfaceStyle === 'oled' && this.isDark;
    const surface = oled ? '#000000' : pal.surface;
    const [containerLow, container, containerHigh, containerHighest] = oled
      ? [pal.surfaceContainerLowest, pal.surfaceContainerLow, pal.surfaceContainer, pal.surfaceContainerHigh]
      : [pal.surfaceContainerLow, pal.surfaceContainer, pal.surfaceContainerHigh, pal.surfaceContainerHighest];
    root.style.setProperty('--surface-tint-rgb', pal.surfaceTintRgb);
    root.style.setProperty('--surface', surface);
    root.style.setProperty('--surface-container-low', containerLow);
    root.style.setProperty('--surface-container', container);
    root.style.setProperty('--surface-container-high', containerHigh);
    root.style.setProperty('--surface-container-highest', containerHighest);
    root.style.setProperty('--outline-variant', pal.outlineVariant);
    root.style.setProperty('--bg-dropdown', containerHigh);

    const acrylicRgb = this.engine?.computeAcrylicTintRgb(pal.primary, this.isDark);
    if (acrylicRgb) {
      root.style.setProperty('--acrylic-tint-rgb', `${acrylicRgb.r}, ${acrylicRgb.g}, ${acrylicRgb.b}`);
    }

    const hairline = { '-1': [0.06, 0.14], '0': [0.08, 0.18], '0.5': [0.16, 0.3], '1': [0.28, 0.45] }[
      String(this.tokens.contrastLevel)
    ] ?? [0.08, 0.18];
    root.style.setProperty('--border-alpha', String(hairline[0]));
    root.style.setProperty('--border-alpha-hover', String(hairline[1]));

    root.style.setProperty('--slider-active', pal.primary);
    root.style.setProperty('--slider-handle', pal.primary);
    root.style.setProperty('--slider-inactive', pal.container);
    root.style.setProperty('--slider-stop-on-active', pal.onPrimary);
    root.style.setProperty('--slider-stop-on-inactive', pal.onContainer);

    if (this.tokens.accent === 'rgb' && !this.overrideAccent) {
      root.classList.add('accent-rgb');
    } else {
      root.classList.remove('accent-rgb');
    }

    root.style.setProperty('--duration-fast', motion.fast);
    root.style.setProperty('--duration-normal', motion.normal);
    root.style.setProperty('--duration-slow', motion.slow);

    root.style.setProperty('--border-width', `${this.tokens.borderWidthPx}px`);
    root.style.setProperty('--backdrop-blur', `${this.tokens.backdropBlurPx}px`);

    root.style.setProperty('--bg-surface-hover', containerHigh);
    root.style.setProperty('--bg-surface', containerLow);
    root.style.setProperty('--bg-base', surface);
    this.syncWindowBackground(surface);

    this.applyFontTokens();
  }

  private syncWindowBackground(color: string) {
    if (!/^#[0-9a-f]{3,8}$/i.test(color) || color === this.lastWindowBackground) return;
    this.lastWindowBackground = color;
    void apiSetWindowBackgroundColor(color).catch((error) =>
      logger.warn('Failed to sync window background colour', error)
    );
  }

  private save() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('pawstash_theme_settings', JSON.stringify(this.tokens));
    }
  }

  private savePalette(palette: AccentPalette) {
    if (typeof localStorage === 'undefined') return;
    try {
      localStorage.setItem(PALETTE_CACHE_KEY, JSON.stringify(palette));
    } catch {}
  }
}

export const themeState = new ThemeState();

