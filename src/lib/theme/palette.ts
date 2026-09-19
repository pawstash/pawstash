import {
  Hct,
  MaterialDynamicColors,
  SchemeContent,
  SchemeExpressive,
  SchemeFidelity,
  SchemeFruitSalad,
  SchemeMonochrome,
  SchemeNeutral,
  SchemeRainbow,
  SchemeTonalSpot,
  SchemeVibrant,
  argbFromHex,
  argbFromRgb,
  hexFromArgb,
  type DynamicScheme
} from '@material/material-color-utilities';

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
  surfaceLowRgb: string;
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
  surface: string;
  surfaceDim: string;
  surfaceBright: string;
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

export const CONTRAST_LEVELS: ContrastLevel[] = [-1, 0, 0.5, 1];

export const DEFAULT_SCHEME_VARIANT: SchemeVariant = 'tonal-spot';
export const DEFAULT_CONTRAST_LEVEL: ContrastLevel = 0;

const SCHEME_CONSTRUCTORS: Record<SchemeVariant, any> = {
  'tonal-spot': SchemeTonalSpot,
  vibrant: SchemeVibrant,
  expressive: SchemeExpressive,
  fidelity: SchemeFidelity,
  content: SchemeContent,
  neutral: SchemeNeutral,
  monochrome: SchemeMonochrome,
  rainbow: SchemeRainbow,
  'fruit-salad': SchemeFruitSalad
};

const ACHROMATIC_CHROMA = 10;
const NEUTRAL_CHROMA_TEXT = 2;
const NEUTRAL_CHROMA_SURFACE = 4;

function neutralize(hex: string, maxChroma: number, tone?: number): string {
  const hct = Hct.fromInt(argbFromHex(hex));
  return hexFromArgb(
    Hct.from(hct.hue, Math.min(hct.chroma, maxChroma), tone ?? hct.tone).toInt()
  );
}

const ACCENT_SURFACE_DARK: RgbColor = { r: 24, g: 24, b: 27 };

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

export const PRESET_PRIMARY_MAP: Record<string, string> = {
  rose: '#f43f5e',
  violet: '#8b5cf6',
  cyan: '#06b6d4',
  emerald: '#10b981',
  amber: '#f59e0b',
  indigo: '#6366f1'
};

export const DEFAULT_ACCENT = '#d69085';

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

