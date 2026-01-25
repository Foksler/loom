<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { getApiClient } from '$lib/api/client';
	import type { ReleaseHealth, CrashProject, Org } from '$lib/api/types';
	import { ReleaseList, ReleaseHealthOverview, CrashFreeChart } from '$lib/components/sessions';
	import { TimeRangePicker } from '$lib/components/common';
	import { trackLinkClick, trackFilterChange } from '$lib/analytics';

	const client = getApiClient();

	let orgs = $state<Org[]>([]);
	let selectedOrgId = $state<string | null>(null);
	let projects = $state<CrashProject[]>([]);
	let selectedProjectId = $state<string | null>(null);
	let releases = $state<ReleaseHealth[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let timeRange = $state('24h');

	$effect(() => {
		loadOrgs();
	});

	$effect(() => {
		if (selectedOrgId) {
			loadProjects();
		}
	});

	$effect(() => {
		if (selectedProjectId) {
			loadReleases();
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
		try {
			const response = await client.listCrashProjects(selectedOrgId);
			projects = response.projects;
			if (projects.length > 0) {
				selectedProjectId = projects[0].id;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load projects';
		}
	}

	async function loadReleases() {
		if (!selectedProjectId) return;
		loading = true;
		error = null;
		try {
			const response = await client.listReleaseHealth(selectedProjectId);
			releases = response.releases;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load release health';
		} finally {
			loading = false;
		}
	}

	function handleReleaseClick(release: ReleaseHealth) {
		trackLinkClick('session_release', `/sessions/releases/${encodeURIComponent(release.release)}`, { release: release.release, crash_free_rate: release.crash_free_session_rate });
		window.location.href = `/sessions/releases/${encodeURIComponent(release.release)}?project_id=${selectedProjectId}`;
	}

	// Compute aggregated overview stats from releases
	function computeOverviewStats(releasesData: ReleaseHealth[]) {
		if (!releasesData.length) {
			return {
				total_sessions: 0,
				total_users: 0,
				crash_free_sessions: 0,
				crash_free_users: 0,
				active_releases: 0,
			};
		}

		const totalSessions = releasesData.reduce((sum, r) => sum + r.total_sessions, 0);
		const totalUsers = releasesData.reduce((sum, r) => sum + r.total_users, 0);

		// Weighted average of crash-free rates
		const weightedCrashFreeSessions =
			totalSessions > 0
				? releasesData.reduce((sum, r) => sum + r.crash_free_session_rate * r.total_sessions, 0) / totalSessions
				: 0;
		const weightedCrashFreeUsers =
			totalUsers > 0
				? releasesData.reduce((sum, r) => sum + r.crash_free_user_rate * r.total_users, 0) / totalUsers
				: 0;

		return {
			total_sessions: totalSessions,
			total_users: totalUsers,
			crash_free_sessions: weightedCrashFreeSessions,
			crash_free_users: weightedCrashFreeUsers,
			active_releases: releasesData.length,
		};
	}

	const overviewStats = $derived(computeOverviewStats(releases));

	// Generate mock crash-free data for the chart
	function computeCrashFreeData(releasesData: ReleaseHealth[]) {
		if (!releasesData.length) return [];
		const data: { timestamp: string; crash_free_rate: number; total_sessions: number; crashed_sessions: number }[] = [];
		const now = new Date();
		for (let i = 23; i >= 0; i--) {
			const timestamp = new Date(now);
			timestamp.setHours(timestamp.getHours() - i);
			data.push({
				timestamp: timestamp.toISOString(),
				crash_free_rate: releasesData[0]?.crash_free_session_rate || 99.5,
				total_sessions: 100,
				crashed_sessions: 1,
			});
		}
		return data;
	}

	const crashFreeData = $derived(computeCrashFreeData(releases));
</script>

<div class="sessions-page">
	<header class="page-header">
		<h1 class="page-title">Release Health</h1>
		<p class="page-description">Monitor crash-free rates and adoption across releases</p>
	</header>

	<div class="selectors">
		{#if orgs.length > 1}
			<div class="selector-group">
				<label class="selector-label" for="org-select">Organization</label>
				<select
					id="org-select"
					class="selector-select"
					value={selectedOrgId}
					onchange={(e) => { trackFilterChange('org', e.currentTarget.value, { page: 'sessions' }); selectedOrgId = e.currentTarget.value; }}
				>
					{#each orgs as org}
						<option value={org.id}>{org.name}</option>
					{/each}
				</select>
			</div>
		{/if}

		{#if projects.length > 0}
			<div class="selector-group">
				<label class="selector-label" for="project-select">Project</label>
				<select
					id="project-select"
					class="selector-select"
					value={selectedProjectId}
					onchange={(e) => { trackFilterChange('project', e.currentTarget.value, { page: 'sessions' }); selectedProjectId = e.currentTarget.value; }}
				>
					{#each projects as project}
						<option value={project.id}>{project.name}</option>
					{/each}
				</select>
			</div>
		{/if}

		<TimeRangePicker value={timeRange} onchange={(value) => { trackFilterChange('time_range', value, { page: 'sessions' }); timeRange = value; }} />
	</div>

	{#if loading}
		<div class="loading">Loading release health...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if releases.length === 0}
		<div class="empty-state">
			<h2>No release data</h2>
			<p>Start tracking sessions to see release health metrics.</p>
		</div>
	{:else}
		<ReleaseHealthOverview stats={overviewStats} />

		{#if crashFreeData.length > 0}
			<CrashFreeChart data={crashFreeData} />
		{/if}

		<section class="releases-section">
			<h2 class="section-title">All Releases</h2>
			<ReleaseList {releases} onreleaseclick={handleReleaseClick} />
		</section>
	{/if}
</div>

<style>
	.sessions-page {
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

	.selectors {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		flex-wrap: wrap;
	}

	.selector-group {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.selector-label {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.selector-select {
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

	.releases-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.section-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-fg);
	}
</style>
