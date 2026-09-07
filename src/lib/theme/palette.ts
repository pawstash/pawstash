export interface AccentPalette {
  primary: string;
  primaryHover: string;
  onPrimary: string;
  container: string;
  onContainer: string;
  subtle: string;
  glow: string;
  quadrants: [string, string, string, string];
  choiceActiveBg: string;
  choiceActiveText: string;
  choiceInactiveBg: string;
  choiceInactiveText: string;
}

export interface RgbColor {
  r: number;
  g: number;
  b: number;
}

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
  const toHex = (n: number) => Math.min(255, Math.max(0, Math.round(n))).toString(16).padStart(2, '0');
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

export function getPerceivedLuminance(r: number, g: number, b: number): number {
  return (r * 299 + g * 587 + b * 114) / 1000;
}

export function mixRgb(color1: RgbColor, color2: RgbColor, weight: number): RgbColor {
  const w = Math.min(1, Math.max(0, weight));
  return {
    r: Math.round(color1.r * (1 - w) + color2.r * w),
    g: Math.round(color1.g * (1 - w) + color2.g * w),
    b: Math.round(color1.b * (1 - w) + color2.b * w)
  };
}

export function generateAccentPalette(
  input: string,
  customQuadrants?: [string, string, string, string]
): AccentPalette {
  const rgb = parseColorToRgb(input);
  const primaryHex = rgbToHex(rgb.r, rgb.g, rgb.b);
  const luminance = getPerceivedLuminance(rgb.r, rgb.g, rgb.b);

  const onPrimary = luminance >= 135 ? '#111215' : '#ffffff';

  const hoverRgb = luminance < 40
    ? mixRgb(rgb, { r: 255, g: 255, b: 255 }, 0.18)
    : mixRgb(rgb, { r: 0, g: 0, b: 0 }, 0.12);
  const primaryHover = rgbToHex(hoverRgb.r, hoverRgb.g, hoverRgb.b);

  const container = `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.16)`;

  let onContainer = primaryHex;
  if (luminance < 95) {
    const lifted = mixRgb(rgb, { r: 255, g: 255, b: 255 }, 0.45);
    onContainer = rgbToHex(lifted.r, lifted.g, lifted.b);
  }

  const subtle = `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.22)`;
  const glow = `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.35)`;

  let quadrants: [string, string, string, string];
  const trimmed = input.trim().toLowerCase();
  if (customQuadrants) {
    quadrants = customQuadrants;
  } else if (PRESET_QUADRANTS[trimmed]) {
    quadrants = PRESET_QUADRANTS[trimmed];
  } else {
    const q1 = primaryHex;
    const q2Rgb = mixRgb(rgb, { r: 255, g: 255, b: 255 }, 0.45);
    const q3Rgb = mixRgb(rgb, { r: 0, g: 0, b: 0 }, 0.55);
    const q4Rgb = mixRgb(rgb, { r: 0, g: 0, b: 0 }, 0.25);
    const q2 = rgbToHex(q2Rgb.r, q2Rgb.g, q2Rgb.b);
    const q3 = rgbToHex(q3Rgb.r, q3Rgb.g, q3Rgb.b);
    const q4 = rgbToHex(q4Rgb.r, q4Rgb.g, q4Rgb.b);
    quadrants = [q1, q2, q3, q4];
  }

  const choiceActiveBg = quadrants[1];

  const inactiveRgb = mixRgb(rgb, { r: 0, g: 0, b: 0 }, 0.82);
  const choiceInactiveBg = rgbToHex(inactiveRgb.r, inactiveRgb.g, inactiveRgb.b);

  const activeTextRgb = mixRgb(rgb, { r: 0, g: 0, b: 0 }, 0.90);
  const choiceActiveText = rgbToHex(activeTextRgb.r, activeTextRgb.g, activeTextRgb.b);

  const inactiveLum = getPerceivedLuminance(inactiveRgb.r, inactiveRgb.g, inactiveRgb.b);
  const choiceInactiveText = inactiveLum < 128 ? '#ffffff' : '#111215';

  return {
    primary: primaryHex,
    primaryHover,
    onPrimary,
    container,
    onContainer,
    subtle,
    glow,
    quadrants,
    choiceActiveBg,
    choiceActiveText,
    choiceInactiveBg,
    choiceInactiveText
  };
}
