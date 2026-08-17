<script lang="ts">
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { i18n } from '$lib/i18n';
  import { motion, tooltip } from '$lib/motion';
  import { syncState } from '$lib/state/syncState.svelte';
  import { openExternalUrl } from '$lib/utils/ipc';
  import IconFeed from '~icons/fluent/grid-24-regular';
  import IconFeedFilled from '~icons/fluent/grid-24-filled';
  import IconCreators from '~icons/fluent/people-24-regular';
  import IconCreatorsFilled from '~icons/fluent/people-24-filled';
  import IconFavorites from '~icons/fluent/heart-24-regular';
  import IconFavoritesFilled from '~icons/fluent/heart-24-filled';
  import IconLibrary from '~icons/fluent/library-24-regular';
  import IconLibraryFilled from '~icons/fluent/library-24-filled';
  import IconDownloads from '~icons/fluent/arrow-download-24-regular';
  import IconDownloadsFilled from '~icons/fluent/arrow-download-24-filled';
  import IconSettings from '~icons/fluent/settings-24-regular';
  import IconSettingsFilled from '~icons/fluent/settings-24-filled';
  import IconChevronLeft from '~icons/fluent/chevron-left-24-regular';
  import IconChevronRight from '~icons/fluent/chevron-right-24-regular';
  import IconTelegram from '~icons/simple-icons/telegram';
  import IconDiscord from '~icons/simple-icons/discord';
  import IconReddit from '~icons/simple-icons/reddit';
  import IconGithub from '~icons/simple-icons/github';
  import IconUser from '~icons/fluent/person-24-regular';
  import IconCloudSync from '~icons/fluent/cloud-sync-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

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

  let isCompact = $state(false);
  let activeRoot = $derived(navigationState.activeRoot);

  let profileName = $derived(
    syncState.status.account_id || i18n.t('profile.local')
  );

  let profileSub = $derived.by(() => {
    if (syncState.status.configured) {
      if (syncState.busy) return i18n.t('sync.status_syncing');
      if (!syncState.status.unlocked) return i18n.t('sync.locked');
      return i18n.t('sync.title');
    }
    return i18n.t('profile.offline_session');
  });

  let syncDotStatus = $derived.by(() => {
    if (syncState.status.configured) {
      if (!syncState.status.unlocked) return 'locked';
      if (syncState.busy) return 'syncing';
      return 'active';
    }
    return 'offline';
  });
</script>

<aside
  data-tauri-drag-region
  class="sidebar-aside"
  class:compact={isCompact}
