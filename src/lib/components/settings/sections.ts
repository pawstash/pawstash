import type { Component } from 'svelte';
import { i18n } from '$lib/i18n';
import { themeState } from '$lib/theme/themeState.svelte';
import { backgroundState, type BackgroundType } from '$lib/theme/backgroundState.svelte';
import { providerState } from '$lib/state/providerState.svelte';
import { formatProviderName } from '$lib/utils/media';
import IconAppearance from '~icons/fluent/paint-brush-24-regular';
import IconAppearanceFilled from '~icons/fluent/paint-brush-24-filled';
import IconInterface from '~icons/fluent/phone-laptop-24-regular';
import IconInterfaceFilled from '~icons/fluent/phone-laptop-24-filled';
import IconGrid from '~icons/fluent/grid-24-regular';
import IconGridFilled from '~icons/fluent/grid-24-filled';
import IconProviders from '~icons/fluent/globe-24-regular';
import IconProvidersFilled from '~icons/fluent/globe-24-filled';
import IconNetwork from '~icons/fluent/arrow-routing-24-regular';
import IconNetworkFilled from '~icons/fluent/arrow-routing-24-filled';
import IconDownloads from '~icons/fluent/arrow-download-24-regular';
import IconDownloadsFilled from '~icons/fluent/arrow-download-24-filled';
import IconNotifications from '~icons/fluent/alert-24-regular';
import IconNotificationsFilled from '~icons/fluent/alert-24-filled';
import IconCache from '~icons/fluent/database-24-regular';
import IconCacheFilled from '~icons/fluent/database-24-filled';
import IconSync from '~icons/fluent/cloud-sync-24-regular';
import IconSyncFilled from '~icons/fluent/cloud-sync-24-filled';
import IconAbout from '~icons/fluent/info-24-regular';
import IconAboutFilled from '~icons/fluent/info-24-filled';

export type SectionId =
  | 'appearance'
  | 'interface'
  | 'grid'
  | 'providers'
  | 'network'
  | 'downloads'
  | 'notifications'
  | 'cache'
  | 'sync'
  | 'about';

interface SectionMeta {
  labelKey: string;
  icon: Component;
  iconActive: Component;
}

const META: Record<SectionId, SectionMeta> = {
  appearance: { labelKey: 'settings.appearance_section', icon: IconAppearance, iconActive: IconAppearanceFilled },
  interface: { labelKey: 'settings.interface_section', icon: IconInterface, iconActive: IconInterfaceFilled },
  grid: { labelKey: 'settings.grid_section', icon: IconGrid, iconActive: IconGridFilled },
  providers: { labelKey: 'settings.providers_section', icon: IconProviders, iconActive: IconProvidersFilled },
  network: { labelKey: 'settings.network_section', icon: IconNetwork, iconActive: IconNetworkFilled },
  downloads: { labelKey: 'nav.downloads', icon: IconDownloads, iconActive: IconDownloadsFilled },
  notifications: { labelKey: 'settings.notifications_section', icon: IconNotifications, iconActive: IconNotificationsFilled },
  cache: { labelKey: 'settings.cache_section', icon: IconCache, iconActive: IconCacheFilled },
  sync: { labelKey: 'settings.sync_section', icon: IconSync, iconActive: IconSyncFilled },
  about: { labelKey: 'settings.about_section', icon: IconAbout, iconActive: IconAboutFilled }
};

export const SECTION_GROUPS: SectionId[][] = [
  ['appearance', 'interface', 'grid'],
  ['providers', 'downloads', 'notifications'],
  ['network', 'sync', 'cache'],
  ['about']
];

export const MOVED_SECTIONS: Record<string, SectionId> = {
  background: 'appearance',
  links: 'network',
  updates: 'about'
};

export const DEFAULT_SECTION: SectionId = 'appearance';

const BACKGROUND_LABEL_KEYS: Record<BackgroundType, string> = {
  acrylic: 'settings.bg_acrylic',
  vibrancy: 'settings.bg_vibrancy',
  'mica-dark': 'settings.bg_mica_dark',
  tabbed: 'settings.bg_tabbed',
  oled: 'settings.bg_oled',
  palette: 'settings.bg_palette',
  custom: 'settings.bg_custom'
};

export function backgroundLabel(type: BackgroundType): string {
  if (type === 'oled' && !themeState.isDark) return i18n.t('settings.bg_oled_light');
  if (type === 'mica-dark' && !themeState.isDark) return i18n.t('settings.bg_mica_light');
  return i18n.t(BACKGROUND_LABEL_KEYS[type]);
}

export function sectionMeta(id: SectionId): SectionMeta {
  return META[id];
}

export function isSectionId(value: string | undefined): value is SectionId {
  return value !== undefined && value in META;
}

export function providerIdOf(section: string | undefined): string | null {
  return section?.startsWith('provider-') ? section.slice('provider-'.length) : null;
}

export function navSectionOf(section: string | undefined): SectionId | null {
  if (providerIdOf(section)) return 'providers';
  return isSectionId(section) ? section : null;
}

export function sectionLabel(section: string): string {
  const providerId = providerIdOf(section);
  if (providerId) {
    const provider = providerState.providers.find((p) => p.id === providerId);
    return formatProviderName(provider?.name || providerId);
  }
  return isSectionId(section) ? i18n.t(META[section].labelKey) : section;
}

export function sectionDescription(id: SectionId): string {
  return i18n.t(`settings.section_desc_${id}`);
}
