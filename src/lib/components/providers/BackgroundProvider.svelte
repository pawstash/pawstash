<script lang="ts">
  import { onMount } from 'svelte';
  import { backgroundState, isWindowsPlatform } from '$lib/theme/backgroundState.svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';

  onMount(() => {
    backgroundState.init();
  });

  let settings = $derived(backgroundState.settings);

  function toAssetUrl(path: string): string {
    if (!path) return '';
    if (/^(https?:|asset:|blob:|data:)/i.test(path)) return path;
    try {
      return convertFileSrc(path);
    } catch (e) {
      return path;
    }
  }

  let filterStyle = $derived(
    `filter: blur(${settings.blurPx}px) brightness(${settings.brightness}) saturate(${settings.saturation}); opacity: ${settings.opacity};`
  );

  let isNativeEffect = $derived.by(() => {
    if (isWindowsPlatform() && ['acrylic', 'mica-dark', 'tabbed'].includes(settings.type)) return true;
    if (typeof navigator !== 'undefined' && navigator.userAgent.toLowerCase().includes('mac') && settings.type === 'vibrancy') return true;
    return false;
  });
</script>

<div class="fixed inset-0 z-[-2] overflow-hidden pointer-events-none select-none">
  {#if settings.type === 'oled' || (!isNativeEffect && settings.type !== 'custom')}
    <div class="absolute inset-0 bg-black"></div>

  {:else if settings.type === 'custom'}
    {#if settings.customKind === 'video' && settings.videoUrl}
      {@const videoSrc = toAssetUrl(settings.videoUrl)}
      <video
        src={videoSrc}
        autoplay
        loop
        muted
        playsinline
        class="custom-background-media"
        style={filterStyle}
      ></video>
    {:else if settings.customKind === 'image' && settings.imageUrl}
      {@const imgSrc = toAssetUrl(settings.imageUrl)}
      <div
        class="custom-background-media bg-cover bg-center"
        style="background-image: url('{imgSrc}'); {filterStyle}"
      ></div>
    {:else}
      <div
        class="custom-background-media"
        style="background: linear-gradient(135deg, {settings.solidColor}, {settings.gradientSecondary}); {filterStyle}"
      ></div>
    {/if}
  {/if}
</div>

<style>
  .custom-background-media {
    position: absolute;
    inset: -5%;
    width: 110%;
    height: 110%;
    object-fit: cover;
    transition: filter var(--duration-normal), opacity var(--duration-normal);
  }
</style>
