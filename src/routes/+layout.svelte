<script lang="ts">
  import type { Snippet } from 'svelte';
  import { initPanicListener } from '$lib/utils/panic';

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  initPanicListener();
</script>

{#if children}
  {@render children()}
{/if}
