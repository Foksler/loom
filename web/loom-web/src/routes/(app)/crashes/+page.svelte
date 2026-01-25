<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { getApiClient } from '$lib/api/client';
	import type { CrashProject, Org } from '$lib/api/types';
	import { Card } from '$lib/ui';
	import { trackLinkClick, trackFilterChange } from '$lib/analytics';

	const client = getApiClient();

	let orgs = $state<Org[]>([]);
	let selectedOrgId = $state<string | null>(null);
	let projects = $state<CrashProject[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	$effect(() => {
		loadOrgs();
	});

	$effect(() => {
		if (selectedOrgId) {
			loadProjects();
		}
	});

	async function loadOrgs() {
		try {
			const response = await client.listOrgs();
			orgs = response.orgs;
			if (orgs.length > 0) {
				selectedOrgId = orgs[0].id;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load organizations';
		}
	}

	async function loadProjects() {
		if (!selectedOrgId) return;
		loading = true;
		error = null;
		try {
			const response = await client.listCrashProjects(selectedOrgId);
			projects = response.projects;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load projects';
		} finally {
			loading = false;
		}
	}
</script>

<div class="crashes-page">
	<header class="page-header">
		<h1 class="page-title">Crash Analytics</h1>
		<p class="page-description">Track and resolve application crashes</p>
	</header>

	{#if orgs.length > 1}
		<div class="org-selector">
			<label class="org-label" for="org-select">Organization</label>
			<select
				id="org-select"
				class="org-select"
				value={selectedOrgId}
				onchange={(e) => { trackFilterChange('org', e.currentTarget.value, { page: 'crashes' }); selectedOrgId = e.currentTarget.value; }}
			>
				{#each orgs as org}
					<option value={org.id}>{org.name}</option>
				{/each}
			</select>
		</div>
	{/if}

	{#if loading}
		<div class="loading">Loading projects...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if projects.length === 0}
		<Card>
			<div class="empty-state">
				<h2>No crash projects</h2>
				<p>Create a project and integrate the crash SDK to start tracking errors.</p>
			</div>
		</Card>
	{:else}
		<div class="projects-grid">
			{#each projects as project}
				<a href="/crashes/{project.id}" class="project-card" onclick={() => trackLinkClick('crash_project', `/crashes/${project.id}`, { project_id: project.id, project_name: project.name })}>
					<Card>
						<div class="project-content">
							<h3 class="project-name">{project.name}</h3>
							<span class="project-slug">{project.slug}</span>
							<span class="project-platform">{project.platform}</span>
						</div>
					</Card>
				</a>
			{/each}
		</div>
	{/if}
</div>

<style>
	.crashes-page {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		max-width: 1200px;
		margin: 0 auto;
		padding: var(--space-6);
	}

	.page-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.page-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xl);
		font-weight: 700;
		color: var(--color-fg);
	}

	.page-description {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.org-selector {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.org-label {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.org-select {
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
	}

	.loading,
	.error {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		text-align: center;
		padding: var(--space-8);
	}

	.error {
		color: var(--color-error);
	}

	.empty-state {
		text-align: center;
		padding: var(--space-8);
	}

	.empty-state h2 {
		margin: 0 0 var(--space-2) 0;
		font-family: var(--font-mono);
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-fg);
	}

	.empty-state p {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.projects-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: var(--space-4);
	}

	.project-card {
		text-decoration: none;
		color: inherit;
	}

	.project-card:hover :global(.card) {
		border-color: var(--color-accent);
	}

	.project-content {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.project-name {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-base);
		font-weight: 600;
		color: var(--color-fg);
	}

	.project-slug {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.project-platform {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-accent);
		text-transform: uppercase;
	}
</style>
