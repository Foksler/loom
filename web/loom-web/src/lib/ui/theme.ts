/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type ThemeMode = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'loom-theme';

function getInitialTheme(): ThemeMode {
	if (!browser) return 'system';

	const stored = localStorage.getItem(STORAGE_KEY);
	if (stored === 'light' || stored === 'dark' || stored === 'system') {
		return stored;
	}
	return 'system';
}

function getSystemTheme(): 'light' | 'dark' {
	if (!browser) return 'light';
	return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function applyTheme(mode: ThemeMode): void {
	if (!browser) return;

	const effectiveTheme = mode === 'system' ? getSystemTheme() : mode;

	if (effectiveTheme === 'dark') {
		document.documentElement.classList.add('dark');
	} else {
		document.documentElement.classList.remove('dark');
	}
}

function createThemeStore() {
	const { subscribe, set, update } = writable<ThemeMode>(getInitialTheme());

	return {
		subscribe,
		set: (mode: ThemeMode) => {
			if (browser) {
				localStorage.setItem(STORAGE_KEY, mode);
			}
			applyTheme(mode);
			set(mode);
		},
		toggle: () => {
			update((current) => {
				const next = current === 'light' ? 'dark' : current === 'dark' ? 'system' : 'light';
				if (browser) {
					localStorage.setItem(STORAGE_KEY, next);
				}
				applyTheme(next);
				return next;
			});
		},
		init: () => {
			const initial = getInitialTheme();
			applyTheme(initial);

			if (browser) {
				window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
					const current = getInitialTheme();
					if (current === 'system') {
						applyTheme('system');
					}
				});
			}
		},
	};
}

export const themeStore = createThemeStore();
