<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import { motion, tooltip } from '$lib/motion';
  import { syncState } from '$lib/state/syncState.svelte';
  import { openExternalUrl } from '$lib/utils/ipc';
  import { scrollable } from '$lib/actions/scrollable';
  import IconFeed from '~icons/fluent/grid-24-regular';
  import IconFeedFilled from '~icons/fluent/grid-24-filled';
  import IconCreators from '~icons/fluent/people-24-regular';
  import IconCreatorsFilled from '~icons/fluent/people-24-filled';
  import IconFavorites from '~icons/fluent/heart-24-regular';
  import IconFavoritesFilled from '~icons/fluent/heart-24-filled';
  import IconLibrary from '~icons/fluent/library-24-regular';
  import IconLibraryFilled from '~icons/fluent/library-24-filled';
  import ProgressRing from '$lib/components/ui/ProgressRing.svelte';
  import IconDownloads from '~icons/fluent/arrow-download-24-regular';
  import IconDownloadsFilled from '~icons/fluent/arrow-download-24-filled';
  import IconSettings from '~icons/fluent/settings-24-regular';
  import IconSettingsFilled from '~icons/fluent/settings-24-filled';
  import IconDockLeft from '~icons/fluent/dock-left-24-regular';
  import IconChevronLeft from '~icons/fluent/chevron-left-24-regular';
  import IconChevronRight from '~icons/fluent/chevron-right-24-regular';
  import IconTelegram from '~icons/simple-icons/telegram';
  import IconDiscord from '~icons/simple-icons/discord';
  import IconReddit from '~icons/simple-icons/reddit';
  import IconGithub from '~icons/simple-icons/github';
  import IconUser from '~icons/fluent/person-24-regular';
  import IconCloudSync from '~icons/fluent/cloud-sync-24-regular';
  import StableWeightLabel from '$lib/components/ui/StableWeightLabel.svelte';
  import pawstashLogo from '$lib/assets/pawstash.png';
  import { logoFlightState } from '$lib/state/logoFlightState.svelte';
  import { themeState } from '$lib/theme/themeState.svelte';

  interface NavItem {
    id: 'feed' | 'favorites' | 'library' | 'creators' | 'downloads' | 'settings';
    labelKey: string;
    icon: any;
    iconActive: any;
    badge?: () => number;
  }

  const navItems: NavItem[] = [
    { id: 'feed', labelKey: 'nav.feed', icon: IconFeed, iconActive: IconFeedFilled },
    { id: 'creators', labelKey: 'nav.creators', icon: IconCreators, iconActive: IconCreatorsFilled },
    { id: 'favorites', labelKey: 'nav.favorites', icon: IconFavorites, iconActive: IconFavoritesFilled },
    { id: 'library', labelKey: 'nav.library', icon: IconLibrary, iconActive: IconLibraryFilled },
    { id: 'downloads', labelKey: 'nav.downloads', icon: IconDownloads, iconActive: IconDownloadsFilled, badge: () => downloadState.activeDownloadsCount },
    { id: 'settings', labelKey: 'nav.settings', icon: IconSettings, iconActive: IconSettingsFilled }
  ];

  const appWindow = getCurrentWindow();
  let isMaximized = $state(false);
  let isMacStyle = $derived(layoutState.effectiveTitlebarStyle === 'macos');
  let isCompact = $derived(layoutState.isSidebarCompact);
  let activeRoot = $derived(navigationState.activeRoot);
  let isTextLogo = $derived(
    activeRoot === 'settings' || (logoFlightState.isFlying && logoFlightState.direction === 'toSidebar')
  );
  let sidebarLogoEl = $state<HTMLElement | null>(null);

  $effect(() => {
    logoFlightState.registerSidebar(sidebarLogoEl);
    if (typeof document !== 'undefined') {
      document.documentElement.style.removeProperty('--sidebar-width-expanded');
    }
  });

  function minimize() {
    appWindow.minimize();
  }

  function toggleMaximize() {
    appWindow.toggleMaximize();
  }

  function close() {
    appWindow.close();
  }


  onMount(() => {
    void appWindow.isMaximized().then((val) => {
      isMaximized = val;
    });

    const unlistenPromise = appWindow.onResized(() => {
      void appWindow.isMaximized().then((val) => {
        isMaximized = val;
      });
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
        const target = e.target as HTMLElement | null;
        if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) {
          return;
        }
        e.preventDefault();
        layoutState.toggleSidebar();
      }
    };
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
      window.removeEventListener('keydown', handleKeyDown);
    };
  });

  let profileName = $derived(
    syncState.status.account_id || i18n.t('profile.local')
  );

  let profileTooltipText = $derived.by(() => {
    if (syncState.isSyncing) {
      if (syncState.progress !== null && syncState.progress !== undefined) {
        return `${profileName} (${Math.round(syncState.progress * 100)}%)`;
      }
      return `${profileName} (${i18n.t('sync.status_syncing')})`;
    }
    return profileName;
  });

  let profileSub = $derived.by(() => {
    if (syncState.isSyncing) {
      if (syncState.progress !== null && syncState.progress !== undefined) {
        const pct = Math.round(syncState.progress * 100);
        if (syncState.phase === 'pushing') {
          return `${i18n.t('sync.status_pushing')} ${pct}%`;
        }
        return `${i18n.t('sync.status_pulling')} ${pct}%`;
      }
      if (syncState.phase === 'connecting') return i18n.t('sync.status_connecting');
      if (syncState.phase === 'pulling') return i18n.t('sync.status_pulling');
      if (syncState.phase === 'pushing') return i18n.t('sync.status_pushing');
      return i18n.t('sync.status_syncing');
    }
    if (syncState.status.configured) {
      if (!syncState.status.unlocked) return i18n.t('sync.locked');
      return i18n.t('sync.title');
    }
    return i18n.t('profile.offline_session');
  });

  function hasProgressRing(id: string): boolean {
    return id === 'downloads' && downloadState.activeDownloadsCount > 0;
  }

