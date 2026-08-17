import ru from './locales/ru.json';
import en from './locales/en.json';

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

const translations: Record<Locale, typeof ru> = {
  ru,
  en,
};

const STORAGE_KEY = 'pawstash_locale';

export class I18nState {
  currentLocale = $state<Locale>('ru');

  init() {
    if (typeof localStorage !== 'undefined') {
      const saved = localStorage.getItem(STORAGE_KEY) as Locale | null;
      if (saved && saved in translations) {
        this.currentLocale = saved;
      } else {
        const browserLang = navigator.language.split('-')[0];
        if (browserLang === 'en') {
          this.currentLocale = 'en';
        } else {
          this.currentLocale = 'ru';
        }
      }
      document.documentElement.lang = this.currentLocale;
    }
  }

  setLocale(locale: Locale) {
    if (locale in translations) {
      this.currentLocale = locale;
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, locale);
        document.documentElement.lang = locale;
      }
    }
  }

  t(key: string, vars?: Record<string, string | number>): string {
    const translation = this.getNestedValue(translations[this.currentLocale], key)
      || this.getNestedValue(translations['ru'], key)
      || key;

    return this.interpolate(translation, vars);
  }

  private getNestedValue(obj: Record<string, unknown>, path: string): string | undefined {
    const keys = path.split('.');
    let current: unknown = obj;

    for (const key of keys) {
      if (current === null || current === undefined || typeof current !== 'object') {
        return undefined;
      }
      current = (current as Record<string, unknown>)[key];
    }

    return typeof current === 'string' ? current : undefined;
  }

  private interpolate(str: string, vars?: Record<string, string | number>): string {
    if (!vars) return str;
    return str.replace(/\{(\w+)\}/g, (_, key) => vars[key]?.toString() ?? `{${key}}`);
  }
}

export const i18n = new I18nState();
