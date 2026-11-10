<script lang="ts">
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import { scrollable } from '$lib/actions/scrollable';
  import Button from '$lib/components/ui/Button.svelte';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import WindowControls from '$lib/components/layout/WindowControls.svelte';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
  import SettingsGroup from './SettingsGroup.svelte';
  import { SECTION_GROUPS, navSectionOf, sectionMeta, sectionDescription } from './sections';

  let active = $derived(
    navigationState.route.name === 'settings' ? navSectionOf(navigationState.route.section) : null
  );
  let isMacStyle = $derived(layoutState.effectiveTitlebarStyle === 'macos');
</script>

<nav class="settings-nav" class:is-mac={isMacStyle} aria-label={i18n.t('nav.settings')} data-tauri-drag-region>
  <div class="settings-nav__header" data-tauri-drag-region="deep">
    {#if isMacStyle}
      <WindowControls variant="macos" />
    {/if}
    <Button variant="ghost" class="btn-icon" aria-label={i18n.t('nav.back')} onclick={() => navigationState.exitSettings()}>
      <IconArrowLeft />
    </Button>
    <h1 class="settings-nav__title">{i18n.t('nav.settings')}</h1>
  </div>

  <div
    class="settings-nav__scroll"
    data-overlayscrollbars-initialize
    use:scrollable={{ overflowX: 'hidden' }}
    data-tauri-drag-region="deep"
  >
    <div class="settings-nav__list">
      {#each SECTION_GROUPS as group, index (`group-${index}`)}
        <SettingsGroup>
          {#each group as id (id)}
            {@const meta = sectionMeta(id)}
            <SettingItem
              title={i18n.t(meta.labelKey)}
              summary={sectionDescription(id) || undefined}
              icon={meta.icon}
              iconActive={meta.iconActive}
              shapeSeed={id}
              active={active === id}
              onclick={() => {
                if (navigationState.route.name !== 'settings' || navigationState.route.section !== id) {
                  navigationState.openSettings(id);
                }
              }}
            />
          {/each}
        </SettingsGroup>
      {/each}
    </div>
  </div>
</nav>

<style>
  .settings-nav {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    width: calc(var(--settings-nav-width) + var(--edge-left, 0px));
    height: 100%;
    padding: var(--edge-top, 0px) 0 var(--edge-bottom, 0px) var(--edge-left, 0px);
    background: transparent;
  }

  .settings-nav__header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
    height: var(--toolbar-height);
    padding-inline: 8px 12px;
    --control-height: var(--control-height-base);
    --control-icon-size: 24px;
  }

  .settings-nav.is-mac .settings-nav__header {
    gap: 12px;
    padding-inline-start: 16px;
  }

  .settings-nav__title {
    margin: 0;
    color: var(--text-primary);
    font-size: var(--type-title-md-size);
    line-height: var(--type-title-md-line);
    font-weight: 500;
  }

  .settings-nav__scroll {
    flex: 1 1 0;
    min-height: 0;
  }

  .settings-nav__list {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 8px 8px 16px;
  }
</style>
