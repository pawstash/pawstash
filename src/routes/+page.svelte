<script module lang="ts">
  import type { AppRoute } from '$lib/state/navigationState.svelte';

  function lazyView<T>(load: () => Promise<{ default: T }>): () => Promise<{ default: T }> {
    let pending: Promise<{ default: T }> | undefined;
    return () => (pending ??= load());
  }

  const views = {
    feed: lazyView(() => import('$lib/components/content/FeedView.svelte')),
    favorites: lazyView(() => import('$lib/components/content/FavoritesView.svelte')),
    library: lazyView(() => import('$lib/components/library/LibraryView.svelte')),
    creators: lazyView(() => import('$lib/components/content/CreatorsView.svelte')),
    downloads: lazyView(() => import('$lib/components/downloads/DownloadQueueList.svelte')),
    profile: lazyView(() => import('$lib/components/profile/ProfilePage.svelte')),
    post: lazyView(() => import('$lib/components/content/PostPage.svelte')),
    creator: lazyView(() => import('$lib/components/content/CreatorPage.svelte')),
    settings: lazyView(() => import('$lib/components/settings/SettingsPage.svelte'))
  };

  function preloadView(name: AppRoute['name']) {
    void views[name]();
  }

  function preloadRemainingViews() {
    for (const name of Object.keys(views) as AppRoute['name'][]) void views[name]();
  }

  function scheduleViewPreload() {
    if ('requestIdleCallback' in window) {
      requestIdleCallback(() => preloadRemainingViews(), { timeout: 4000 });
    } else {
      setTimeout(preloadRemainingViews, 1500);
    }
  }
</script>

