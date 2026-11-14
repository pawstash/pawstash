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
import {
  DEFAULT_CONTRAST_LEVEL,
  DEFAULT_SCHEME_VARIANT,
  PRESET_QUADRANTS,
  parseColorToRgb,
  type AccentPalette,
  type ContrastLevel,
  type RgbColor,
  type SchemeVariant
} from './palette';

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

function achromatic(hex: string): string {
  return hexFromArgb(Hct.from(0, 0, Hct.fromInt(argbFromHex(hex)).tone).toInt());
}

function rgbToHex(r: number, g: number, b: number): string {
  const toHex = (n: number) =>
    Math.min(255, Math.max(0, Math.round(n))).toString(16).padStart(2, '0');
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

function getRelativeLuminance(color: RgbColor): number {
  const channel = (value: number) => {
    const v = value / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b);
}

function mixRgb(color1: RgbColor, color2: RgbColor, weight: number): RgbColor {
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

function sourceHct(input: string): Hct {
  const { r, g, b } = parseColorToRgb(input);
  return Hct.fromInt(argbFromRgb(r, g, b));
}

export function isAchromatic(input: string): boolean {
  return sourceHct(input).chroma < ACHROMATIC_CHROMA;
}

function buildScheme(
  input: string,
  variant: SchemeVariant = DEFAULT_SCHEME_VARIANT,
  contrastLevel: ContrastLevel = DEFAULT_CONTRAST_LEVEL,
  isDark = true
): DynamicScheme {
  const hct = sourceHct(input);
  const effective = hct.chroma < ACHROMATIC_CHROMA ? 'monochrome' : variant;
  const Scheme = SCHEME_CONSTRUCTORS[effective] ?? SchemeTonalSpot;
  return new Scheme(hct, isDark, contrastLevel, '2021', 'phone');
}

function role(scheme: DynamicScheme, name: string): string {
  const color = (MaterialDynamicColors as any)[name];
  return color?.getArgb ? hexFromArgb(color.getArgb(scheme)) : '#ffffff';
}

function accentOnSurface(scheme: DynamicScheme, primary: string, isDark: boolean): string {
  const surface = parseColorToRgb(achromatic(role(scheme, 'surfaceContainerHigh')));
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
  const surfaceContainerLow = achromatic(role(scheme, 'surfaceContainerLow'));
  const onSurface = achromatic(role(scheme, 'onSurface'));

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
    outline: achromatic(role(scheme, 'outline')),
    outlineVariant: achromatic(role(scheme, 'outlineVariant')),
    textPrimary: onSurface,
    textSecondary: achromatic(role(scheme, 'onSurfaceVariant')),
    textMuted: achromatic(role(scheme, 'outline')),
    surfaceTintRgb: rgbTriple(onSurface),
    surface: achromatic(role(scheme, 'surface')),
    surfaceDim: achromatic(role(scheme, 'surfaceDim')),
    surfaceBright: achromatic(role(scheme, 'surfaceBright')),
    surfaceContainerLowest: achromatic(role(scheme, 'surfaceContainerLowest')),
    surfaceContainerLow,
    surfaceContainer: achromatic(role(scheme, 'surfaceContainer')),
    surfaceContainerHigh: achromatic(role(scheme, 'surfaceContainerHigh')),
    surfaceContainerHighest: achromatic(role(scheme, 'surfaceContainerHighest'))
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
