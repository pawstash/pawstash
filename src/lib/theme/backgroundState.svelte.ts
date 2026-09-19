import { invoke } from '@tauri-apps/api/core';
import { type as osType, version as osVersion } from '@tauri-apps/plugin-os';
import { parseColorToRgb, mixRgb, computeAcrylicTintRgb } from './palette';
import { logger } from '$lib/utils/logger';

export type BackgroundType =
  | 'oled'
  | 'palette'
  | 'acrylic'
  | 'vibrancy'
  | 'mica-dark'
  | 'tabbed'
  | 'custom';

export type CustomBackgroundKind = 'color' | 'image' | 'video' | 'palette';

export interface BackgroundSettings {
  type: BackgroundType;
  customKind: CustomBackgroundKind;
  solidColor: string;
  gradientSecondary: string;
  imageUrl: string;
  videoUrl: string;
  blurPx: number;
  opacity: number;
  brightness: number;
  saturation: number;
  acrylicTint: boolean;
  acrylicOpacity: number;
}

export class BackgroundState {
  version = $state(0);
  settings = $state<BackgroundSettings>({
    type: defaultBackgroundType(),
    customKind: 'color',
    solidColor: '#000000',
    gradientSecondary: '#111827',
    imageUrl: '',
    videoUrl: '',
    blurPx: 24,
    opacity: 0.85,
    brightness: 0.5,
    saturation: 1.2,
    acrylicTint: false,
    acrylicOpacity: 0.5
  });

  reset() {
    Object.assign(this.settings, {
      type: defaultBackgroundType(),
      customKind: 'color',
      solidColor: '#000000',
      gradientSecondary: '#111827',
      imageUrl: '',
      videoUrl: '',
      blurPx: 24,
      opacity: 0.85,
      brightness: 0.5,
      saturation: 1.2,
      acrylicTint: false,
      acrylicOpacity: defaultAcrylicOpacity()
    } satisfies BackgroundSettings);
    this.version++;
    void this.applyWindowEffect(this.settings.type);
    this.save();
  }

  setType(type: BackgroundType) {
    this.settings.type = type;
    if (type === 'custom' || type === 'oled' || type === 'palette') {
      this.applyWindowEffect('none');
    } else {
      this.applyWindowEffect(type);
    }
    this.save();
  }

  setImageUrl(url: string) {
    this.settings.imageUrl = url;
    this.settings.customKind = 'image';
    this.settings.type = 'custom';
    this.version++;
    this.applyWindowEffect('none');
    this.save();
  }

  setVideoUrl(url: string) {
    this.settings.videoUrl = url;
    this.settings.customKind = 'video';
    this.settings.type = 'custom';
    this.version++;
    this.applyWindowEffect('none');
    this.save();
  }

  setBlur(blurPx: number) {
    this.settings.blurPx = blurPx;
    this.save();
  }

  setOpacity(opacity: number) {
    this.settings.opacity = opacity;
    this.save();
  }

  setAcrylicTint(acrylicTint: boolean) {
    this.settings.acrylicTint = acrylicTint;
    this.save();
    windowTint(this.settings.acrylicOpacity);
    if (this.settings.type === 'acrylic') {
      void this.applyWindowEffect('acrylic');
    }
  }

  setAcrylicOpacity(acrylicOpacity: number) {
    this.settings.acrylicOpacity = acrylicOpacity;
    this.save();
    windowTint(acrylicOpacity);
    if (this.settings.type === 'acrylic') {
      void this.applyWindowEffect('acrylic');
    }
  }

  setSolidColor(color: string) {
    this.settings.solidColor = color;
    this.save();
  }

  setGradientSecondary(color: string) {
    this.settings.gradientSecondary = color;
    this.save();
  }

  setBrightness(brightness: number) {
    this.settings.brightness = brightness;
    this.save();
  }

  setSaturation(saturation: number) {
    this.settings.saturation = saturation;
    this.save();
  }

  setCustomKind(kind: CustomBackgroundKind) {
    this.settings.customKind = kind;
    this.settings.type = 'custom';
    void this.applyWindowEffect('none');
    this.save();
  }

  clearCustomMedia(kind: 'image' | 'video') {
    if (kind === 'image') this.settings.imageUrl = '';
    else this.settings.videoUrl = '';
    this.save();
    void invoke('clear_custom_background', { kind }).catch((error) => {
      logger.warn('Failed to remove custom background file', error);
    });
  }

