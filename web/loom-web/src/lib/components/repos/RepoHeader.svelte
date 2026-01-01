<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { Repository } from '$lib/api/repos';
	import { Badge, Button } from '$lib/ui';

	interface Props {
		repo: Repository;
	}

	let { repo }: Props = $props();

	let showCloneUrl = $state(false);
	let copied = $state(false);

	async function copyCloneUrl() {
		await navigator.clipboard.writeText(repo.clone_url);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}
</script>

<div class="flex items-start justify-between gap-4 mb-6">
	<div class="flex items-center gap-3">
		<svg class="w-8 h-8 text-fg-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
		</svg>
		<div>
			<h1 class="text-xl font-bold text-fg flex items-center gap-2">
				<a href="/repos/{repo.owner_id}" class="hover:text-accent">{repo.owner_id}</a>
				<span class="text-fg-muted">/</span>
				<span>{repo.name}</span>
			</h1>
			<div class="flex items-center gap-2 mt-1">
				<Badge variant={repo.visibility === 'public' ? 'success' : 'muted'} size="sm">
					{repo.visibility}
				</Badge>
				<span class="text-sm text-fg-muted">
					Default branch: <code class="font-mono text-xs bg-bg-muted px-1 py-0.5 rounded">{repo.default_branch}</code>
				</span>
			</div>
		</div>
	</div>

	<div class="relative">
		<Button variant="secondary" size="sm" onclick={() => (showCloneUrl = !showCloneUrl)}>
			<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
			</svg>
			Clone
		</Button>

		{#if showCloneUrl}
			<div class="absolute right-0 mt-2 p-3 bg-bg border border-border rounded-lg shadow-lg z-10 w-80">
				<div class="text-sm font-medium text-fg mb-2">Clone with HTTPS</div>
				<div class="flex gap-2">
					<input
						type="text"
						readonly
						value={repo.clone_url}
						class="flex-1 text-sm font-mono bg-bg-muted border border-border rounded px-2 py-1"
					/>
					<Button variant="secondary" size="sm" onclick={copyCloneUrl}>
						{copied ? 'Copied!' : 'Copy'}
					</Button>
				</div>
			</div>
		{/if}
	</div>
</div>
