/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import type { PageServerLoad } from './$types';
import { redirect } from '@sveltejs/kit';

export const prerender = false;
export const ssr = true;

export const load: PageServerLoad = async ({ url, fetch }) => {
	try {
		const response = await fetch('/api/auth/me');

		if (!response.ok) {
			const redirectTo = url.pathname + url.search;
			throw redirect(303, `/login?redirectTo=${encodeURIComponent(redirectTo)}`);
		}

		const user = await response.json();
		return { user };
	} catch (err) {
		if (err && typeof err === 'object' && 'status' in err && err.status === 303) {
			throw err;
		}

		const redirectTo = url.pathname + url.search;
		throw redirect(303, `/login?redirectTo=${encodeURIComponent(redirectTo)}`);
	}
};
