export interface PressOptions {
  scale?: number;
  ripple?: boolean;
  disabled?: boolean;
}

export interface HoverOptions {
  spotlight?: boolean;
  glowColor?: string;
  disabled?: boolean;
}

export type MotionPreset = 'button' | 'window-control' | 'sidebar-item' | 'card' | 'tab';

export interface MotionOptions {
  preset?: MotionPreset;
  ripple?: boolean;
  spotlight?: boolean;
  glowColor?: string;
  disabled?: boolean;
}

