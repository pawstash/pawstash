let dismissed = false;

export function syncBootTheme() {
  if (typeof document === 'undefined' || typeof localStorage === 'undefined') return;
  try {
    const tokens = JSON.parse(localStorage.getItem('pawstash_theme_settings') || '{}');
    const mode = tokens.colorMode || 'dark';
    const dark =
      mode !== 'light' &&
      !(mode === 'system' && window.matchMedia('(prefers-color-scheme: light)').matches);
    const root = document.documentElement;
    root.classList.toggle('dark', dark);
    root.classList.toggle('light', !dark);

    const palette = JSON.parse(localStorage.getItem('pawstash_theme_palette') || 'null');
    const screen = document.getElementById('boot-screen');
    if (!palette || !screen) return;
    screen.style.background = palette.surface;
    screen.style.color = palette.textPrimary;
    const track = screen.querySelector<HTMLElement>('.boot-track');
    const bar = track?.querySelector<HTMLElement>('span');
    if (track) track.style.background = palette.surfaceContainerHigh;
    if (bar) bar.style.background = palette.primary;
  } catch {}
}

export function dismissBootScreen() {
  if (dismissed || typeof document === 'undefined') return;
  dismissed = true;
  const screen = document.getElementById('boot-screen');
  if (!screen) return;
  screen.classList.add('is-hidden');
  window.setTimeout(() => screen.remove(), 220);
}
