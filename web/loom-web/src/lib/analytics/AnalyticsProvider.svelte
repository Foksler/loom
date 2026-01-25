<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

<script lang="ts">
	import { onMount } from 'svelte';
	import type { Snippet } from 'svelte';
	import { initializeAnalytics, identify, reset, shutdownAnalytics } from './self-monitoring';

	interface Props {
		children: Snippet;
		/** User to identify with analytics. Pass null/undefined for anonymous users. */
		user?: { id: string; email?: string | null; display_name?: string | null } | null;
		/** Enable debug logging */
		debug?: boolean;
	}

	let { children, user = null, debug = false }: Props = $props();

	let isInitialized = $state(false);
	let previousUserId = $state<string | null>(null);

	onMount(() => {
		// Initialize analytics asynchronously
		initializeAnalytics(debug).then((client) => {
			if (client) {
				isInitialized = true;

				// If user is already logged in, identify them
				if (user?.id) {
					const props: Record<string, string> = {};
					if (user.email) props.email = user.email;
					if (user.display_name) props.display_name = user.display_name;
					identify(user.id, props);
					previousUserId = user.id;
				}
			}
		});

		// Cleanup on unmount
		return () => {
			shutdownAnalytics();
		};
	});

	// React to user changes (login/logout)
	$effect(() => {
		if (!isInitialized) return;

		if (user?.id && user.id !== previousUserId) {
			// User logged in or changed - identify them
			const props: Record<string, string> = {};
			if (user.email) props.email = user.email;
			if (user.display_name) props.display_name = user.display_name;
			identify(user.id, props);
			previousUserId = user.id;
		} else if (!user?.id && previousUserId) {
			// User logged out - reset analytics identity
			reset();
			previousUserId = null;
		}
	});
</script>

{@render children()}