export function rgbToHex(r: number, g: number, b: number): string {
  const toHex = (n: number) =>
    Math.min(255, Math.max(0, Math.round(n))).toString(16).padStart(2, '0');
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

export function getPerceivedLuminance(r: number, g: number, b: number): number {
  return (r * 299 + g * 587 + b * 114) / 1000;
}

export function getRelativeLuminance(color: RgbColor): number {
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

export function mixRgb(color1: RgbColor, color2: RgbColor, weight: number): RgbColor {
  const w = Math.min(1, Math.max(0, weight));
  return {
    r: Math.round(color1.r * (1 - w) + color2.r * w),
    g: Math.round(color1.g * (1 - w) + color2.g * w),
    b: Math.round(color1.b * (1 - w) + color2.b * w)
  };
}

function contrastRatio(a: RgbColor, b: RgbColor): number {
  const la = getRelativeLuminance(a);
  const lb = getRelativeLuminance(b);
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

function rgba(hex: string, alpha: number): string {
  const { r, g, b } = parseColorToRgb(hex);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function rgbTriple(hex: string): string {
  const { r, g, b } = parseColorToRgb(hex);
  return `${r}, ${g}, ${b}`;
}

function stateLayer(base: string, over: string, opacity: number): string {
  const mixed = mixRgb(parseColorToRgb(base), parseColorToRgb(over), opacity);
  return rgbToHex(mixed.r, mixed.g, mixed.b);
}

export function sourceHct(input: string): Hct {
  const { r, g, b } = parseColorToRgb(input);
  return Hct.fromInt(argbFromRgb(r, g, b));
}

export function isAchromatic(input: string): boolean {
  return sourceHct(input).chroma < ACHROMATIC_CHROMA;
}

export function buildScheme(
  input: string,
  variant: SchemeVariant = DEFAULT_SCHEME_VARIANT,
  contrastLevel: ContrastLevel = DEFAULT_CONTRAST_LEVEL,
  isDark = true
): DynamicScheme {
  const hct = sourceHct(input);
  const effective = hct.chroma < ACHROMATIC_CHROMA ? 'monochrome' : variant;
  const Scheme = SCHEME_CONSTRUCTORS[effective] ?? SchemeTonalSpot;
  return new Scheme(hct, isDark, contrastLevel, '2025', 'phone');
}

function role(scheme: DynamicScheme, name: string): string {
  const color = (MaterialDynamicColors as any)[name];
  return color?.getArgb ? hexFromArgb(color.getArgb(scheme)) : '#ffffff';
}

function accentOnSurface(scheme: DynamicScheme, primary: string, isDark: boolean): string {
  const surface = isDark
    ? ACCENT_SURFACE_DARK
    : parseColorToRgb(role(scheme, 'surfaceContainer'));
  if (contrastRatio(parseColorToRgb(primary), surface) >= 4.5) return primary;

  const tones = isDark ? [60, 65, 70, 75, 80, 85, 90, 95, 100] : [50, 45, 40, 35, 30, 25, 20, 10, 0];
  for (const tone of tones) {
    const candidate = hexFromArgb(scheme.primaryPalette.tone(tone));
    if (contrastRatio(parseColorToRgb(candidate), surface) >= 4.5) return candidate;
  }
  return isDark ? '#ffffff' : '#000000';
}

export function generateAccentPalette(
  input: string,
  customQuadrants?: [string, string, string, string],
  variant: SchemeVariant = DEFAULT_SCHEME_VARIANT,
  contrastLevel: ContrastLevel = DEFAULT_CONTRAST_LEVEL,
  isDark = true
): AccentPalette {
  const scheme = buildScheme(input, variant, contrastLevel, isDark);

  const primary = role(scheme, 'primary');
  const onPrimary = role(scheme, 'onPrimary');
  const secondaryContainer = role(scheme, 'secondaryContainer');
  const onSecondaryContainer = role(scheme, 'onSecondaryContainer');
  const surfaceContainerLow = role(scheme, 'surfaceContainerLow');

  const quadrants: [string, string, string, string] =
    customQuadrants ??
    PRESET_QUADRANTS[input.trim().toLowerCase()] ??
    [
      primary,
      hexFromArgb(scheme.primaryPalette.tone(90)),
      hexFromArgb(scheme.primaryPalette.tone(30)),
      hexFromArgb(scheme.secondaryPalette.tone(60))
    ];

  return {
    primary,
    primaryHover: stateLayer(primary, onPrimary, 0.08),
    onPrimary,
    container: secondaryContainer,
    onContainer: onSecondaryContainer,
    onSurface: accentOnSurface(scheme, primary, isDark),
    subtle: rgba(primary, 0.22),
    glow: rgba(primary, 0.35),
    quadrants,
    choiceActiveBg: secondaryContainer,
    choiceActiveText: onSecondaryContainer,
    outline: role(scheme, 'outline'),
    outlineVariant: role(scheme, 'outlineVariant'),
    textPrimary: neutralize(role(scheme, 'onSurface'), NEUTRAL_CHROMA_TEXT, isDark ? 96 : 8),
    textSecondary: neutralize(role(scheme, 'onSurfaceVariant'), NEUTRAL_CHROMA_TEXT),
    textMuted: neutralize(role(scheme, 'outline'), NEUTRAL_CHROMA_TEXT),
    surfaceTintRgb: rgbTriple(
      neutralize(
        hexFromArgb(scheme.neutralPalette.tone(isDark ? 95 : 10)),
        NEUTRAL_CHROMA_TEXT,
        isDark ? 97 : 8
      )
    ),
    surfaceLowRgb: rgbTriple(neutralize(surfaceContainerLow, NEUTRAL_CHROMA_SURFACE)),
    surface: neutralize(role(scheme, 'surface'), NEUTRAL_CHROMA_SURFACE),
    surfaceDim: neutralize(role(scheme, 'surfaceDim'), NEUTRAL_CHROMA_SURFACE),
    surfaceBright: neutralize(role(scheme, 'surfaceBright'), NEUTRAL_CHROMA_SURFACE),
    surfaceContainerLow: neutralize(surfaceContainerLow, NEUTRAL_CHROMA_SURFACE),
    surfaceContainer: neutralize(role(scheme, 'surfaceContainer'), NEUTRAL_CHROMA_SURFACE),
    surfaceContainerHigh: neutralize(role(scheme, 'surfaceContainerHigh'), NEUTRAL_CHROMA_SURFACE),
    surfaceContainerHighest: neutralize(
      role(scheme, 'surfaceContainerHighest'),
      NEUTRAL_CHROMA_SURFACE
    )
  };
}

export function computeAcrylicTintRgb(accent: string, isDark: boolean): RgbColor {
  const accentRgb = parseColorToRgb(accent);
  if (isAchromatic(accent)) {
    return isDark ? { r: 18, g: 19, b: 24 } : { r: 248, g: 248, b: 250 };
  }
  const base = isDark ? { r: 14, g: 16, b: 22 } : { r: 252, g: 252, b: 254 };
  return mixRgb(base, accentRgb, isDark ? 0.38 : 0.26);
}
