import { i18n } from '$lib/i18n';
import { sectionLabel, type SectionId } from './sections';

interface SettingEntry {
  section: SectionId;
  titleKey: string;
  descriptionKey?: string;
  groupKey?: string;
}

export interface SettingMatch extends SettingEntry {
  title: string;
  description?: string;
  sectionTitle: string;
}

const SOURCES = import.meta.glob('./sections/*Section.svelte', { query: '?raw', import: 'default', eager: true }) as Record<
  string,
  string
>;

const FILE_SECTIONS: Record<string, SectionId> = {
  AppearanceSection: 'appearance',
  BackgroundSection: 'appearance',
  InterfaceSection: 'interface',
  GridSection: 'grid',
  PlayerSection: 'player',
  ProvidersSection: 'providers',
  NetworkSection: 'network',
  LinksSection: 'network',
  DownloadsSection: 'downloads',
  NotificationsSection: 'notifications',
  CacheSection: 'cache',
  SyncSection: 'sync',
  AboutSection: 'about'
};

const TAG = /<(SettingItem|SettingChoice|SettingsGroup)\b/g;

function tagAttributes(source: string, from: number): string {
  let depth = 0;
  for (let i = from; i < source.length; i++) {
    const ch = source[i];
    if (ch === '{') depth++;
    else if (ch === '}') depth--;
    else if (ch === '>' && depth === 0) return source.slice(from, i);
  }
  return '';
}

function keyOf(attributes: string, name: string): string | undefined {
  return new RegExp(`\\b${name}=\\{i18n\\.t\\('([^']+)'`).exec(attributes)?.[1];
}

function buildIndex(): SettingEntry[] {
  const entries: SettingEntry[] = [];
  for (const [path, source] of Object.entries(SOURCES)) {
    const section = FILE_SECTIONS[path.split('/').pop()!.replace('.svelte', '')];
    if (!section) continue;
    let groupKey: string | undefined;
    for (const match of source.matchAll(TAG)) {
      const attributes = tagAttributes(source, match.index! + match[0].length);
      if (match[1] === 'SettingsGroup') {
        groupKey = keyOf(attributes, 'label');
        continue;
      }
      const titleKey = keyOf(attributes, 'title');
      if (titleKey) entries.push({ section, titleKey, descriptionKey: keyOf(attributes, 'description'), groupKey });
    }
  }
  return entries;
}

const INDEX = buildIndex();

function normalize(text: string): string {
  return text.toLocaleLowerCase().normalize('NFKD').replace(/\p{M}/gu, '');
}

export function searchSettings(query: string): SettingMatch[] {
  const words = normalize(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return [];
  const scored: { match: SettingMatch; score: number }[] = [];
  for (const entry of INDEX) {
    const title = i18n.t(entry.titleKey);
    const description = entry.descriptionKey ? i18n.t(entry.descriptionKey) : undefined;
    const sectionTitle = sectionLabel(entry.section);
    const group = entry.groupKey ? i18n.t(entry.groupKey) : '';
    const titleText = normalize(title);
    const haystack = normalize(`${title} ${description ?? ''} ${group} ${sectionTitle}`);
    if (!words.every((word) => haystack.includes(word))) continue;
    const score = words.reduce(
      (total, word) => total + (titleText.startsWith(word) ? 3 : titleText.includes(word) ? 2 : 0),
      0
    );
    scored.push({ match: { ...entry, title, description, sectionTitle }, score });
  }
  return scored.sort((a, b) => b.score - a.score).map((item) => item.match);
}

export function revealSetting(title: string, onmissing?: () => void) {
  const deadline = performance.now() + 1500;
  const find = () => {
    const row = [...document.querySelectorAll<HTMLElement>('.setting-item')].find(
      (item) => item.querySelector('.setting-item__title')?.textContent?.trim() === title
    );
    if (row) {
      row.scrollIntoView({ block: 'center', behavior: 'smooth' });
      row.classList.remove('is-revealed');
      void row.offsetWidth;
      row.classList.add('is-revealed');
      return;
    }
    if (performance.now() < deadline) requestAnimationFrame(find);
    else onmissing?.();
  };
  requestAnimationFrame(find);
}
