<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { Branch } from '$lib/api/repos';

	interface Props {
		branches: Branch[];
		currentRef: string;
		onSelect: (ref: string) => void;
	}

	let { branches, currentRef, onSelect }: Props = $props();

	let open = $state(false);
	let search = $state('');

	const filteredBranches = $derived(
		branches.filter((b) => b.name.toLowerCase().includes(search.toLowerCase()))
	);

	function selectBranch(name: string) {
		onSelect(name);
		open = false;
		search = '';
	}
</script>

<div class="relative">
	<button
		type="button"
		onclick={() => (open = !open)}
		class="flex items-center gap-2 px-3 py-1.5 text-sm font-medium bg-bg-muted border border-border rounded-md hover:bg-bg-subtle"
	>
		<svg class="w-4 h-4 text-fg-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
		</svg>
		<span class="font-mono">{currentRef}</span>
		<svg class="w-4 h-4 text-fg-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
		</svg>
	</button>

	{#if open}
		<div class="absolute left-0 mt-1 w-64 bg-bg border border-border rounded-lg shadow-lg z-20">
			<div class="p-2 border-b border-border">
				<input
					type="text"
					placeholder="Find a branch..."
					bind:value={search}
					class="w-full px-2 py-1.5 text-sm bg-bg-muted border border-border rounded focus:outline-none focus:ring-2 focus:ring-accent"
				/>
			</div>
			<div class="max-h-64 overflow-y-auto py-1">
				{#if filteredBranches.length === 0}
					<div class="px-3 py-2 text-sm text-fg-muted">No branches found</div>
				{:else}
					{#each filteredBranches as branch}
						<button
							type="button"
							onclick={() => selectBranch(branch.name)}
							class="w-full flex items-center gap-2 px-3 py-2 text-sm text-left hover:bg-bg-muted {branch.name === currentRef ? 'bg-accent/10' : ''}"
						>
							<svg class="w-4 h-4 text-fg-muted flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
							</svg>
							<span class="font-mono truncate">{branch.name}</span>
							{#if branch.is_default}
								<span class="ml-auto text-xs text-fg-muted bg-bg-subtle px-1.5 py-0.5 rounded">default</span>
							{/if}
							{#if branch.name === currentRef}
								<svg class="w-4 h-4 text-accent ml-auto flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
									<path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
								</svg>
							{/if}
						</button>
					{/each}
				{/if}
			</div>
		</div>
	{/if}
</div>
