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
  import StableWeightLabel from '$lib/components/ui/StableWeightLabel.svelte';

  interface NavItem {
    id: 'feed' | 'creators' | 'favorites' | 'library' | 'downloads';
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
    { id: 'downloads', labelKey: 'nav.downloads', icon: IconDownloads, iconActive: IconDownloadsFilled, badge: () => downloadState.activeDownloadsCount }
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
  inert={selectionState.active}
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
            <ProgressRing value={downloadState.activeProgress} size={30} />
          {/if}
          {#if isActive}
            <item.iconActive class="dock-icon" />
          {:else}
            <item.icon class="dock-icon" />
          {/if}

          {#if item.badge && item.badge() > 0 && !hasProgressRing(item.id)}
            <span class="dock-badge">{item.badge() > 99 ? '99+' : i18n.number(item.badge())}</span>
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
    bottom: max(var(--safe-bottom), var(--dock-margin));
    left: 50%;
    z-index: var(--z-dock);
    width: min(calc(100vw - var(--dock-margin) * 2), var(--dock-max-width));
    transform: translateX(-50%);
    transition: transform var(--duration-normal) var(--ease-standard),
                opacity var(--duration-fast) var(--ease-standard);
  }

  .mobile-bottom-dock.hidden-dock {
    transform: translate(-50%, calc(100% + var(--dock-margin) * 2));
    opacity: 0;
    pointer-events: none;
  }

  .mobile-dock-capsule {
    display: flex;
    align-items: center;
    height: var(--mobile-nav-height);
    padding: 0 4px;
    border-radius: var(--dock-radius);
    background: var(--floating-bg);
    box-shadow: var(--floating-shadow);
  }

  .mobile-dock-btn {
    display: flex;
    flex: 1 1 0;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    outline: none;
  }

  .mobile-dock-btn:focus-visible .dock-icon-wrapper {
    outline: 2px solid var(--accent-primary);
    outline-offset: 2px;
  }

  .dock-icon-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 56px;
    height: 32px;
    border-radius: var(--radius-full);
    background: transparent;
    color: inherit;
    overflow: hidden;
    isolation: isolate;
    transition: background-color var(--duration-fast) var(--ease-standard),
                color var(--duration-normal) var(--ease-expo);
  }

  .dock-icon-wrapper::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    z-index: -1;
    width: 72px;
    height: 72px;
    margin: -36px 0 0 -36px;
    border-radius: 50%;
    background: var(--accent-container);
    opacity: 0;
    transform: scale(0);
    will-change: transform;
    pointer-events: none;
    transition:
      transform var(--duration-normal) var(--ease-expo),
      opacity var(--duration-fast) var(--ease-standard);
  }

  @media (prefers-reduced-motion: reduce) {
    .dock-icon-wrapper::before,
    .mobile-dock-btn.active .dock-icon-wrapper::before {
      transition: opacity var(--duration-fast) var(--ease-standard);
    }
  }

  @media (hover: hover) {
    .mobile-dock-btn:hover:not(.active) .dock-icon-wrapper {
      background: var(--state-layer-hover);
    }
  }

  .mobile-dock-btn:active:not(.active) .dock-icon-wrapper {
    background: var(--state-layer-pressed);
  }

  .mobile-dock-btn.active .dock-icon-wrapper::before {
    transform: scale(1);
    opacity: 1;
    transition:
      transform var(--duration-slow) var(--ease-expo),
      opacity var(--duration-fast) var(--ease-standard);
  }

  .mobile-dock-btn.active .dock-icon-wrapper {
    color: var(--accent-on-container);
  }

  :global(.dock-icon) {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    color: inherit;
    transition: transform var(--duration-normal) var(--ease-standard);
  }

  .dock-icon-wrapper.has-progress :global(.dock-icon) {
    transform: scale(0.75);
  }

  .dock-label {
    max-width: 100%;
    overflow: hidden;
    color: var(--text-secondary);
    font-size: var(--type-label-md-size);
    line-height: var(--type-label-md-line);
    font-weight: 500;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .mobile-dock-btn.active .dock-label {
    color: var(--text-primary);
    font-weight: 700;
  }

  .dock-badge {
    position: absolute;
    top: 0;
    left: calc(50% + 4px);
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: var(--radius-full);
    background: var(--status-error);
    color: var(--on-status);
    font-size: var(--type-label-sm-size);
    font-weight: 600;
    line-height: 16px;
    text-align: center;
    font-variant-numeric: tabular-nums;
    box-shadow: 0 0 0 2px var(--floating-bg);
    box-sizing: border-box;
  }

  .mobile-dock-btn.active :global(.dock-icon-wrapper .progress-ring .ring-track) {
    stroke: var(--accent-on-container);
    opacity: 0.25;
  }

  .mobile-dock-btn.active :global(.dock-icon-wrapper .progress-ring .ring-value) {
    stroke: var(--accent-on-container);
  }
</style>