<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { emit, listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { onBackButtonPress } from '@tauri-apps/api/app';
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
  import { providerState } from '$lib/state/providerState.svelte';
  import { apiGetSettings, apiGetPendingDeepLink, apiSetAppHidden, apiShowMainWindow } from '$lib/utils/ipc';
  import BackgroundProvider from '$lib/components/providers/BackgroundProvider.svelte';
  import SidebarNav from '$lib/components/layout/SidebarNav.svelte';
  import MobileBottomNav from '$lib/components/layout/MobileBottomNav.svelte';
  import SettingsNav from '$lib/components/settings/SettingsNav.svelte';
  import UpdateModal from '$lib/components/ui/UpdateModal.svelte';
  import NoticeModal from '$lib/components/ui/NoticeModal.svelte';
  import { noticeState } from '$lib/state/noticeState.svelte';
  import { innerFormatsState } from '$lib/state/innerFormatsState.svelte';
  import { uiScaleState } from '$lib/state/uiScaleState.svelte';
  import { scaleIndicator } from '$lib/state/scaleState.svelte';
  import IconGrid from '~icons/fluent/grid-24-regular';
  import IconWindow from '~icons/fluent/window-24-regular';
  import SubscriptionModal from '$lib/components/content/SubscriptionModal.svelte';
  import UnsubscribeDialog from '$lib/components/content/UnsubscribeDialog.svelte';
  import { updateState } from '$lib/state/updateState.svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { onboardingState } from '$lib/state/onboardingState.svelte';
  import { initFrontendLogging, logger } from '$lib/utils/logger';
  import { initDeepLinkListener, handleDeepLinkUrl } from '$lib/utils/deepLink';
  import { initRestartNotice } from '$lib/utils/restartNotice';
  import Toaster from '$lib/components/ui/Toaster.svelte';
  import ContextMenu from '$lib/components/ui/ContextMenu.svelte';

  navigationState.init();
  preloadView(navigationState.route.name);

  onMount(() => {
    initFrontendLogging();
    themeState.init();
    i18n.init();
    document.documentElement.classList.toggle('webkitgtk', layoutState.isLinux);

    const preventPinchZoom = (e: TouchEvent) => {
      if (e.touches.length > 1) {
        e.preventDefault();
      }
    };
    document.addEventListener('touchstart', preventPinchZoom, { passive: false });
    document.addEventListener('touchmove', preventPinchZoom, { passive: false });

    function handleDeepLinkPayload(payloadStr: string | null) {
      if (!payloadStr) return;
      if (
        payloadStr.startsWith('pawstash://') ||
        payloadStr.startsWith('http://') ||
        payloadStr.startsWith('https://')
      ) {
        void handleDeepLinkUrl(payloadStr);
        return;
      }
      try {
        const payload = JSON.parse(payloadStr);
        if (payload.action === 'open_post' && payload.service && payload.creatorId && payload.postId) {
          navigationState.openPost(payload.service, payload.creatorId, payload.postId, undefined, true);
        } else if (payload.action === 'open_downloads') {
          navigationState.navigateRoot('downloads');
        }
      } catch (e) {
        logger.warn('Failed to parse deep link payload', e);
      }
    }

    const cleanupDeepLink = initDeepLinkListener();
    const cleanupRestartNotice = initRestartNotice();
    const reportVisibility = () =>
      void apiSetAppHidden(document.hidden).catch((e) => logger.debug('Failed to report window visibility', e));
    document.addEventListener('visibilitychange', reportVisibility);
    void apiGetPendingDeepLink()
      .then(handleDeepLinkPayload)
      .catch((e) => logger.warn('Failed to read pending deep link', e));
    const unlistenDeepLink = listen<string>('open-post-deep-link', (event) => {
      handleDeepLinkPayload(event.payload);
    });
    const unlistenPanic = listen('panic-mode', () => {
      document.querySelectorAll('video, audio').forEach((el) => {
        try {
          (el as HTMLMediaElement).pause();
        } catch {}
      });
    });

    void apiGetSettings()
      .then((settings) => {
        configState.hydrate(settings);
        // Read once: transparency is fixed when the window is created.
        document.documentElement.classList.toggle(
          'opaque-window',
          layoutState.isLinux && settings.linux_transparent_window !== true
        );
        if (settings.auto_check_updates ?? true) {
          setTimeout(async () => {
            await noticeState.check();
            await noticeState.whenClosed();
            void updateState.check(true);
          }, 3000);
        }
      })
      .catch((error) => logger.warn('Failed to load application settings', error));
    void libraryState.init().catch((error) => logger.warn('Failed to initialize library', error));
    void downloadState.init().catch((error) => logger.warn('Failed to initialize downloads', error));
    void subscriptionState.init().catch((error) => logger.warn('Failed to initialize subscriptions', error));
    void accountState.refresh().catch((error) => logger.warn('Failed to check Pawchive session', error));
    void syncState.init().catch((error) => logger.warn('Failed to initialize encrypted sync', error));
    void innerFormatsState.init().catch((error) => logger.warn('Failed to load indexed post contents', error));
    void uiScaleState.init().then(() => {
      const legacyScale = themeState.takeLegacyUiScale();
      if (legacyScale && !uiScaleState.percent) uiScaleState.set(legacyScale);
    });
    void providerState.loadProviders().catch((error) => logger.warn('Failed to load providers', error));
    void creatorsState.init().catch((error) => logger.warn('Failed to initialize creators list', error));

    requestAnimationFrame(() => {
      void emit('frontend-ready');
      void apiShowMainWindow().catch((e) => logger.warn('Failed to show main window', e));
      scheduleViewPreload();
    });

    return () => {
      document.removeEventListener('touchstart', preventPinchZoom);
      document.removeEventListener('touchmove', preventPinchZoom);
      cleanupDeepLink();
      cleanupRestartNotice();
      document.removeEventListener('visibilitychange', reportVisibility);
      void unlistenDeepLink.then((u) => u());
      void unlistenPanic.then((u) => u());
    };
  });

  $effect(() => {
    document.documentElement.dataset.layout = layoutState.isMobile ? 'mobile' : 'desktop';
    document.documentElement.dataset.platform = layoutState.isMobileDevice ? 'touch' : 'desktop';
  });

  onMount(() => {
    if (!layoutState.isAndroid) return;
    let listener: { unregister: () => Promise<void> } | undefined;
    void onBackButtonPress(() => {
      if (!navigationState.handleSystemBack()) void invoke('android_minimize');
    }).then((registered) => (listener = registered));
    return () => void listener?.unregister();
  });

  $effect(() => {
    if (!libraryState.initialized || !syncState.statusLoaded) return;
    onboardingState.observeLibrary(libraryState.savedCount, libraryState.stashCount, syncState.status.configured);
  });

  $effect(() => {
    if (!selectionState.active) return;
    return untrack(() =>
      navigationState.registerBackHandler(() => {
        selectionState.exit();
        return true;
      })
    );
  });
</script>

<div
  class="app-shell h-screen w-screen flex overflow-hidden select-none text-[var(--text-primary)] font-sans relative"
>
  <BackgroundProvider />
  <Toaster />
  <ContextMenu />

  {#if !layoutState.isMobile}
    {#if navigationState.route.name === 'settings'}
      <SettingsNav />
    {:else}
      <SidebarNav />
    {/if}
  {/if}

  <div class="app-content flex-1 flex overflow-hidden relative min-w-0">
    <main class="flex-1 flex overflow-hidden w-full h-full">
      {#key navigationState.entryKey}
        {#if navigationState.route.name === 'feed'}
          {#await views.feed() then viewModule}
            {@const FeedView = viewModule.default}
            <FeedView />
          {/await}
        {:else if navigationState.route.name === 'favorites'}
          {#await views.favorites() then viewModule}
            {@const FavoritesView = viewModule.default}
            <FavoritesView />
          {/await}
        {:else if navigationState.route.name === 'library'}
          {#await views.library() then viewModule}
            {@const LibraryView = viewModule.default}
            <LibraryView />
          {/await}
        {:else if navigationState.route.name === 'downloads'}
          {#await views.downloads() then viewModule}
            {@const DownloadQueueList = viewModule.default}
            <DownloadQueueList />
          {/await}
        {:else if navigationState.route.name === 'creators'}
          {#await views.creators() then viewModule}
            {@const CreatorsView = viewModule.default}
            <CreatorsView />
          {/await}
        {:else if navigationState.route.name === 'profile'}
          {#await views.profile() then viewModule}
            {@const ProfilePage = viewModule.default}
            <ProfilePage />
          {/await}
        {:else if navigationState.route.name === 'post'}
          {@const route = navigationState.route}
          {#await views.post() then viewModule}
            {@const PostPage = viewModule.default}
            <PostPage
              service={route.service}
              creatorId={route.creatorId}
              postId={route.postId}
              initialMedia={route.initialMedia}
              openViewer={route.openViewer}
            />
          {/await}
        {:else if navigationState.route.name === 'creator'}
          {@const route = navigationState.route}
          {#await views.creator() then viewModule}
            {@const CreatorPage = viewModule.default}
            <CreatorPage
              service={route.service}
              creatorId={route.creatorId}
              initialTag={route.initialTag}
            />
          {/await}
        {:else if navigationState.route.name === 'settings'}
          {#await views.settings() then viewModule}
            {@const SettingsPage = viewModule.default}
            <SettingsPage />
          {/await}
        {/if}
      {/key}
    </main>
  </div>

  {#if layoutState.isMobile}
    <MobileBottomNav />
  {/if}

  <UpdateModal />
  <NoticeModal />
  {#if scaleIndicator.kind}
    <div class="scale-indicator" role="status">
      {#if scaleIndicator.kind === 'grid'}<IconGrid />{:else}<IconWindow />{/if}
      {i18n.t(scaleIndicator.kind === 'grid' ? 'settings.grid_scale_indicator' : 'settings.ui_scale_indicator', { percent: scaleIndicator.percent })}
    </div>
  {/if}
  <SubscriptionModal />
  <UnsubscribeDialog />
</div>
