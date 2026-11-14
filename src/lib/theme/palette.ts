export interface AccentPalette {
  primary: string;
  primaryHover: string;
  onPrimary: string;
  container: string;
  onContainer: string;
  onSurface: string;
  subtle: string;
  glow: string;
  quadrants: [string, string, string, string];
  choiceActiveBg: string;
  choiceActiveText: string;
  outline: string;
  outlineVariant: string;
  surfaceTintRgb: string;
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
  surface: string;
  surfaceDim: string;
  surfaceBright: string;
  surfaceContainerLowest: string;
  surfaceContainerLow: string;
  surfaceContainer: string;
  surfaceContainerHigh: string;
  surfaceContainerHighest: string;
}

export interface RgbColor {
  r: number;
  g: number;
  b: number;
}

export type SchemeVariant =
  | 'tonal-spot'
  | 'vibrant'
  | 'expressive'
  | 'fidelity'
  | 'content'
  | 'neutral'
  | 'monochrome'
  | 'rainbow'
  | 'fruit-salad';

export type ContrastLevel = -1 | 0 | 0.5 | 1;

export type ColorMode = 'dark' | 'light' | 'system';

export const COLOR_MODES: ColorMode[] = ['system', 'light', 'dark'];
export const DEFAULT_COLOR_MODE: ColorMode = 'dark';

export const SCHEME_VARIANTS: SchemeVariant[] = [
  'tonal-spot',
  'vibrant',
  'expressive',
  'fidelity',
  'content',
  'neutral',
  'monochrome',
  'rainbow',
  'fruit-salad'
];

export const DEFAULT_SCHEME_VARIANT: SchemeVariant = 'tonal-spot';
export const DEFAULT_CONTRAST_LEVEL: ContrastLevel = 0;

export const PRESET_QUADRANTS: Record<string, [string, string, string, string]> = {
  '#d69085': ['#d69085', '#e8c2bc', '#60413c', '#a16c64'],
  '#D69085': ['#d69085', '#e8c2bc', '#60413c', '#a16c64'],
  rose: ['#f43f5e', '#fda4af', '#9f1239', '#e11d48'],
  violet: ['#8b5cf6', '#c4b5fd', '#5b21b6', '#7c3aed'],
  cyan: ['#06b6d4', '#67e8f9', '#0e7490', '#0891b2'],
  emerald: ['#10b981', '#6ee7b7', '#047857', '#059669'],
  amber: ['#f59e0b', '#fcd34d', '#b45309', '#d97706'],
  indigo: ['#6366f1', '#a5b4fc', '#3730a3', '#4f46e5']
};

const PRESET_PRIMARY_MAP: Record<string, string> = {
  rose: '#f43f5e',
  violet: '#8b5cf6',
  cyan: '#06b6d4',
  emerald: '#10b981',
  amber: '#f59e0b',
  indigo: '#6366f1'
};

const DEFAULT_ACCENT = '#d69085';

export const ACCENT_PRESETS: { id: string; hex: string }[] = [
  { id: 'terracotta', hex: DEFAULT_ACCENT },
  { id: 'rose', hex: PRESET_PRIMARY_MAP.rose },
  { id: 'violet', hex: PRESET_PRIMARY_MAP.violet },
  { id: 'indigo', hex: PRESET_PRIMARY_MAP.indigo },
  { id: 'cyan', hex: PRESET_PRIMARY_MAP.cyan },
  { id: 'emerald', hex: PRESET_PRIMARY_MAP.emerald },
  { id: 'amber', hex: PRESET_PRIMARY_MAP.amber }
];

