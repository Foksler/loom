<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { getApiClient } from '$lib/api/client';
	import type { Issue, CrashProject } from '$lib/api/types';
	import { IssueList, IssueStatusBadge } from '$lib/components/crash';
	import { TimeRangePicker } from '$lib/components/common';
	import { trackLinkClick, trackFilterChange } from '$lib/analytics';

	const client = getApiClient();
	const projectId = $derived($page.params.projectId);

	let project = $state<CrashProject | null>(null);
	let issues = $state<Issue[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let statusFilter = $state<string>('all');
	let timeRange = $state('24h');

	$effect(() => {
		if (projectId) {
			loadData();
		}
	});

	async function loadData() {
		loading = true;
		error = null;
		try {
			const [projectRes, issuesRes] = await Promise.all([
				client.getCrashProject(projectId),
				client.listIssues(projectId, {
					status: statusFilter === 'all' ? undefined : statusFilter,
					limit: 50,
				}),
			]);
			project = projectRes;
			issues = issuesRes.issues;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load data';
		} finally {
			loading = false;
		}
	}

	function handleIssueClick(issue: Issue) {
		trackLinkClick('crash_issue', `/crashes/${projectId}/issues/${issue.id}`, { issue_id: issue.id, issue_status: issue.status });
		window.location.href = `/crashes/${projectId}/issues/${issue.id}`;
	}

	function handleStatusChange(e: Event) {
		const value = (e.target as HTMLSelectElement).value;
		trackFilterChange('status', value, { page: 'crash_project' });
		statusFilter = value;
		loadData();
	}
</script>

<div class="issues-page">
	<header class="page-header">
		<div class="header-top">
			<a href="/crashes" class="back-link" onclick={() => trackLinkClick('back_to_projects', '/crashes')}>&larr; Projects</a>
		</div>
		{#if project}
			<h1 class="page-title">{project.name}</h1>
			<p class="page-description">{project.slug} - {project.platform}</p>
		{/if}
	</header>

	<div class="filters">
		<div class="filter-group">
			<label class="filter-label" for="status-filter">Status</label>
			<select
				id="status-filter"
				class="filter-select"
				value={statusFilter}
				onchange={handleStatusChange}
			>
				<option value="all">All</option>
				<option value="unresolved">Unresolved</option>
				<option value="resolved">Resolved</option>
				<option value="ignored">Ignored</option>
				<option value="regressed">Regressed</option>
			</select>
		</div>
		<TimeRangePicker
			value={timeRange}
			onchange={(value) => { trackFilterChange('time_range', value, { page: 'crash_project' }); timeRange = value; }}
		/>
	</div>

	{#if loading}
		<div class="loading">Loading issues...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if issues.length === 0}
		<div class="empty-state">
			<h2>No issues found</h2>
			<p>No crashes have been reported yet.</p>
		</div>
	{:else}
		<IssueList {issues} onissueclick={handleIssueClick} />
	{/if}
</div>

<style>
	.issues-page {
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

	.header-top {
		margin-bottom: var(--space-2);
	}

	.back-link {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-accent);
		text-decoration: none;
	}

	.back-link:hover {
		text-decoration: underline;
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

	.filters {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		flex-wrap: wrap;
	}

	.filter-group {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.filter-label {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.filter-select {
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
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
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
</style>
