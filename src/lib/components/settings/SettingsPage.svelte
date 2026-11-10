<script lang="ts">
  import { onMount, type Component } from 'svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { providerState } from '$lib/state/providerState.svelte';
  import { syncState } from '$lib/state/syncState.svelte';
  import { i18n } from '$lib/i18n';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import type { Crumb } from '$lib/components/layout/TopAppBar.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import { searchSettings, revealSetting, type SettingMatch } from './settingsSearch';
  import { notify } from '$lib/utils/toast';
  import IconSearch from '~icons/fluent/search-24-regular';
  import type { MenuItem } from '$lib/components/layout/OverflowMenu.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import SettingsGroup from './SettingsGroup.svelte';
  import VersionLine from './VersionLine.svelte';
  import { settingsController as ctl, sectionIsResettable } from './settingsController.svelte';
  import {
    DEFAULT_SECTION,
    MOVED_SECTIONS,
    SECTION_GROUPS,
    isSectionId,
    providerIdOf,
    sectionLabel,
    sectionMeta,
    sectionDescription,
    type SectionId
  } from './sections';
  import AppearanceSection from './sections/AppearanceSection.svelte';
  import InterfaceSection from './sections/InterfaceSection.svelte';
  import GridSection from './sections/GridSection.svelte';
  import ProvidersSection from './sections/ProvidersSection.svelte';
  import ProviderSection from './sections/ProviderSection.svelte';
  import NetworkSection from './sections/NetworkSection.svelte';
  import DownloadsSection from './sections/DownloadsSection.svelte';
  import NotificationsSection from './sections/NotificationsSection.svelte';
  import CacheSection from './sections/CacheSection.svelte';
  import SyncSection from './sections/SyncSection.svelte';
  import AboutSection from './sections/AboutSection.svelte';
  import IconArrowReset from '~icons/fluent/arrow-reset-24-regular';
  import IconPerson from '~icons/fluent/person-24-regular';
  import IconCloudSync from '~icons/fluent/cloud-sync-24-regular';
  import { syncStatusText } from '$lib/utils/syncStatus';
  import { COMMUNITY_SETTINGS, openCommunity } from '$lib/utils/community';

  const SECTIONS: Record<SectionId, Component> = {
    appearance: AppearanceSection,
    interface: InterfaceSection,
    grid: GridSection,
    providers: ProvidersSection,
    network: NetworkSection,
    downloads: DownloadsSection,
    notifications: NotificationsSection,
    cache: CacheSection,
    sync: SyncSection,
    about: AboutSection
  };

  let searchOpen = $state(false);
  let searchQuery = $state('');
  let searching = $derived(searchQuery.trim().length > 0);
  let section = $derived(navigationState.route.name === 'settings' ? navigationState.route.section : undefined);
  let providerId = $derived(providerIdOf(section));
  let isRootList = $derived(!section);
  let title = $derived(section && !searching ? sectionLabel(section) : i18n.t('nav.settings'));
  let SectionView = $derived(isSectionId(section) ? SECTIONS[section] : null);
  let trail = $derived.by<Crumb[]>(() => {
    if (searching || !section || !providerId) return [];
    return [
      { label: sectionLabel('providers'), onclick: () => navigationState.openSettings('providers') },
      { label: title }
    ];
  });
  let canReset = $derived(Boolean(section) && sectionIsResettable(section!));
  let confirmReset = $state(false);
  let pageMenu = $derived<MenuItem[]>(
    canReset ? [{ label: i18n.t('settings.reset_section_title'), icon: IconArrowReset, onclick: () => (confirmReset = true) }] : []
  );

  let resultGroups = $derived.by(() => {
    const groups: { section: string; title: string; items: SettingMatch[] }[] = [];
    for (const match of searchSettings(searchQuery)) {
      let group = groups.find((item) => item.section === match.section);
      if (!group) groups.push((group = { section: match.section, title: match.sectionTitle, items: [] }));
      group.items.push(match);
    }
    return groups;
  });

  function openResult(match: SettingMatch) {
    searchQuery = '';
    searchOpen = false;
    navigationState.openSettings(match.section);
    revealSetting(match.title, () => notify.info(i18n.t('settings.search_hidden_title', { title: match.title }), i18n.t('settings.search_hidden_desc')));
  }

  $effect(() => {
    const known = !section || isSectionId(section) || providerIdOf(section);
    if (known && (section || layoutState.isMobile)) return;
    const fallback = (section && MOVED_SECTIONS[section]) || (layoutState.isMobile ? undefined : DEFAULT_SECTION);
    navigationState.replaceRoute({ name: 'settings', section: fallback });
  });

  $effect(() => {
    void providerState.providers;
    ctl.refreshWebviewProxyStatus();
  });

  onMount(() => {
    void ctl.load();
  });

  function back() {
    if (layoutState.isMobile && isRootList) {
      navigationState.exitSettings();
      return;
    }
    if (!navigationState.canGoBack) {
      navigationState.replaceRoute({
        name: 'settings',
        section: providerId ? 'providers' : layoutState.isMobile ? undefined : DEFAULT_SECTION
      });
      return;
    }
    navigationState.back();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.defaultPrevented || layoutState.isMobile) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest('input, textarea, [contenteditable], [role="dialog"], [role="menu"]')) return;
    if (document.querySelector('[role="dialog"], .select-dropdown-portal, .popover-portal')) return;
    if (providerId) back();
    else navigationState.exitSettings();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<PageShell
  {title}
  {trail}
  largeTitle
  back={layoutState.isMobile ? back : false}
  hideOnScroll={false}
  menu={pageMenu}
  scrollKey={navigationState.entryKey}
