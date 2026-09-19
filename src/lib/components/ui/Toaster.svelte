<script lang="ts">
  import { Toaster as SonnerToaster } from 'svelte-sonner';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { themeState } from '$lib/theme/themeState.svelte';
  import type { ToasterProps } from 'svelte-sonner';

  type Position = NonNullable<ToasterProps['position']>;

  const position = $derived.by((): Position => {
    const configured = configState.settings.toast_position || 'auto';
    if (configured === 'auto') {
      return layoutState.isMobile ? 'top-center' : 'bottom-right';
    }
    return configured as Position;
  });

  const theme = $derived(themeState.isDark ? 'dark' : 'light');
</script>

<SonnerToaster {position} {theme} />
