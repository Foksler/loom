/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { i18n } from '@lingui/core';

export type Locale = 'en' | 'es' | 'ar';

export const locales: Locale[] = ['en', 'es', 'ar'];
export const defaultLocale: Locale = 'en';

export const localeNames: Record<Locale, string> = {
	en: 'English',
	es: 'Español',
	ar: 'العربية',
};

export const rtlLocales: Locale[] = ['ar'];

export function isRtl(locale: Locale): boolean {
	return rtlLocales.includes(locale);
}

export async function loadCatalog(locale: Locale): Promise<void> {
	let messages;

	switch (locale) {
		case 'es':
			messages = (await import('../../locales/es/messages')).messages;
			break;
		case 'ar':
			messages = (await import('../../locales/ar/messages')).messages;
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
	if (stored === 'en' || stored === 'es' || stored === 'ar') return stored;

	const browserLang = navigator.language.split('-')[0];
	if (browserLang === 'es') return 'es';
	if (browserLang === 'ar') return 'ar';
	return 'en';
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