>
  {#snippet actions()}
    <HeaderActions
      pinOpen
      bind:searchOpen
      bind:searchQuery
      searchPlaceholder={i18n.t('settings.search_placeholder')}
    />
  {/snippet}

  <div class="settings-body" data-control-size={layoutState.isMobile ? 'md' : 'base'}>
    {#if searching}
      {#if resultGroups.length === 0}
        <EmptyState icon={IconSearch} title={i18n.t('settings.search_empty_title')} description={i18n.t('settings.search_empty_desc', { query: searchQuery.trim() })} />
      {:else}
        {#each resultGroups as group (group.section)}
          <SettingsGroup label={group.title}>
            {#each group.items as match, index (`${match.titleKey}:${index}`)}
              <SettingItem title={match.title} description={match.description} onclick={() => openResult(match)} />
            {/each}
          </SettingsGroup>
        {/each}
      {/if}
    {:else if isRootList}
      <SettingsGroup>
        <SettingItem
          title={i18n.t('profile.title')}
          summary={syncStatusText()}
          icon={syncState.status.configured ? IconCloudSync : IconPerson}
          shapeSeed="profile"
          onclick={() => navigationState.navigateRoot('profile')}
        />
      </SettingsGroup>
      {#each SECTION_GROUPS as group}
        <SettingsGroup>
          {#each group as id (id)}
            <SettingItem
              title={i18n.t(sectionMeta(id).labelKey)}
              summary={sectionDescription(id) || undefined}
              icon={sectionMeta(id).icon}
              iconActive={sectionMeta(id).iconActive}
              shapeSeed={id}
              onclick={() => navigationState.openSettings(id)}
            />
          {/each}
        </SettingsGroup>
      {/each}
      <SettingsGroup label={i18n.t('settings.community')}>
        {#each COMMUNITY_SETTINGS as link (link.id)}
          <SettingItem
            title={link.label}
            summary={i18n.t(`settings.community_${link.id}`)}
            icon={link.icon}
            shapeSeed={link.id}
            external
            onclick={() => openCommunity(link)}
          />
        {/each}
      </SettingsGroup>
      <div class="root-version"><VersionLine /></div>
    {:else if providerId}
      {#key providerId}
        <ProviderSection {providerId} />
      {/key}
    {:else if SectionView}
      <SectionView />
    {/if}
  </div>
</PageShell>

<ConfirmDialog
  isOpen={confirmReset}
  title={i18n.t('settings.reset_section_title')}
  description={i18n.t('settings.reset_section_confirm')}
  confirmLabel={i18n.t('common.reset')}
  confirmVariant="danger"
  confirmIcon={IconArrowReset}
  onconfirm={() => {
    confirmReset = false;
    if (section) void ctl.resetSection(section, title);
  }}
  onclose={() => (confirmReset = false)}
/>

<style>
  .root-version :global(.version-line) {
    padding: 0 16px;
  }

  .settings-body {
    display: flex;
    flex-direction: column;
    gap: 24px;
    width: 100%;
    max-width: var(--content-max-width);
    margin-inline: auto;
    padding-block: 8px 24px;
  }
</style>
