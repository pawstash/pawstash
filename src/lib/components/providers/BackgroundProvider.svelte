<script lang="ts">
  import { onMount } from 'svelte';
  import { backgroundState, isWindowsPlatform } from '$lib/theme/backgroundState.svelte';
  import { themeState } from '$lib/theme/themeState.svelte';
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
    {:else if settings.customKind === 'palette'}
      {@const p = themeState.palette}
      <div
        class="custom-background-media"
        style="background: radial-gradient(circle at 18% 22%, {p.quadrants[0]}44 0%, transparent 48%), radial-gradient(circle at 82% 16%, {p.quadrants[1]}38 0%, transparent 44%), radial-gradient(circle at 75% 78%, {p.quadrants[3]}34 0%, transparent 52%), radial-gradient(circle at 22% 82%, {p.quadrants[2]}2e 0%, transparent 46%), linear-gradient(135deg, #07090e 0%, #0f121a 100%); {filterStyle}"
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