export function parseColorToRgb(input: string): RgbColor {
  if (!input) return { r: 214, g: 144, b: 133 };

  const trimmed = input.trim().toLowerCase();

  if (PRESET_PRIMARY_MAP[trimmed] && PRESET_PRIMARY_MAP[trimmed] !== trimmed) {
    return parseColorToRgb(PRESET_PRIMARY_MAP[trimmed]);
  }

  if (trimmed.startsWith('#')) {
    const hex = trimmed.replace('#', '');
    if (hex.length === 3 || hex.length === 4) {
      return {
        r: parseInt(hex[0] + hex[0], 16) || 0,
        g: parseInt(hex[1] + hex[1], 16) || 0,
        b: parseInt(hex[2] + hex[2], 16) || 0
      };
    }
    if (hex.length >= 6) {
      return {
        r: parseInt(hex.substring(0, 2), 16) || 0,
        g: parseInt(hex.substring(2, 4), 16) || 0,
        b: parseInt(hex.substring(4, 6), 16) || 0
      };
    }
  }

  const rgbMatch = trimmed.match(/\d+/g);
  if (rgbMatch && rgbMatch.length >= 3) {
    return {
      r: Math.min(255, Math.max(0, parseInt(rgbMatch[0], 10))),
      g: Math.min(255, Math.max(0, parseInt(rgbMatch[1], 10))),
      b: Math.min(255, Math.max(0, parseInt(rgbMatch[2], 10)))
    };
  }

  return { r: 214, g: 144, b: 133 };
}

export function getPerceivedLuminance(r: number, g: number, b: number): number {
  return (r * 299 + g * 587 + b * 114) / 1000;
}

function getRelativeLuminance(color: RgbColor): number {
  const channel = (value: number) => {
    const v = value / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b);
}

export function getContrastInk(background: string): 'light' | 'dark' {
  const luminance = getRelativeLuminance(parseColorToRgb(background));
  const againstWhite = 1.05 / (luminance + 0.05);
  const againstBlack = (luminance + 0.05) / 0.05;
  return againstWhite >= againstBlack ? 'light' : 'dark';
}

export const FALLBACK_PALETTE: AccentPalette = {
  primary: '#ffb4a8',
  primaryHover: '#f0a99e',
  onPrimary: '#561e17',
  container: '#5d3f3b',
  onContainer: '#ffdad4',
  onSurface: '#ffb4a8',
  subtle: 'rgba(255, 180, 168, 0.22)',
  glow: 'rgba(255, 180, 168, 0.35)',
  quadrants: ['#d69085', '#e8c2bc', '#60413c', '#a16c64'],
  choiceActiveBg: '#5d3f3b',
  choiceActiveText: '#ffdad4',
  outline: '#909090',
  outlineVariant: '#474747',
  surfaceTintRgb: '227, 227, 227',
  textPrimary: '#e3e3e3',
  textSecondary: '#c7c7c7',
  textMuted: '#909090',
  surface: '#131313',
  surfaceDim: '#131313',
  surfaceBright: '#373737',
  surfaceContainerLowest: '#0e0e0e',
  surfaceContainerLow: '#1b1b1b',
  surfaceContainer: '#1f1f1f',
  surfaceContainerHigh: '#2a2a2a',
  surfaceContainerHighest: '#343434'
};

export const FALLBACK_LIGHT_PALETTE: AccentPalette = {
  primary: '#904b40',
  primaryHover: '#7d4137',
  onPrimary: '#ffffff',
  container: '#ffdad4',
  onContainer: '#5d3f3b',
  onSurface: '#904b40',
  subtle: 'rgba(144, 75, 64, 0.22)',
  glow: 'rgba(144, 75, 64, 0.35)',
  quadrants: ['#d69085', '#e8c2bc', '#60413c', '#a16c64'],
  choiceActiveBg: '#ffdad4',
  choiceActiveText: '#5d3f3b',
  outline: '#777777',
  outlineVariant: '#c7c7c7',
  surfaceTintRgb: '27, 27, 27',
  textPrimary: '#1b1b1b',
  textSecondary: '#474747',
  textMuted: '#777777',
  surface: '#f9f9f9',
  surfaceDim: '#d9d9d9',
  surfaceBright: '#f9f9f9',
  surfaceContainerLowest: '#ffffff',
  surfaceContainerLow: '#f3f3f3',
  surfaceContainer: '#eeeeee',
  surfaceContainerHigh: '#e8e8e8',
  surfaceContainerHighest: '#e3e3e3'
};
