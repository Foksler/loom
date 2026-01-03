<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { DocSearch } from '$lib/docs/components';

	let { children } = $props();
</script>

<div class="docs-layout">
	<aside class="sidebar">
		<div class="sidebar-header">
			<a href="/docs" class="sidebar-logo">Loom Docs</a>
			<div class="sidebar-search">
				<DocSearch />
			</div>
		</div>
	</aside>

	<main class="docs-content" data-pagefind-body>
		{@render children()}
	</main>
</div>

<style>
	.docs-layout {
		display: grid;
		grid-template-columns: 280px 1fr;
		min-height: 100vh;
	}

	.sidebar {
		position: sticky;
		top: 0;
		height: 100vh;
		overflow-y: auto;
		border-right: 1px solid var(--color-border);
		background: var(--color-bg-subtle);
		padding: var(--space-4);
	}

	.sidebar-header {
		margin-bottom: var(--space-6);
	}

	.sidebar-logo {
		font-family: var(--font-mono);
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-fg);
		text-decoration: none;
	}

	.sidebar-logo:hover {
		color: var(--color-accent);
	}

	.sidebar-search {
		margin-top: var(--space-3);
	}

	.docs-content {
		padding: var(--space-8);
		max-width: 900px;
	}
</style>
