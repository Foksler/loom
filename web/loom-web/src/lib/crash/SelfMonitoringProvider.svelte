<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

<script lang="ts">
	import { onMount } from 'svelte';
	import type { Snippet } from 'svelte';
	import { initializeSelfMonitoring, shutdownSelfMonitoring } from './self-monitoring';

	interface Props {
		children: Snippet;
		debug?: boolean;
	}

	let { children, debug = false }: Props = $props();

	onMount(() => {
		// Initialize self-monitoring asynchronously
		initializeSelfMonitoring(debug);

		// Cleanup on unmount
		return () => {
			shutdownSelfMonitoring();
		};
	});
</script>

{@render children()}
