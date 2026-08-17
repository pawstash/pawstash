<script lang="ts">
  import type { Snippet } from 'svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { i18n } from '$lib/i18n';
  import Button from '$lib/components/ui/Button.svelte';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';

  interface Props {
    title?: string | Snippet;
    subtitle?: string | Snippet;
    badge?: string | number | Snippet;
    back?: boolean | (() => void);
    tabs?: Snippet;
    filters?: Snippet;
    actions?: Snippet;
    extra?: Snippet;
    class?: string;
  }

  let {
    title,
    subtitle,
    badge,
    back = false,
    tabs,
    filters,
    actions,
    extra,
    class: extraClass = ''
  }: Props = $props();

  function handleBack() {
    if (typeof back === 'function') {
      back();
    } else {
      navigationState.back();
    }
  }
</script>

<header class="unified-page-header {extraClass}" class:is-mobile={layoutState.isMobile}>
  {#if !layoutState.isMobile}
    <div class="header-desktop-row">
      <div class="header-leading-zone">
        {#if back}
          <Button variant="ghost" size="md" onclick={handleBack} class="btn-back">
            <IconArrowLeft class="w-5 h-5" />
            <span>{i18n.t('nav.back') || 'Back'}</span>
          </Button>
        {/if}

        {#if title}
          <div class="header-title-wrapper">
            <div class="flex items-center gap-2">
              {#if typeof title === 'string'}
                <h1 class="header-title-text">{title}</h1>
              {:else}
                {@render title()}
              {/if}

              {#if badge !== undefined}
                {#if typeof badge === 'string' || typeof badge === 'number'}
                  <span class="header-badge">{badge}</span>
                {:else}
                  {@render badge()}
                {/if}
              {/if}
            </div>

            {#if subtitle}
              {#if typeof subtitle === 'string'}
                <p class="header-subtitle-text">{subtitle}</p>
              {:else}
                {@render subtitle()}
              {/if}
            {/if}
          </div>
        {/if}

        {#if tabs}
          <div class="header-inline-tabs">
            {@render tabs()}
          </div>
        {/if}

        {#if filters}
          <div class="header-inline-filters">
            {@render filters()}
          </div>
        {/if}
      </div>

      {#if actions}
        <div class="header-trailing-zone">
          {@render actions()}
        </div>
      {/if}
    </div>

    {#if extra}
      <div class="header-extra-row">
        {@render extra()}
      </div>
    {/if}
  {:else}
    <div class="header-mobile-wrapper">
      <div class="mobile-tier-1">
        <div class="mobile-leading-zone flex items-center gap-2 min-w-0 flex-1">
          {#if back}
            <Button variant="ghost" size="md" onclick={handleBack} class="btn-icon">
              <IconArrowLeft class="w-5 h-5" />
            </Button>
          {/if}

          {#if title}
            <div class="flex items-center gap-1.5 min-w-0">
              {#if typeof title === 'string'}
                <h1 class="mobile-title-text truncate">{title}</h1>
              {:else}
                {@render title()}
              {/if}

              {#if badge !== undefined}
                {#if typeof badge === 'string' || typeof badge === 'number'}
                  <span class="header-badge">{badge}</span>
                {:else}
                  {@render badge()}
                {/if}
              {/if}
            </div>
          {/if}
        </div>

        {#if actions}
          <div class="mobile-actions-wrapper">
            {@render actions()}
          </div>
        {/if}
      </div>

      {#if tabs || filters}
        <div class="mobile-tier-2">
          {#if tabs}
            <div class="mobile-tabs-scroll">
              {@render tabs()}
            </div>
          {/if}

          {#if filters}
            <div class="mobile-filters-wrapper">
              {@render filters()}
            </div>
          {/if}
        </div>
      {/if}

      {#if extra}
        <div class="mobile-extra-wrapper">
          {@render extra()}
        </div>
      {/if}
    </div>
  {/if}
</header>

<style>
  .unified-page-header {
    width: 100%;
    box-sizing: border-box;
    margin-bottom: 24px;
  }

  .header-desktop-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 48px;
    gap: 16px;
  }

  .header-leading-zone {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
    flex-wrap: nowrap;
  }

  .header-inline-tabs,
  .header-inline-filters {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .header-trailing-zone {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    flex-shrink: 0;
  }

  .header-extra-row {
    margin-top: 12px;
  }

  .header-mobile-wrapper {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .mobile-tier-1 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 44px;
    position: relative;
    gap: 8px;
  }

  .mobile-tier-1:has(:global(.search-active)) .mobile-leading-zone,
  .mobile-tier-1:has(:global(.search-active)) > div:first-child {
    display: none !important;
  }

  .mobile-tier-1:has(:global(.search-active)) .mobile-actions-wrapper {
    width: 100%;
    flex: 1;
  }

  .mobile-title-text {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
    letter-spacing: -0.2px;
  }

  .mobile-actions-wrapper {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .mobile-tier-2 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
    padding-bottom: 2px;
  }

  .mobile-tier-2::-webkit-scrollbar {
    display: none;
  }

  .mobile-tabs-scroll {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .mobile-tabs-scroll::-webkit-scrollbar {
    display: none;
  }

  .mobile-filters-wrapper {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .mobile-extra-wrapper {
    margin-top: 4px;
  }
</style>