>
  <button
    data-tauri-drag-region="false"
    use:motion={'sidebar-item'}
    onclick={() => navigationState.navigateRoot('feed')}
    class="sidebar-btn logo-btn"
    aria-label="Pawstash Logo"
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
    <span class="sidebar-label logo-label">Pawstash</span>
  </button>

  <button
    data-tauri-drag-region="false"
    use:motion={'sidebar-item'}
    class="sidebar-btn profile-btn"
    class:active={activeRoot === 'profile'}
    aria-label={profileName}
    onclick={() => navigationState.navigateRoot('profile')}
  >
    <div class="sidebar-icon relative">
      {#if syncDotStatus === 'syncing'}
        <IconLoading class="text-[var(--accent)]" />
      {:else if syncState.status.configured}
        <IconCloudSync />
      {:else}
        <IconUser />
      {/if}

      <span
        class="status-dot"
        class:status-dot-active={syncDotStatus === 'active'}
        class:status-dot-locked={syncDotStatus === 'locked'}
        class:status-dot-syncing={syncDotStatus === 'syncing'}
        class:status-dot-offline={syncDotStatus === 'offline'}
      ></span>
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
        <div class="sidebar-icon" class:active={isActive}>
          {#if isActive}
            <item.iconActive />
          {:else}
            <item.icon />
          {/if}

          {#if item.badge && item.badge() > 0}
            <span class="sidebar-badge">{item.badge()}</span>
          {/if}
        </div>

        <span class="sidebar-label" class:active={isActive}>{title}</span>
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
    >
      <div class="sidebar-icon">
        <IconGithub />
      </div>
      <span class="sidebar-label">GitHub</span>
    </button>

    <button
      data-tauri-drag-region="false"
      use:motion={'sidebar-item'}
      onclick={() => isCompact = !isCompact}
      class="sidebar-btn"
      aria-label={i18n.t('nav.toggle_sidebar')}
    >
      <div class="sidebar-icon">
        {#if isCompact}
          <IconChevronRight />
        {:else}
          <IconChevronLeft />
        {/if}
      </div>
      <span class="sidebar-label">{i18n.t('nav.collapse')}</span>
    </button>
  </div>
</aside>

<style>
  .logo-btn {
    margin-bottom: 8px;
  }

  .logo-svg {
    width: 22px;
    height: 22px;
    opacity: 0.9;
    transition: opacity var(--duration-fast) var(--ease-expo);
  }

  .logo-svg path {
    fill: #ffffff;
    transition: fill var(--duration-normal) var(--ease-expo);
  }

  .logo-btn:hover .logo-svg path {
    fill: url(#logo-grad);
  }

  .logo-btn:hover .logo-svg {
    opacity: 1;
  }

  .logo-label {
    font-weight: 600 !important;
    font-size: 15px !important;
    letter-spacing: 0.03em !important;
    color: #ffffff !important;
    opacity: 0.95 !important;
  }

  .profile-btn {
    height: 48px !important;
    margin-bottom: 4px;
    width: 100% !important;
    min-width: 0;
    padding: 0 12px 0 11px !important;
    gap: 8px !important;
  }

  .profile-label {
    display: flex !important;
    flex-direction: column !important;
    align-items: flex-start !important;
    text-align: left !important;
    gap: 1px;
    line-height: 1.2 !important;
    min-width: 0;
    width: 100%;
  }

  .profile-name {
    font-size: 13.5px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.9);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
  }

  .profile-sub {
    font-size: 10px;
    font-weight: 400;
    color: rgba(255, 255, 255, 0.4);
    letter-spacing: 0.02em;
    text-transform: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
  }

  .profile-divider {
    height: 1px;
    width: 80%;
    margin: 4px auto 8px auto;
    background: rgba(255, 255, 255, 0.08);
  }

  .status-dot {
    position: absolute;
    bottom: -1px;
    right: -1px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    border: 1.5px solid var(--bg-surface, #121214);
    pointer-events: none;
    transition: all var(--duration-fast) var(--ease-out);
  }

  .status-dot-active {
    background-color: #10b981;
    box-shadow: 0 0 6px rgba(16, 185, 129, 0.5);
  }

  .status-dot-locked {
    background-color: #f59e0b;
    box-shadow: 0 0 6px rgba(245, 158, 11, 0.5);
  }

  .status-dot-syncing {
    background-color: #38bdf8;
    box-shadow: 0 0 6px rgba(56, 189, 248, 0.5);
    animation: pulse 1.2s infinite;
  }

  .status-dot-offline {
    background-color: rgba(255, 255, 255, 0.25);
  }

  .sidebar-aside.compact .profile-divider {
    margin-left: 4px;
    margin-right: 4px;
  }

  .sidebar-aside {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 160px;
    padding: 12px 6px 12px 6px;
    background: rgba(255, 255, 255, 0.02);
    border-right: 1px solid rgba(255, 255, 255, 0.04);
    user-select: none;
    flex-shrink: 0;
    border-top: none;
    border-left: none;
    border-bottom: none;
    outline: none;
    cursor: default;
    transition: width 300ms ease-out, padding 300ms ease-out;
  }

  .sidebar-aside.compact {
    width: 56px;
    padding-left: 6px;
    padding-right: 6px;
    padding-top: 12px;
  }

  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }

  .sidebar-bottom {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }

  .sidebar-btn {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 11px;
    width: max-content;
    height: 44px;
    padding: 0 20px 0 11px;
    border-radius: 9999px;
    border: none;
    outline: none;
    background: transparent;
    color: rgba(255, 255, 255, 0.55);
    cursor: pointer;
    text-decoration: none !important;
    transition: all 300ms ease-out;
  }

  .sidebar-btn:hover {
    color: rgba(255, 255, 255, 1);
    background: rgba(255, 255, 255, 0.08);
  }

  .sidebar-btn.active {
    color: #ffffff;
    background: rgba(255, 255, 255, 0.10);
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

  .sidebar-icon :global(svg) {
    width: 22px;
    height: 22px;
  }

  .sidebar-btn:hover .sidebar-icon {
    opacity: 1;
  }

  .sidebar-icon.active {
    opacity: 1;
  }

  .sidebar-label {
    font-size: 14px;
    font-weight: 300;
    letter-spacing: 0.015em;
    white-space: nowrap;
    overflow: hidden;
    opacity: 0.65;
    transition: opacity 300ms ease-out, max-width 300ms ease-out, margin 300ms ease-out;
    max-width: 120px;
    pointer-events: none;
  }

  .sidebar-label.active {
    font-weight: 500;
    opacity: 1;
  }

  .sidebar-btn:hover .sidebar-label {
    opacity: 1;
  }

  .sidebar-aside.compact .sidebar-label {
    max-width: 0;
    margin: 0;
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
    background: white;
    color: black;
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
</style>
