<script lang="ts">
  import { portal } from '$lib/actions/portal';

  interface Props {
    src?: string | null;
    height?: number;
    opacity?: number;
    blur?: number;
  }

  let {
    src = null,
    height = 480,
    opacity = 0.65,
    blur = 12
  }: Props = $props();

  let failed = $state(false);

  $effect(() => {
    if (src) {
      failed = false;
      const img = new Image();
      img.src = src;
      img.onerror = () => {
        failed = true;
      };
    } else {
      failed = true;
    }
  });
</script>

{#if src && !failed}
  <div
    use:portal={'.app-shell'}
    class="hero-backdrop-portal"
    style="
      background-image: url('{src}');
      height: {height}px;
      opacity: {opacity};
      filter: blur({blur}px) brightness(0.45) saturate(1.2);
    "
    role="presentation"
  ></div>
{/if}

<style>
  .hero-backdrop-portal {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    width: 100vw;
    background-size: cover;
    background-position: center;
    z-index: -1;
    pointer-events: none;
    mask-image: radial-gradient(ellipse at 50% 0%, black 15%, transparent 75%);
    -webkit-mask-image: radial-gradient(ellipse at 50% 0%, black 15%, transparent 75%);
  }
</style>