</script>

<aside
  data-tauri-drag-region
  class="sidebar-aside"
  class:compact={isCompact}
  class:mac-style={isMacStyle}
>
  {#if isMacStyle}
    <div class="sidebar-traffic-lights" data-tauri-drag-region="false">
      <button
        data-tauri-drag-region="false"
        use:tooltip={i18n.t('actions.close')}
        onclick={close}
        class="mac-light mac-close"
        aria-label={i18n.t('actions.close')}
      >
        <svg class="mac-glyph" viewBox="0 0 6 6" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M1 1L5 5M5 1L1 5" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
        </svg>
      </button>
      <button
        data-tauri-drag-region="false"
        use:tooltip={i18n.t('actions.minimize')}
        onclick={minimize}
        class="mac-light mac-minimize"
        aria-label={i18n.t('actions.minimize')}
      >
        <svg class="mac-glyph" viewBox="0 0 6 6" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M0.75 3H5.25" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
        </svg>
      </button>
      <button
        data-tauri-drag-region="false"
        use:tooltip={i18n.t(isMaximized ? 'actions.restore' : 'actions.maximize')}
        onclick={toggleMaximize}
        class="mac-light mac-maximize"
        aria-label={i18n.t(isMaximized ? 'actions.restore' : 'actions.maximize')}
      >
        <svg class="mac-glyph" viewBox="0 0 6 6" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M1 1H4L1 4V1Z" fill="currentColor"/>
          <path d="M5 5H2L5 2V5Z" fill="currentColor"/>
        </svg>
      </button>
    </div>
  {/if}

  <div
    data-overlayscrollbars-initialize
    class="sidebar-scrollable"
    use:scrollable={{ overflowX: 'hidden' }}
    data-tauri-drag-region
  >
    <div class="sidebar-content" data-tauri-drag-region>
      <button
    data-tauri-drag-region="false"
    use:motion={'sidebar-item'}
    onclick={() => navigationState.navigateRoot('feed')}
    class="sidebar-btn logo-btn"
    aria-label="Pawstash Logo"
    use:tooltip={isCompact ? { text: 'Pawstash', placement: 'right' } : undefined}
  >
    <div class="sidebar-icon">
      <svg viewBox="0 0 602 602" fill="none" class="logo-svg" xmlns="http://www.w3.org/2000/svg">
        <defs>
          <linearGradient id="logo-grad" x1="301" y1="0" x2="-2.17166e-05" y2="584.337" gradientUnits="userSpaceOnUse">
            <stop stop-color="#FCD8D2"/>
            <stop offset="1" stop-color="#FEB8AD"/>
          </linearGradient>
        </defs>
        <g transform="translate(0, 8.5)">
          <path d="M130.548 56.3212L414.821 178.14L301 226.902L18.361 105.771C24.725 99.2782 32.508 94.0322 41.366 90.6352L130.548 56.3212ZM188.082 34.2192L254.732 8.59119C284.529 -2.86373 317.514 -2.86373 347.311 8.59119L560.677 90.6352C569.492 94.0752 577.275 99.2352 583.639 105.771L469.431 154.705L188.082 34.2192ZM601.742 144.815L322.5 264.484V584.834C330.957 583.401 339.227 581.136 347.311 578.04L560.677 495.953C572.841 491.269 583.301 483.01 590.677 472.264C598.054 461.517 602.002 448.788 602 435.753V150.835C602 148.829 601.9 146.822 601.699 144.815M279.5 584.834V264.484L0.300999 144.815C0.130354 146.818 0.0299598 148.826 0 150.835V435.753C0.00172613 448.793 3.95568 461.526 11.3404 472.273C18.7252 483.02 29.1939 491.276 41.366 495.953L254.689 578.04C262.773 581.136 271.043 583.401 279.5 584.834Z" />
        </g>
      </svg>
    </div>
    <span class="sidebar-label logo-label">
      <span class="logo-swap logo-swap-text" class:visible={isTextLogo}>
        Pawstash
      </span>
      <span class="logo-swap logo-swap-img" class:visible={!isTextLogo}>
        <img
          bind:this={sidebarLogoEl}
          src={pawstashLogo}
          alt="Pawstash"
          class="logo-text-img"
          class:is-flying={logoFlightState.isFlying}
        />
      </span>
    </span>
  </button>

  <button
    data-tauri-drag-region="false"
    use:motion={'sidebar-item'}
    class="sidebar-btn profile-btn"
    class:active={activeRoot === 'profile'}
    aria-label={profileName}
    onclick={() => navigationState.navigateRoot('profile')}
    use:tooltip={isCompact ? { text: profileTooltipText, placement: 'right' } : undefined}
  >
    <div
      class="sidebar-icon relative"
      class:has-progress={syncState.isSyncing}
    >
      {#if syncState.isSyncing}
        <ProgressRing value={syncState.progress} variant="segmented" size={32} />
      {/if}
      {#if syncState.status.configured || syncState.isSyncing}
        <IconCloudSync />
      {:else}
        <IconUser />
      {/if}
    </div>
    <span class="sidebar-label profile-label">
      <span class="profile-name">{profileName}</span>
      <span class="profile-sub">{profileSub}</span>
    </span>
  </button>
  <div class="profile-divider"></div>

  <nav data-tauri-drag-region class="sidebar-nav">
    {#each navItems as item}
      {@const isActive = activeRoot === item.id}
      {@const title = i18n.t(item.labelKey)}
      <button
        data-tauri-drag-region="false"
        use:motion={'sidebar-item'}
        onclick={() => navigationState.navigateRoot(item.id)}
        class="sidebar-btn"
        class:active={isActive}
        aria-label={title}
      >
        <div
          class="sidebar-icon"
          class:active={isActive}
          class:has-progress={hasProgressRing(item.id)}
        >
          {#if hasProgressRing(item.id)}
            <ProgressRing value={downloadState.activeProgress} size={32} />
          {/if}
          {#if isActive}
            <item.iconActive />
          {:else}
            <item.icon />
          {/if}

          {#if item.badge && item.badge() > 0 && !hasProgressRing(item.id)}
            <span class="sidebar-badge">{item.badge()}</span>
          {/if}
        </div>

        <span class="sidebar-label" class:active={isActive}>
          <StableWeightLabel text={title} />
        </span>
      </button>
    {/each}
  </nav>

  <div data-tauri-drag-region class="sidebar-bottom">
    <button
      type="button"
      data-tauri-drag-region="false"
      use:motion={'sidebar-item'}
      onclick={() => openExternalUrl('https://t.me/pawstashapp')}
      class="sidebar-btn"
      aria-label="Telegram"
      use:tooltip={isCompact ? { text: 'Telegram', placement: 'right' } : undefined}
    >
      <div class="sidebar-icon">
        <IconTelegram />
      </div>
      <span class="sidebar-label">Telegram</span>
    </button>

    <button
      type="button"
      data-tauri-drag-region="false"
      use:motion={'sidebar-item'}
      onclick={() => openExternalUrl('https://discord.gg/ahcx8ub5Ck')}
      class="sidebar-btn"
      aria-label="Discord"
      use:tooltip={isCompact ? { text: 'Discord', placement: 'right' } : undefined}
    >
      <div class="sidebar-icon">
        <IconDiscord />
      </div>
      <span class="sidebar-label">Discord</span>
    </button>

    <button
      type="button"
      data-tauri-drag-region="false"
      use:motion={'sidebar-item'}
      onclick={() => openExternalUrl('https://reddit.com/r/pawstash')}
      class="sidebar-btn"
      aria-label="Reddit"
      use:tooltip={isCompact ? { text: 'Reddit', placement: 'right' } : undefined}
    >
      <div class="sidebar-icon">
        <IconReddit />
      </div>
      <span class="sidebar-label">Reddit</span>
    </button>

    <button
      type="button"
      data-tauri-drag-region="false"
      use:motion={'sidebar-item'}
      onclick={() => openExternalUrl('https://github.com/pawstash')}
      class="sidebar-btn"
      aria-label="GitHub"
      use:tooltip={isCompact ? { text: 'GitHub', placement: 'right' } : undefined}
    >
      <div class="sidebar-icon">
        <IconGithub />
      </div>
      <span class="sidebar-label">GitHub</span>
    </button>
  </div>
    </div>
  </div>
</aside>

<style>
  .sidebar-btn.logo-btn {
    height: 44px;
    padding: 0 16px 0 11px;
    gap: 10px;
    margin-bottom: 8px;
    width: max-content;
    max-width: 100%;
    box-sizing: border-box;
    overflow: hidden;
    flex-shrink: 0;
    transition: background 200ms ease, color 200ms ease, padding 300ms cubic-bezier(0.16, 1, 0.3, 1), gap 300ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .sidebar-aside.compact .sidebar-btn.logo-btn {
    padding-right: 11px;
    gap: 0;
  }

  .sidebar-btn.logo-btn .sidebar-icon {
    opacity: 0.95;
  }

  .sidebar-btn.logo-btn:hover .sidebar-icon {
    opacity: 1;
  }

  .logo-svg {
    width: 22px;
    height: 22px;
    opacity: 0.95;
    transition: opacity var(--duration-fast) var(--ease-expo);
  }

  .logo-svg path {
    fill: var(--text-primary);
    transition: fill var(--duration-normal) var(--ease-expo);
  }

  .sidebar-btn.logo-btn:hover .logo-svg path {
    fill: url(#logo-grad);
  }

  .sidebar-btn.logo-btn:hover .logo-svg {
    opacity: 1;
  }

  .sidebar-btn.logo-btn .logo-label {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    min-width: 0;
    flex: 1 1 auto;
    width: 82px;
    max-width: 82px;
    height: 100%;
    font-weight: 600;
    font-size: 15px;
    letter-spacing: 0.03em;
    color: var(--text-primary);
    opacity: 0.95;
    white-space: nowrap;
    overflow: hidden;
    pointer-events: none;
    transition: width 300ms cubic-bezier(0.16, 1, 0.3, 1), max-width 300ms cubic-bezier(0.16, 1, 0.3, 1), opacity 200ms ease;
  }

  .logo-swap {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    opacity: 0;
    pointer-events: none;
    transform: translateY(4px) scale(0.96);
    transition: opacity 260ms cubic-bezier(0.16, 1, 0.3, 1), transform 260ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .logo-swap.visible {
    opacity: 1;
    transform: translateY(0) scale(1);
    pointer-events: auto;
  }

  .sidebar-btn.logo-btn:hover .logo-label {
    opacity: 1;
  }

  .logo-text-img {
    height: 33px;
    width: auto;
    max-width: 100%;
    object-fit: contain;
    object-position: left center;
    user-select: none;
    -webkit-user-drag: none;
    pointer-events: none;
    flex-shrink: 0;
    transition: opacity 160ms ease;
  }

  .logo-text-img.is-flying {
    opacity: 0;
  }

  :global(.light) .logo-text-img {
    filter: invert(1);
  }

  .profile-btn {
    height: 44px !important;
    margin-bottom: 4px;
    padding: 0 16px 0 11px !important;
    gap: 10px !important;
    width: max-content !important;
    max-width: 100% !important;
    box-sizing: border-box;
    overflow: hidden !important;
    transition: background 200ms ease, color 200ms ease, padding 300ms cubic-bezier(0.16, 1, 0.3, 1), gap 300ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .sidebar-aside.compact .profile-btn {
    padding-right: 11px !important;
    gap: 0 !important;
  }

  .profile-label {
    display: flex !important;
    flex-direction: column !important;
    align-items: flex-start !important;
    justify-content: center !important;
    text-align: left !important;
    gap: 1px;
    line-height: 1.2 !important;
    min-width: 0 !important;
    flex: 1 1 0% !important;
    max-width: 100% !important;
    overflow: hidden !important;
    pointer-events: none;
    transition: max-width 300ms cubic-bezier(0.16, 1, 0.3, 1), opacity 200ms ease;
  }

  .profile-name {
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
    min-width: 0;
  }

  .profile-sub {
    font-size: 10px;
    font-weight: 400;
    color: var(--text-muted);
    letter-spacing: 0.02em;
    text-transform: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
    min-width: 0;
  }

  .profile-divider {
    height: 1px;
    width: calc(100% - 12px);
    max-width: calc(100% - 12px);
    margin: 4px 0 8px 0;
    background: rgba(var(--surface-tint-rgb), 0.08);
    transition: width 300ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .sidebar-aside.compact .profile-divider {
    width: 44px;
  }

  .sidebar-aside {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    height: 100%;
    width: var(--sidebar-width-expanded, 160px);
    padding: 0;
    background: rgba(var(--surface-tint-rgb), 0.02);
    border-right: 1px solid rgba(var(--surface-tint-rgb), 0.04);
    user-select: none;
    flex-shrink: 0;
    border-top: none;
    border-left: none;
    border-bottom: none;
    outline: none;
    cursor: default;
    box-sizing: border-box;
    overflow: hidden;
    transition: width 300ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .sidebar-aside.compact {
    width: var(--sidebar-width-collapsed, 56px);
  }

  .sidebar-scrollable {
    flex: 1 1 0%;
    min-height: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .sidebar-scrollable :global(.os-viewport) {
    display: flex !important;
    flex-direction: column !important;
    height: 100% !important;
    width: 100% !important;
    box-sizing: border-box !important;
  }

  .sidebar-scrollable :global(.os-content) {
    display: flex !important;
    flex-direction: column !important;
    flex: 1 1 auto !important;
    min-height: 100% !important;
    width: 100% !important;
    box-sizing: border-box !important;
  }

  .sidebar-content {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    flex: 1 1 auto;
    min-height: 100%;
    width: 100%;
    box-sizing: border-box;
    padding: 12px 6px;
  }

  .sidebar-nav {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    width: 100%;
    max-width: 100%;
    box-sizing: border-box;
    min-width: 0;
  }

  .sidebar-bottom {
    margin-top: auto;
    padding-top: 8px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    width: 100%;
    max-width: 100%;
    box-sizing: border-box;
    min-width: 0;
  }

  .sidebar-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    width: max-content;
    max-width: 100%;
    height: 44px;
    padding: 0 20px 0 11px;
    gap: 11px;
    border-radius: 9999px;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    line-height: normal;
    text-decoration: none !important;
    overflow: hidden;
    box-sizing: border-box;
    flex-shrink: 0;
    transition: background 200ms ease, color 200ms ease, padding 300ms cubic-bezier(0.16, 1, 0.3, 1), gap 300ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .sidebar-btn:hover {
    color: var(--text-primary);
    background: rgba(var(--surface-tint-rgb), 0.08);
  }

  .sidebar-btn.active {
    color: var(--text-primary);
    background: rgba(var(--surface-tint-rgb), 0.10);
  }

  .sidebar-aside.compact .sidebar-btn {
    padding-right: 11px;
    gap: 0;
  }

  .sidebar-icon {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    opacity: 0.65;
    transition: opacity 200ms ease;
    pointer-events: none;
  }

  .sidebar-icon :global(svg:not(.progress-ring)) {
    width: 22px;
    height: 22px;
    transition: transform var(--duration-normal) var(--ease-expo);
  }

  .sidebar-icon.has-progress :global(svg:not(.progress-ring)) {
    transform: scale(0.74);
  }

  .sidebar-btn:hover .sidebar-icon {
    opacity: 1;
  }

  .sidebar-icon.has-progress {
    opacity: 1;
  }

  .sidebar-icon.active {
    opacity: 1;
  }

  .sidebar-label {
    display: block;
    font-size: 14px;
    font-weight: 475;
    letter-spacing: 0.015em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    flex: 1 1 0%;
    max-width: 100%;
    opacity: 0.65;
    line-height: normal;
    pointer-events: none;
    transition: max-width 300ms cubic-bezier(0.16, 1, 0.3, 1), opacity 200ms ease;
  }

  .sidebar-label.active {
    font-weight: 500;
    opacity: 1;
  }

  .sidebar-btn:hover .sidebar-label {
    opacity: 1;
  }

  .sidebar-aside.compact .sidebar-label,
  .sidebar-aside.compact .profile-label {
    max-width: 0 !important;
    flex: 0 0 0% !important;
    opacity: 0 !important;
    overflow: hidden !important;
  }

  .sidebar-aside.compact .logo-label {
    width: 0;
    max-width: 0;
    opacity: 0;
  }

  .sidebar-badge {
    position: absolute;
    top: -5px;
    right: -5px;
    min-width: 16px;
    height: 16px;
    padding: 0 3px;
    font-size: 8px;
    font-weight: 700;
    border-radius: 9999px;
    background: var(--text-primary);
    color: var(--surface-container);
    display: flex;
    align-items: center;
    justify-content: center;
    animation: pulse 2s infinite;
    pointer-events: none;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }

  .sidebar-aside.mac-style .sidebar-content {
    padding-top: 0;
  }

  .sidebar-traffic-lights {
    position: relative;
    height: 38px;
    padding: 0 0 0 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    z-index: 50;
    flex-shrink: 0;
  }

  .sidebar-aside.compact .sidebar-traffic-lights {
    padding-left: 8px;
    gap: 6px;
  }

  .mac-light {
    position: relative;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1px solid transparent;
    padding: 0;
    margin: 0;
    display: grid;
    place-items: center;
    cursor: pointer;
    box-sizing: border-box;
    transition: filter 120ms ease;
  }

  .mac-light:active {
    filter: brightness(0.82);
  }

  .mac-close {
    background-color: var(--color-mac-close, #ff5f57);
    border-color: var(--color-mac-close-border, rgba(224, 68, 62, 0.6));
  }

  .mac-close:active {
    background-color: var(--color-mac-close-active, #bf4942);
  }

  .mac-minimize {
    background-color: var(--color-mac-minimize, #febc2e);
    border-color: var(--color-mac-minimize-border, rgba(216, 158, 36, 0.6));
  }

  .mac-minimize:active {
    background-color: var(--color-mac-minimize-active, #be8e25);
  }

  .mac-maximize {
    background-color: var(--color-mac-maximize, #28c840);
    border-color: var(--color-mac-maximize-border, rgba(26, 171, 41, 0.6));
  }

  .mac-maximize:active {
    background-color: var(--color-mac-maximize-active, #1f9a31);
  }

  .mac-glyph {
    width: 6px;
    height: 6px;
    color: var(--color-mac-icon, rgba(0, 0, 0, 0.72));
    opacity: 0;
    transition: opacity 120ms ease;
    pointer-events: none;
  }

  .sidebar-traffic-lights:hover .mac-glyph {
    opacity: 1;
  }
</style>
