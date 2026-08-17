<script lang="ts">
  import { onMount } from 'svelte';
  import { emit } from '@tauri-apps/api/event';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { i18n } from '$lib/i18n';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { syncState } from '$lib/state/syncState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { apiGetSettings } from '$lib/utils/ipc';
  import BackgroundProvider from '$lib/components/providers/BackgroundProvider.svelte';
  import DesktopTitlebar from '$lib/components/layout/DesktopTitlebar.svelte';
  import SidebarNav from '$lib/components/layout/SidebarNav.svelte';
  import MobileBottomNav from '$lib/components/layout/MobileBottomNav.svelte';
  import SettingsModal from '$lib/components/settings/SettingsModal.svelte';
  import FeedView from '$lib/components/pawchive/FeedView.svelte';
  import PostPage from '$lib/components/pawchive/PostPage.svelte';
  import CreatorPage from '$lib/components/pawchive/CreatorPage.svelte';
  import LibraryView from '$lib/components/library/LibraryView.svelte';
  import DownloadQueueList from '$lib/components/downloads/DownloadQueueList.svelte';
  import CreatorsView from '$lib/components/pawchive/CreatorsView.svelte';
  import FavoritesView from '$lib/components/pawchive/FavoritesView.svelte';
  import ProfilePage from '$lib/components/profile/ProfilePage.svelte';
  import UpdateModal from '$lib/components/ui/UpdateModal.svelte';
  import { updateState } from '$lib/state/updateState.svelte';
  import { Toaster } from 'svelte-sonner';

  navigationState.init();

  let effectiveToastPosition = $derived.by(() => {
    const pos = configState.settings.toast_position;
    if (!pos || pos === 'auto') {
      return layoutState.isMobile ? 'top-center' : 'bottom-right';
    }
    return pos as any;
  });

  onMount(() => {
    themeState.init();
    i18n.init();

    const preventPinchZoom = (e: TouchEvent) => {
      if (e.touches.length > 1) {
        e.preventDefault();
      }
    };
    document.addEventListener('touchstart', preventPinchZoom, { passive: false });
    document.addEventListener('touchmove', preventPinchZoom, { passive: false });

    void apiGetSettings()
      .then((settings) => {
        configState.updateSettings(settings);
        if (settings.auto_check_updates ?? true) {
          setTimeout(() => void updateState.check(true), 3000);
        }
      })
      .catch((error) => console.warn('Failed to load application settings', error));
    void libraryState.init().catch((error) => console.warn('Failed to initialize library', error));
    void downloadState.init().catch((error) => console.warn('Failed to initialize downloads', error));
    void subscriptionState.init().catch((error) => console.warn('Failed to initialize subscriptions', error));
    void accountState.refresh().catch((error) => console.warn('Failed to check Pawchive session', error));
    void syncState.init().catch((error) => console.warn('Failed to initialize encrypted sync', error));
    void creatorsState.load().catch((error) => console.warn('Failed to preload creators list', error));

    requestAnimationFrame(() => void emit('frontend-ready'));
  });
</script>

<div
  class="app-shell h-screen w-screen flex overflow-hidden select-none text-[var(--text-primary)] font-sans relative"
  class:mobile-layout={layoutState.isMobile}
  data-layout={layoutState.isMobile ? 'mobile' : 'desktop'}
>
  <BackgroundProvider />
  <Toaster theme="dark" position={effectiveToastPosition} richColors />

  {#if !layoutState.isMobile}
    <SidebarNav />
  {/if}

  <div class="flex-1 flex overflow-hidden relative">
    {#if !layoutState.isMobile}
      <div class="absolute top-0 left-0 w-full z-50 pointer-events-none">
        <div class="pointer-events-auto">
          <DesktopTitlebar />
        </div>
      </div>
    {/if}

    <main class="flex-1 flex overflow-hidden w-full h-full">
      {#key navigationState.entryKey}
        {#if navigationState.route.name === 'feed'}
          <FeedView />
        {:else if navigationState.route.name === 'favorites'}
          <FavoritesView />
        {:else if navigationState.route.name === 'library'}
          <LibraryView />
        {:else if navigationState.route.name === 'downloads'}
          <DownloadQueueList />
        {:else if navigationState.route.name === 'creators'}
          <CreatorsView />
        {:else if navigationState.route.name === 'profile'}
          <ProfilePage />
        {:else if navigationState.route.name === 'post'}
          <PostPage
            service={navigationState.route.service}
            creatorId={navigationState.route.creatorId}
            postId={navigationState.route.postId}
          />
        {:else if navigationState.route.name === 'creator'}
          <CreatorPage service={navigationState.route.service} creatorId={navigationState.route.creatorId} />
        {:else if navigationState.route.name === 'settings'}
          <SettingsModal />
        {:else}
          <div class="flex-1 flex items-center justify-center text-gray-500 text-xs">
            {i18n.t('nav.coming_soon')}
          </div>
        {/if}
      {/key}
    </main>
  </div>

  {#if layoutState.isMobile}
    <MobileBottomNav />
  {/if}

  <UpdateModal />
</div>

<style>
  .mobile-layout :global(.md\:grid-cols-2) {
    grid-template-columns: minmax(0, 1fr) !important;
  }
</style>
