import { configState } from './configState.svelte';

export class LayoutState {
  screenWidth = $state(typeof window !== 'undefined' ? window.innerWidth : 1024);
  isMobileDevice = typeof navigator !== 'undefined' && /Android|iPhone|iPad|iPod|IEMobile/i.test(navigator.userAgent);
  isAndroid = typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent);
  isMacOS = typeof navigator !== 'undefined' && /Macintosh|Mac OS X|MacPPC|MacIntel/i.test(navigator.userAgent);

  constructor() {
    if (typeof window !== 'undefined') {
      window.addEventListener('resize', () => {
        this.screenWidth = window.innerWidth;
      });
    }
  }

  isMobile = $derived.by(() => {
    const mode = configState.settings.layout_mode || 'auto';
    if (mode === 'mobile') return true;
    if (mode === 'desktop') return false;
    return this.screenWidth <= 768;
  });

  showStickyHeaderBg = $derived.by(() => {
    const mode = configState.settings.layout_mode || 'auto';
    if (mode === 'mobile') return true;
    if (mode === 'desktop') return false;
    return this.isMobileDevice && this.screenWidth <= 768;
  });

  SIDEBAR_STORAGE_KEY = 'pawstash_sidebar_compact';

  isSidebarCompact = $state(
    typeof localStorage !== 'undefined' ? localStorage.getItem('pawstash_sidebar_compact') === 'true' : false
  );

  toggleSidebar() {
    this.isSidebarCompact = !this.isSidebarCompact;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(this.SIDEBAR_STORAGE_KEY, String(this.isSidebarCompact));
    }
  }

  setSidebarCompact(compact: boolean) {
    this.isSidebarCompact = compact;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(this.SIDEBAR_STORAGE_KEY, String(this.isSidebarCompact));
    }
  }

  effectiveTitlebarStyle = $derived.by((): 'windows' | 'macos' => {
    const mode = configState.settings.titlebar_style || 'auto';
    if (mode === 'macos') return 'macos';
    if (mode === 'windows') return 'windows';
    return this.isMacOS ? 'macos' : 'windows';
  });
}

export const layoutState = new LayoutState();
