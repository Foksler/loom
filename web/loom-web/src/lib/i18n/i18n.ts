import { i18n } from '@lingui/core';

export type Locale = 'en' | 'es';

export const locales: Locale[] = ['en', 'es'];
export const defaultLocale: Locale = 'en';

export const localeNames: Record<Locale, string> = {
  en: 'English',
  es: 'Español',
};

export async function loadCatalog(locale: Locale): Promise<void> {
  let messages;
  
  switch (locale) {
    case 'es':
      messages = (await import('../../locales/es/messages')).messages;
      break;
    case 'en':
    default:
      messages = (await import('../../locales/en/messages')).messages;
      break;
  }
  
  i18n.load(locale, messages);
  i18n.activate(locale);
}

export function getPreferredLocale(): Locale {
  if (typeof window === 'undefined') return defaultLocale;
  
  const stored = localStorage.getItem('loom-locale');
  if (stored === 'en' || stored === 'es') return stored;
  
  const browserLang = navigator.language.split('-')[0];
  return browserLang === 'es' ? 'es' : 'en';
}

export function setLocale(locale: Locale): void {
  if (typeof window !== 'undefined') {
    localStorage.setItem('loom-locale', locale);
  }
  loadCatalog(locale);
}

export function getCurrentLocale(): Locale {
  return (i18n.locale as Locale) || defaultLocale;
}

export { i18n };
