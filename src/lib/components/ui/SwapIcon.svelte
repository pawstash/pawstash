<script lang="ts">
  import type { Component } from 'svelte';

  interface Props {
    icon: Component;
    iconActive?: Component;
    active?: boolean;
    size?: number;
  }

  let { icon: Icon, iconActive: IconActive, active = false, size = 24 }: Props = $props();
  let ActiveIcon = $derived(IconActive ?? Icon);
</script>

<span
  class="swap-icon"
  class:is-active={active}
  style="--swap-icon-size: {size}px"
  aria-hidden="true"
>
  <span class="swap-icon__layer swap-icon__layer--idle"><Icon /></span>
  <span class="swap-icon__layer swap-icon__layer--active"><ActiveIcon /></span>
</span>

<style>
  .swap-icon {
    position: relative;
    display: grid;
    place-items: center;
    width: var(--swap-icon-size);
    height: var(--swap-icon-size);
    flex: none;
  }

  .swap-icon__layer {
    grid-area: 1 / 1;
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    transition:
      opacity var(--duration-normal) var(--ease-standard),
      transform var(--duration-normal) var(--ease-expo);
  }

  .swap-icon__layer :global(svg) {
    width: 100%;
    height: 100%;
  }

  .swap-icon__layer--idle {
    opacity: 1;
    transform: none;
  }

  .swap-icon.is-active .swap-icon__layer--idle {
    opacity: 0;
    transform: scale(0.6) rotate(-10deg);
  }

  .swap-icon__layer--active {
    opacity: 0;
    transform: scale(0.6) rotate(10deg);
  }

  .swap-icon.is-active .swap-icon__layer--active {
    opacity: 1;
    transform: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .swap-icon__layer {
      transition: none;
    }
  }
</style>
