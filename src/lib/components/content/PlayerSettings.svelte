<script lang="ts">
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { ripple } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import { PLAYBACK_RATES } from '$lib/state/playerPrefs.svelte';
  import IconSettings from '~icons/fluent/settings-24-regular';
  import IconGauge from '~icons/fluent/gauge-24-regular';
  import IconLoop from '~icons/fluent/arrow-repeat-all-24-regular';
  import IconChevronRight from '~icons/fluent/chevron-right-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';

  interface Props {
    playbackRate: number;
    loopEnabled: boolean;
    onrate: (rate: number) => void;
    onloop: () => void;
  }

  let { playbackRate, loopEnabled, onrate, onloop }: Props = $props();


  let open = $state(false);
  let speedOpen = $state(false);
  let speedRowEl = $state<HTMLButtonElement>();

  $effect(() => {
    if (!open) speedOpen = false;
  });

  function pickRate(rate: number) {
    onrate(rate);
    speedOpen = false;
    open = false;
  }
</script>

<PopoverMenu bind:open align="right" width="auto" menuClass="player-settings-menu">
  {#snippet trigger({ toggle, open: isOpen })}
    <Button
      variant="ghost"
      class="btn-icon player-icon-btn"
      onclick={toggle}
      title={i18n.t('post.viewer_settings')}
      aria-label={i18n.t('post.viewer_settings')}
      aria-haspopup="menu"
      aria-expanded={isOpen}
    >
      <IconSettings />
    </Button>
  {/snippet}

  {#snippet children({ close })}
    <div class="player-menu-list">
      <button
        bind:this={speedRowEl}
        type="button"
        class="floating-item"
        class:is-open={speedOpen}
        onclick={() => (speedOpen = !speedOpen)}
        aria-haspopup="menu"
        aria-expanded={speedOpen}
        use:ripple
      >
        <IconGauge />
        <span class="floating-item-label">{i18n.t('post.viewer_speed')}</span>
        <span class="floating-item-detail">{playbackRate}×</span>
        <IconChevronRight />
      </button>

      <button
        type="button"
        class="floating-item"
        role="menuitemcheckbox"
        aria-checked={loopEnabled}
        onclick={() => {
          onloop();
          close();
        }}
        use:ripple
      >
        <IconLoop />
        <span class="floating-item-label">{i18n.t('post.viewer_loop')}</span>
        {#if loopEnabled}<IconCheckmark />{/if}
      </button>
    </div>

    <PopoverMenu bind:open={speedOpen} anchorEl={speedRowEl} align="left" width="auto" keepOpen menuClass="player-settings-menu">
      <div class="player-menu-list">
        {#each PLAYBACK_RATES as rate (rate)}
          {@const active = Math.abs(rate - playbackRate) < 0.001}
          <button
            type="button"
            class="floating-item"
            role="menuitemradio"
            aria-checked={active}
            onclick={() => pickRate(rate)}
            use:ripple
          >
            <span class="floating-item-label">{rate}×</span>
            {#if active}<IconCheckmark />{/if}
          </button>
        {/each}
      </div>
    </PopoverMenu>
  {/snippet}
</PopoverMenu>

<style>
  .floating-item.is-open {
    background: var(--state-layer-hover);
  }
</style>
