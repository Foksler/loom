<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { getReposClient, type Repository } from '$lib/api/repos';
	import { getApiClient } from '$lib/api/client';
	import { Card, Badge, Button, Input } from '$lib/ui';

	let repos = $state<Repository[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let searchQuery = $state('');

	const client = getReposClient();
	const apiClient = getApiClient();

	const filteredRepos = $derived(
		repos.filter(
			(r) =>
				r.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
				r.owner_id.toLowerCase().includes(searchQuery.toLowerCase())
		)
	);

	async function loadRepos() {
		loading = true;
		error = null;

		try {
			const user = await apiClient.getCurrentUser();
			const response = await client.listRepos(user.id);
			repos = response.repos;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load repositories';
		} finally {
			loading = false;
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
		});
	}

	$effect(() => {
		loadRepos();
	});
</script>

<svelte:head>
	<title>Repositories - Loom</title>
</svelte:head>

<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
	<div class="flex items-center justify-between mb-6">
		<h1 class="text-2xl font-bold text-fg">Repositories</h1>
		<Button variant="primary">
			<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
			</svg>
			New repository
		</Button>
	</div>

	<div class="mb-6">
		<Input
			type="search"
			placeholder="Find a repository..."
			bind:value={searchQuery}
		/>
	</div>

	{#if loading}
		<div class="space-y-3">
			{#each Array(3) as _}
				<Card>
					<div class="animate-pulse">
						<div class="h-5 bg-bg-muted rounded w-1/3 mb-2"></div>
						<div class="h-4 bg-bg-muted rounded w-1/4"></div>
					</div>
				</Card>
			{/each}
		</div>
	{:else if error}
		<Card>
			<div class="text-center py-8">
				<p class="text-error mb-4">{error}</p>
				<Button variant="secondary" onclick={loadRepos}>
					Try again
				</Button>
			</div>
		</Card>
	{:else if filteredRepos.length === 0}
		<Card>
			<div class="text-center py-8">
				{#if searchQuery}
					<p class="text-fg-muted">No repositories match your search</p>
				{:else}
					<svg class="w-16 h-16 mx-auto text-fg-muted mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
					</svg>
					<p class="text-fg-muted mb-4">You don't have any repositories yet</p>
					<Button variant="primary">
						Create your first repository
					</Button>
				{/if}
			</div>
		</Card>
	{:else}
		<div class="space-y-3">
			{#each filteredRepos as repo (repo.id)}
				<Card>
					<div class="flex items-start justify-between">
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2 mb-1">
								<a
									href="/repos/{repo.owner_id}/{repo.name}"
									class="font-semibold text-accent hover:underline"
								>
									{repo.owner_id}/{repo.name}
								</a>
								<Badge variant={repo.visibility === 'public' ? 'success' : 'muted'} size="sm">
									{repo.visibility}
								</Badge>
							</div>
							<div class="flex items-center gap-4 text-sm text-fg-muted">
								<span>
									Default branch:
									<code class="font-mono text-xs bg-bg-muted px-1 py-0.5 rounded">{repo.default_branch}</code>
								</span>
								<span>Updated {formatDate(repo.updated_at)}</span>
							</div>
						</div>
					</div>
				</Card>
			{/each}
		</div>
	{/if}
</div>
