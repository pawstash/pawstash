export type Locale = 'ru' | 'en';

export interface LocaleConfig {
  code: Locale;
  name: string;
  nativeName: string;
}

export const LOCALES: LocaleConfig[] = [
  { code: 'ru', name: 'Russian', nativeName: 'Русский' },
  { code: 'en', name: 'English', nativeName: 'English' },
];

type Messages = { [key: string]: string | Messages };
type PluralForms = Partial<Record<Intl.LDMLPluralRule, string>> & { other: string };
type Vars = Record<string, string | number>;
type Formatter = Intl.NumberFormat | Intl.DateTimeFormat | Intl.PluralRules | Intl.RelativeTimeFormat;

const LOCALE_LOADERS: Record<Locale, () => Promise<{ default: Messages }>> = {
  ru: () => import('./locales/ru.json'),
  en: () => import('./locales/en.json')
};
const translations: Partial<Record<Locale, Messages>> = {};
const FALLBACK_LOCALE: Locale = 'en';
const LOCALE_TAGS: Record<Locale, string> = { ru: 'ru-RU', en: 'en-US' };
const RUSSIAN_SPEAKING = new Set(['ru', 'uk', 'be', 'kk']);
const PLURAL_CATEGORIES = new Set(['zero', 'one', 'two', 'few', 'many', 'other']);
const BYTE_UNITS = ['byte', 'kilobyte', 'megabyte', 'gigabyte', 'terabyte'] as const;
const RELATIVE_STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['second', 60],
  ['minute', 60],
  ['hour', 24],
  ['day', 7],
  ['week', 4.34524],
  ['month', 12],
];

const STORAGE_KEY = 'pawstash_locale';

function isPluralForms(value: unknown): value is PluralForms {
  if (!value || typeof value !== 'object') return false;
  const keys = Object.keys(value);
  return keys.includes('other') && keys.every((key) => PLURAL_CATEGORIES.has(key));
}

function lookup(messages: Messages, key: string): string | PluralForms | undefined {
  let current: string | Messages | undefined = messages;
  for (const part of key.split('.')) {
    if (!current || typeof current !== 'object') return undefined;
    current = current[part];
  }
  if (typeof current === 'string' || isPluralForms(current)) return current;
  return undefined;
}

function detectLocale(): Locale {
  const lang = navigator.language.split('-')[0].toLowerCase();
  return RUSSIAN_SPEAKING.has(lang) ? 'ru' : 'en';
}

export class I18nState {
  currentLocale = $state<Locale>('ru');

  private formatCache = new Map<string, Formatter>();
  private missingReported = new Set<string>();
  private loading = new Map<Locale, Promise<void>>();

  get tag(): string {
    return LOCALE_TAGS[this.currentLocale];
  }

  private load(locale: Locale): Promise<void> {
    const pending = this.loading.get(locale);
    if (pending) return pending;
    const promise = LOCALE_LOADERS[locale]().then((module) => {
      translations[locale] = module.default;
    });
    this.loading.set(locale, promise);
    return promise;
  }

  async init() {
    const saved = typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_KEY) : null;
    const locale = saved === 'ru' || saved === 'en' ? saved : detectLocale();
    try {
      await this.load(locale);
    } catch {
      await this.load(FALLBACK_LOCALE);
    }
    this.currentLocale = translations[locale] ? locale : FALLBACK_LOCALE;
    if (typeof document !== 'undefined') document.documentElement.lang = this.currentLocale;
  }

  async setLocale(locale: Locale) {
    if (!(locale in LOCALE_LOADERS)) return;
    try {
      await this.load(locale);
    } catch {
      return;
    }
    this.currentLocale = locale;
    document.documentElement.lang = locale;
    localStorage.setItem(STORAGE_KEY, locale);
  }

  has(key: string): boolean {
    const messages = translations[this.currentLocale];
    return messages ? lookup(messages, key) !== undefined : false;
  }

  /** `count` picks the form when the message is a plural object. */
  t(key: string, vars?: Vars): string {
    const current = translations[this.currentLocale];
    const fallback = translations[FALLBACK_LOCALE];
    const entry =
      (current && lookup(current, key)) ??
      (fallback ? lookup(fallback, key) : undefined);
    if (entry === undefined) {
      if (import.meta.env.DEV && !this.missingReported.has(key)) {
        this.missingReported.add(key);
        console.warn(`[i18n] missing key: ${key}`);
      }
      return key;
    }
    const template = typeof entry === 'string' ? entry : this.pluralForm(entry, vars?.count);
    return this.interpolate(template, vars);
  }

  number(value: number, options?: Intl.NumberFormatOptions): string {
    return this.cached(`n:${JSON.stringify(options ?? {})}`, () => new Intl.NumberFormat(this.tag, options)).format(value);
  }

  /** Plain bytes use the long unit name: en's short form is the singular "byte". */
  bytes(bytes: number, maximumFractionDigits = 2): string {
    if (!Number.isFinite(bytes) || bytes < 1024) {
      const whole = Number.isFinite(bytes) ? Math.max(0, Math.round(bytes)) : 0;
      return this.number(whole, { style: 'unit', unit: 'byte', unitDisplay: 'long' });
    }
    const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), BYTE_UNITS.length - 1);
    return this.number(bytes / 1024 ** exponent, {
      style: 'unit',
      unit: BYTE_UNITS[exponent],
      unitDisplay: 'short',
      maximumFractionDigits,
    });
  }

  date(value: Date | number, options: Intl.DateTimeFormatOptions): string {
    return this.cached(`d:${JSON.stringify(options)}`, () => new Intl.DateTimeFormat(this.tag, options)).format(value);
  }

  relativeTime(value: Date | number): string {
    const format = this.cached('rt', () => new Intl.RelativeTimeFormat(this.tag, { numeric: 'auto' }));
    let amount = ((typeof value === 'number' ? value : value.getTime()) - Date.now()) / 1000;
    for (const [unit, size] of RELATIVE_STEPS) {
      if (Math.abs(amount) < size) return format.format(Math.round(amount), unit);
      amount /= size;
    }
    return format.format(Math.round(amount), 'year');
  }

  private pluralForm(forms: PluralForms, count: string | number | undefined): string {
    if (typeof count !== 'number') return forms.other;
    const rules = this.cached('plural', () => new Intl.PluralRules(this.tag));
    return forms[rules.select(count)] ?? forms.other;
  }

  private interpolate(template: string, vars?: Vars): string {
    if (!vars) return template;
    return template.replace(/\{(\w+)\}/g, (match, name: string) => {
      const value = vars[name];
      if (value === undefined) return match;
      return typeof value === 'number' ? this.number(value) : value;
    });
  }

  private cached<T extends Formatter>(id: string, create: () => T): T {
    const key = `${this.tag}|${id}`;
    let formatter = this.formatCache.get(key) as T | undefined;
    if (!formatter) {
      formatter = create();
      this.formatCache.set(key, formatter);
    }
    return formatter;
  }
}

export const i18n = new I18nState();