  private async applyWindowEffect(effectType: string) {
    try {
      await invoke('set_window_effect', {
        effectType: forColorMode(effectType),
        tint: windowTint(this.settings.acrylicOpacity)
      });
    } catch (e) {
      logger.warn('Native window effect not supported on this platform', e);
    }
  }

  refreshWindowEffect() {
    const { type } = this.settings;
    if (type === 'custom' || type === 'oled' || type === 'palette') return;
    void this.applyWindowEffect(type);
  }

  private save() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('pawstash_bg_settings', JSON.stringify(this.settings));
    }
  }

  init() {
    if (typeof localStorage !== 'undefined') {
      const saved = localStorage.getItem('pawstash_bg_settings');
      if (saved) {
        try {
          Object.assign(this.settings, JSON.parse(saved));
        } catch (e) {
          logger.warn('[BackgroundState] Failed to parse saved settings:', e);
        }
      }
    }
    if (typeof this.settings.acrylicTint !== 'boolean') {
      this.settings.acrylicTint = false;
    }
    if (typeof this.settings.acrylicOpacity !== 'number') {
      this.settings.acrylicOpacity = defaultAcrylicOpacity();
    }
    if (!supportedBackgroundTypes().includes(this.settings.type)) {
      this.settings.type = defaultBackgroundType();
      this.save();
    }
    this.applyWindowEffect(this.settings.type);
  }
}

export const backgroundState = new BackgroundState();

const LIGHT_EFFECT_TWINS: Record<string, string> = {
  'mica-dark': 'mica-light',
  tabbed: 'tabbed-light'
};

function windowTint(acrylicOpacity: number = defaultAcrylicOpacity()): [number, number, number, number] | undefined {
  if (typeof document === 'undefined') return undefined;
  const styles = getComputedStyle(document.documentElement);
  const isDark = document.documentElement.classList.contains('dark');
  const accent = styles.getPropertyValue('--accent-primary').trim() || '#d69085';
  const mixed = computeAcrylicTintRgb(accent, isDark);

  const opacity = typeof acrylicOpacity === 'number' ? acrylicOpacity : defaultAcrylicOpacity(isDark);
  const alpha = Math.min(255, Math.max(0, Math.round(opacity * 255)));

  document.documentElement.style.setProperty(
    '--acrylic-tint',
    `rgba(${mixed.r}, ${mixed.g}, ${mixed.b}, ${opacity.toFixed(2)})`
  );
  document.documentElement.style.setProperty('--acrylic-tint-rgb', `${mixed.r}, ${mixed.g}, ${mixed.b}`);
  document.documentElement.style.setProperty('--acrylic-opacity', opacity.toFixed(2));

  if (!backgroundState.settings.acrylicTint) {
    return undefined;
  }

  return [mixed.r, mixed.g, mixed.b, alpha];
}

function forColorMode(effectType: string): string {
  if (typeof document === 'undefined') return effectType;
  const isDark = document.documentElement.classList.contains('dark');
  return isDark ? effectType : (LIGHT_EFFECT_TWINS[effectType] ?? effectType);
}

export function isWindowsPlatform(): boolean {
  try {
    return osType() === 'windows';
  } catch {
    if (typeof navigator !== 'undefined') {
      return navigator.userAgent.toLowerCase().includes('windows');
    }
    return false;
  }
}

export function supportedBackgroundTypes(): BackgroundType[] {
  try {
    const platform = osType();
    if (platform === 'windows') {
      const parts = osVersion().split('.').map((part) => Number.parseInt(part, 10) || 0);
      const supportsWindows11Effects = parts[0] >= 11 || (parts[0] === 10 && (parts[2] ?? 0) >= 22000);
      return supportsWindows11Effects
        ? ['acrylic', 'mica-dark', 'tabbed', 'palette', 'oled', 'custom']
        : ['acrylic', 'palette', 'oled', 'custom'];
    }
    if (platform === 'macos') return ['palette', 'oled', 'vibrancy', 'custom'];
  } catch {
  }
  return ['palette', 'oled', 'custom'];
}

export function defaultBackgroundType(): BackgroundType {
  return isWindowsPlatform() ? 'acrylic' : 'oled';
}

export function defaultAcrylicOpacity(isDark?: boolean): number {
  if (typeof isDark === 'boolean') return isDark ? 0.5 : 0.7;
  if (typeof document !== 'undefined') {
    return document.documentElement.classList.contains('light') ? 0.7 : 0.5;
  }
  return 0.5;
}
