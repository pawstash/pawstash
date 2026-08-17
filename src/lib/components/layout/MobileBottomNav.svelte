<script lang="ts">
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { i18n } from '$lib/i18n';
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

  interface NavItem {
    id: 'feed' | 'creators' | 'favorites' | 'library' | 'downloads' | 'settings';
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

  let activeRoot = $derived(navigationState.activeRoot);
</script>

<nav class="mobile-bottom-dock" aria-label="Mobile Navigation">
  <div class="mobile-dock-capsule">
    {#each navItems as item}
      {@const isActive = activeRoot === item.id}
      {@const title = i18n.t(item.labelKey)}
      <button
        onclick={() => navigationState.navigateRoot(item.id)}
        class="mobile-dock-btn"
        class:active={isActive}
        aria-label={title}
      >
        <div class="dock-icon-wrapper">
          {#if isActive}
            <item.iconActive class="dock-icon" />
          {:else}
            <item.icon class="dock-icon" />
          {/if}

          {#if item.badge && item.badge() > 0}
            <span class="dock-badge">{item.badge() > 99 ? '99+' : item.badge()}</span>
          {/if}
        </div>
        <span class="dock-label">{title}</span>
      </button>
    {/each}
  </div>
</nav>

<style>
  .mobile-bottom-dock {
    position: fixed;
    bottom: max(14px, env(safe-area-inset-bottom, 14px));
    left: 50%;
    transform: translateX(-50%);
    z-index: 999;
    width: min(calc(100vw - 28px), 440px);
    pointer-events: auto;
  }

  .mobile-dock-capsule {
    display: flex;
    align-items: center;
    justify-content: space-around;
    height: 64px;
    padding: 0 8px;
    background: rgba(16, 17, 22, 0.88);
    border: none !important;
    border-radius: 22px;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(24px) saturate(1.6);
    -webkit-backdrop-filter: blur(24px) saturate(1.6);
  }

  .mobile-dock-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    height: 100%;
    background: transparent;
    color: rgba(255, 255, 255, 0.45);
    border: none;
    cursor: pointer;
    padding: 4px 0;
    transition: color 0.18s ease;
  }

  .mobile-dock-btn:hover {
    color: rgba(255, 255, 255, 0.8);
  }

  .mobile-dock-btn.active {
    color: #ffffff;
  }

  .dock-icon-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  :global(.dock-icon) {
    width: 24px !important;
    height: 24px !important;
    flex-shrink: 0;
  }

  .dock-label {
    font-size: 11px;
    font-weight: 500;
    line-height: 1;
    margin-top: 4px;
    white-space: nowrap;
    letter-spacing: -0.01em;
    color: inherit;
  }

  .mobile-dock-btn.active .dock-label {
    font-weight: 700;
    color: #ffffff;
  }

  .dock-badge {
    position: absolute;
    top: -5px;
    right: -8px;
    background: #ef4444;
    color: #ffffff;
    font-size: 9.5px;
    font-weight: 700;
    line-height: 1;
    padding: 2px 4px;
    border-radius: 9999px;
    border: 1.5px solid rgba(16, 17, 22, 0.95);
  }
</style>
