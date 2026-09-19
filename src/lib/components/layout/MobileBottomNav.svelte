<script lang="ts">
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { i18n } from '$lib/i18n';
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
  import StableWeightLabel from '$lib/components/ui/StableWeightLabel.svelte';

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

  function hasProgressRing(id: string): boolean {
    return id === 'downloads' && downloadState.activeDownloadsCount > 0;
  }

</script>

<nav
  class="mobile-bottom-dock"
  class:hidden-dock={selectionState.active}
  aria-hidden={selectionState.active}
>
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
        <div class="dock-icon-wrapper" class:has-progress={hasProgressRing(item.id)}>
          {#if hasProgressRing(item.id)}
            <ProgressRing value={downloadState.activeProgress} size={34} />
          {/if}
          {#if isActive}
            <item.iconActive class="dock-icon" />
          {:else}
            <item.icon class="dock-icon" />
          {/if}

          {#if item.badge && item.badge() > 0 && !hasProgressRing(item.id)}
            <span class="dock-badge">{item.badge() > 99 ? '99+' : item.badge()}</span>
          {/if}
        </div>
        <span class="dock-label">
          <StableWeightLabel text={title} reserveWeight="var(--font-weight-bold)" />
        </span>
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
    transition: transform var(--duration-normal, 0.24s) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
                opacity var(--duration-fast, 0.18s) ease;
  }

  .mobile-bottom-dock.hidden-dock {
    transform: translate(-50%, calc(100% + 32px)) scale(0.95);
    opacity: 0;
    pointer-events: none;
  }

  .mobile-dock-capsule {
    display: flex;
    align-items: center;
    justify-content: space-around;
    height: 64px;
    padding: 0 8px;
    background: var(--floating-bg);
    border: var(--floating-border);
    border-radius: 22px;
    box-shadow: var(--floating-shadow);
    backdrop-filter: var(--floating-backdrop) saturate(1.6);
    -webkit-backdrop-filter: var(--floating-backdrop) saturate(1.6);
  }

  .mobile-dock-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    height: 100%;
    background: transparent;
    color: var(--text-muted);
    border: none;
    cursor: pointer;
    padding: 2px 0;
    outline: none;
    transition: color var(--duration-fast, 0.18s) ease;
  }

  .mobile-dock-btn:hover {
    color: var(--text-secondary);
  }

  .mobile-dock-btn.active {
    color: var(--text-primary);
  }

  .dock-icon-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: clamp(48px, 88%, 58px);
    height: 32px;
    border-radius: 9999px;
    background: transparent;
    color: var(--text-muted);
    transition: background-color var(--duration-fast, 0.18s) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
                color var(--duration-fast, 0.18s) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1)),
                transform var(--duration-fast, 0.18s) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1));
  }

  .mobile-dock-btn:hover:not(.active) .dock-icon-wrapper {
    background: rgba(var(--surface-tint-rgb), 0.08);
    color: var(--text-secondary);
  }

  .mobile-dock-btn:active .dock-icon-wrapper {
    transform: scale(0.96);
  }

  .mobile-dock-btn.active .dock-icon-wrapper {
    background: var(--accent-container);
    color: var(--accent-on-container);
  }

  :global(.dock-icon) {
    width: 22px !important;
    height: 22px !important;
    flex-shrink: 0;
    color: inherit;
    transition: transform var(--duration-normal, 0.22s) var(--ease-expo, cubic-bezier(0.16, 1, 0.3, 1));
  }

  .dock-label {
    font-size: 11px;
    font-weight: 500;
    line-height: 1;
    margin-top: 3px;
    white-space: nowrap;
    letter-spacing: -0.01em;
    color: var(--text-muted);
    transition: color var(--duration-fast, 0.18s) ease;
  }

  .mobile-dock-btn.active .dock-label {
    font-weight: 700;
    color: var(--text-primary);
  }

  .dock-icon-wrapper.has-progress :global(.dock-icon) {
    transform: scale(0.74);
  }

  .dock-badge {
    position: absolute;
    top: -2px;
    right: 3px;
    background: var(--status-error);
    color: var(--text-primary);
    font-size: 9.5px;
    font-weight: 700;
    line-height: 1;
    padding: 2px 4px;
    border-radius: 9999px;
    border: 1.5px solid var(--floating-bg);
  }

  .mobile-dock-btn.active :global(.dock-icon-wrapper .progress-ring .ring-track) {
    stroke: var(--accent-on-container);
    opacity: 0.25;
  }

  .mobile-dock-btn.active :global(.dock-icon-wrapper .progress-ring .ring-value) {
    stroke: var(--accent-on-container);
  }
</style>
