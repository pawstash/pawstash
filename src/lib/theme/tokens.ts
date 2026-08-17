export type FontSizeScale = 'compact' | 'standard' | 'large';
export type RadiusScale = 'sharp' | 'smooth' | 'rounded' | 'pill';
export type SurfaceStyle = 'glass' | 'oled' | 'acrylic';
export type AccentColor = 'violet' | 'emerald' | 'amber' | 'rose' | 'cyan' | 'indigo' | (string & {});
export type MotionSpeed = 'instant' | 'snappy' | 'smooth';

export interface ThemeTokens {
  fontScale: FontSizeScale;
  radiusScale: RadiusScale;
  surfaceStyle: SurfaceStyle;
  accent: AccentColor;
  motionSpeed: MotionSpeed;
  backdropBlurPx: number;
  borderWidthPx: number;
  titlebarHeightPx: number;
  sidebarWidthPx: number;
}

export const FONT_SCALE_MAP: Record<FontSizeScale, { xs: string; sm: string; base: string; lg: string; xl: string }> = {
  compact: { xs: '11px', sm: '13px', base: '14px', lg: '16px', xl: '18px' },
  standard: { xs: '12px', sm: '14px', base: '16px', lg: '18px', xl: '20px' },
  large: { xs: '13px', sm: '15px', base: '17px', lg: '19px', xl: '22px' },
};

export const RADIUS_SCALE_MAP: Record<RadiusScale, { sm: string; md: string; lg: string; xl: string }> = {
  sharp: { sm: '2px', md: '4px', lg: '6px', xl: '8px' },
  smooth: { sm: '6px', md: '10px', lg: '14px', xl: '20px' },
  rounded: { sm: '10px', md: '16px', lg: '22px', xl: '28px' },
  pill: { sm: '9999px', md: '9999px', lg: '9999px', xl: '9999px' },
};

export const ACCENT_COLOR_MAP: Record<AccentColor, { primary: string; hover: string; glow: string }> = {
  violet: { primary: '#8b5cf6', hover: '#7c3aed', glow: 'rgba(139, 92, 246, 0.35)' },
  emerald: { primary: '#10b981', hover: '#059669', glow: 'rgba(16, 185, 129, 0.35)' },
  amber: { primary: '#f59e0b', hover: '#d97706', glow: 'rgba(245, 158, 11, 0.35)' },
  rose: { primary: '#f43f5e', hover: '#e11d48', glow: 'rgba(244, 63, 94, 0.35)' },
  cyan: { primary: '#06b6d4', hover: '#0891b2', glow: 'rgba(6, 182, 212, 0.35)' },
  indigo: { primary: '#6366f1', hover: '#4f46e5', glow: 'rgba(99, 102, 241, 0.35)' },
};

export const MOTION_SPEED_MAP: Record<MotionSpeed, { fast: string; normal: string; slow: string }> = {
  instant: { fast: '0ms', normal: '50ms', slow: '100ms' },
  snappy: { fast: '80ms', normal: '150ms', slow: '250ms' },
  smooth: { fast: '120ms', normal: '220ms', slow: '400ms' },
};
