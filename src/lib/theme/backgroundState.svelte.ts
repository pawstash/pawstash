import { invoke } from '@tauri-apps/api/core';
import { type as osType, version as osVersion } from '@tauri-apps/plugin-os';

export type BackgroundType =
  | 'oled'
  | 'acrylic'
  | 'vibrancy'
  | 'mica-dark'
  | 'tabbed'
  | 'custom';

export type CustomBackgroundKind = 'color' | 'image' | 'video';

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
}

export class BackgroundState {
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
    saturation: 1.2
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
      saturation: 1.2
    } satisfies BackgroundSettings);
    void this.applyWindowEffect(this.settings.type);
    this.save();
  }

  setType(type: BackgroundType) {
    this.settings.type = type;
    if (type === 'custom' || type === 'oled') {
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
    this.applyWindowEffect('none');
    this.save();
  }

  setVideoUrl(url: string) {
    this.settings.videoUrl = url;
    this.settings.customKind = 'video';
    this.settings.type = 'custom';
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
      console.warn('Failed to remove custom background file:', error);
    });
  }

  private async applyWindowEffect(effectType: string) {
    try {
      await invoke('set_window_effect', { effectType });
    } catch (e) {
      console.warn('Native window effect not supported on this platform:', e);
    }
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
        } catch (e) {}
      }
    }
    if (!supportedBackgroundTypes().includes(this.settings.type)) {
      this.settings.type = defaultBackgroundType();
      this.save();
    }
    this.applyWindowEffect(this.settings.type);
  }
}

export const backgroundState = new BackgroundState();

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
        ? ['acrylic', 'mica-dark', 'tabbed', 'oled', 'custom']
        : ['acrylic', 'oled', 'custom'];
    }
    if (platform === 'macos') return ['oled', 'vibrancy', 'custom'];
  } catch {
  }
  return ['oled', 'custom'];
}

export function defaultBackgroundType(): BackgroundType {
  return isWindowsPlatform() ? 'acrylic' : 'oled';
}
