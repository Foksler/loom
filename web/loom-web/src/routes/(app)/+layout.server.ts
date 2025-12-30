/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import type { LayoutServerLoad } from './$types';
import { redirect } from '@sveltejs/kit';

// Protected routes cannot be prerendered - they require server-side auth
export const prerender = false;
export const ssr = true;

export const load: LayoutServerLoad = async ({ url, fetch }) => {
	try {
		const response = await fetch('/api/auth/me');
		
		if (!response.ok) {
			const redirectTo = url.pathname + url.search;
			throw redirect(303, `/login?redirectTo=${encodeURIComponent(redirectTo)}`);
		}
		
		const user = await response.json();
		return { user };
	} catch (err) {
		// If it's already a redirect, re-throw it
		if (err && typeof err === 'object' && 'status' in err && err.status === 303) {
			throw err;
		}
		
		// For other errors (network, etc.), redirect to login
		const redirectTo = url.pathname + url.search;
		throw redirect(303, `/login?redirectTo=${encodeURIComponent(redirectTo)}`);
	}
};
