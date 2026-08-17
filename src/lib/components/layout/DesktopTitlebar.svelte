<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { tooltip } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
  import IconArrowRight from '~icons/fluent/arrow-right-24-regular';
  import IconMinimize from '~icons/fluent/subtract-24-regular';
  import IconMaximize from '~icons/fluent/square-24-regular';
  import IconRestore from '~icons/fluent/square-multiple-24-regular';
  import IconClose from '~icons/fluent/dismiss-24-regular';

  const appWindow = getCurrentWindow();
  let isMaximized = $state(false);
  let isOffline = $state(false);
  let isMacStyle = $derived(layoutState.effectiveTitlebarStyle === 'macos');

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
    const updateConnectivity = () => {
      isOffline = !navigator.onLine;
    };
    updateConnectivity();
    window.addEventListener('online', updateConnectivity);
    window.addEventListener('offline', updateConnectivity);

    void appWindow.isMaximized().then((val) => {
      isMaximized = val;
    });

    const unlistenPromise = appWindow.onResized(() => {
      void appWindow.isMaximized().then((val) => {
        isMaximized = val;
      });
    });

    return () => {
      window.removeEventListener('online', updateConnectivity);
      window.removeEventListener('offline', updateConnectivity);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  });
</script>

<div
  data-tauri-drag-region
  class="h-[34px] w-full flex items-center justify-between pr-0 select-none bg-transparent text-xs text-gray-400 z-50 shrink-0 relative cursor-default"
  class:pl-2={!isMacStyle}
  class:pl-6={isMacStyle}
>
  <div class="flex items-center space-x-1 z-10" data-tauri-drag-region="false">
    <button
      data-tauri-drag-region="false"
      use:tooltip={i18n.t('nav.back')}
      onclick={() => navigationState.back()}
      disabled={!navigationState.canGoBack}
      class="left-button"
      aria-label={i18n.t('nav.back')}
    >
      <IconArrowLeft class="w-[18px] h-[18px]" />
    </button>
    <button
      data-tauri-drag-region="false"
      use:tooltip={i18n.t('nav.forward')}
      onclick={() => navigationState.forward()}
      disabled={!navigationState.canGoForward}
      class="left-button"
      aria-label={i18n.t('nav.forward')}
    >
      <IconArrowRight class="w-[18px] h-[18px]" />
    </button>
    {#if isOffline}
      <span class="offline-status" role="status" aria-live="polite">
        <span class="offline-dot" aria-hidden="true"></span>
        {i18n.t('nav.offline')}
      </span>
    {/if}
  </div>

  {#if !isMacStyle}
    <div class="flex items-center z-10 ml-auto h-full" data-tauri-drag-region="false">
      <button
        data-tauri-drag-region="false"
        use:tooltip={i18n.t('actions.minimize')}
        onclick={minimize}
        class="control-button"
        aria-label={i18n.t('actions.minimize')}
      >
        <IconMinimize class="w-[16px] h-[16px]" />
      </button>
      <button
        data-tauri-drag-region="false"
        use:tooltip={i18n.t(isMaximized ? 'actions.restore' : 'actions.maximize')}
        onclick={toggleMaximize}
        class="control-button"
        aria-label={i18n.t(isMaximized ? 'actions.restore' : 'actions.maximize')}
      >
        {#if isMaximized}
          <IconRestore class="w-[16px] h-[16px]" />
        {:else}
          <IconMaximize class="w-[16px] h-[16px]" />
        {/if}
      </button>
      <button
        data-tauri-drag-region="false"
        use:tooltip={i18n.t('actions.close')}
        onclick={close}
        class="control-button control-close"
        aria-label={i18n.t('actions.close')}
      >
        <IconClose class="w-[16px] h-[16px]" />
      </button>
    </div>
  {/if}
</div>

<style>
  .left-button {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: rgb(209 213 219);
    cursor: pointer;
    transition: color 150ms ease, background-color 150ms ease;
  }

  .left-button:hover:not(:disabled) {
    color: white;
    background: rgba(255, 255, 255, 0.1);
  }

  .left-button:disabled {
    color: rgba(156, 163, 175, 0.28);
    cursor: default;
  }

  .offline-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    margin-left: 8px;
    color: rgba(156, 163, 175, 0.68);
    font-family: var(--font-sans);
    font-size: 11px;
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
  }

  .offline-dot {
    width: 5px;
    height: 5px;
    flex: 0 0 5px;
    border-radius: 50%;
    background: currentColor;
  }

  .control-button {
    width: 46px;
    height: 100%;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: rgb(209 213 219);
    cursor: pointer;
    transition: color 150ms ease, background-color 150ms ease;
  }

  .control-button:hover:not(:disabled) {
    color: white;
    background: rgba(255, 255, 255, 0.1);
  }

  .control-close:hover:not(:disabled) {
    background: rgb(239 68 68);
  }

  .control-button:disabled {
    color: rgba(156, 163, 175, 0.28);
    cursor: default;
  }
</style>
