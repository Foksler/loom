<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { getApiClient } from '$lib/api/client';
	import type { Monitor, Org } from '$lib/api/types';
	import { MonitorList, MonitorHealthBadge } from '$lib/components/crons';
	import { Button } from '$lib/ui';

	const client = getApiClient();

	let orgs = $state<Org[]>([]);
	let selectedOrgId = $state<string | null>(null);
	let monitors = $state<Monitor[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let healthFilter = $state<string>('all');

	$effect(() => {
		loadOrgs();
	});

	$effect(() => {
		if (selectedOrgId) {
			loadMonitors();
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

	async function loadMonitors() {
		if (!selectedOrgId) return;
		loading = true;
		error = null;
		try {
			const response = await client.listMonitors(selectedOrgId);
			monitors = response.monitors;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load monitors';
		} finally {
			loading = false;
		}
	}

	const filteredMonitors = $derived(() => {
		if (healthFilter === 'all') return monitors;
		return monitors.filter((m) => m.health === healthFilter);
	});

	function handleMonitorClick(monitor: Monitor) {
		window.location.href = `/crons/${monitor.slug}?org_id=${selectedOrgId}`;
	}
</script>

<div class="crons-page">
	<header class="page-header">
		<div class="header-content">
			<div>
				<h1 class="page-title">Cron Monitors</h1>
				<p class="page-description">Monitor scheduled jobs and receive alerts for failures</p>
			</div>
			<Button href="/crons/new">New Monitor</Button>
		</div>
	</header>

	{#if orgs.length > 1}
		<div class="org-selector">
			<label class="org-label" for="org-select">Organization</label>
			<select
				id="org-select"
				class="org-select"
				value={selectedOrgId}
				onchange={(e) => (selectedOrgId = e.currentTarget.value)}
			>
				{#each orgs as org}
					<option value={org.id}>{org.name}</option>
				{/each}
			</select>
		</div>
	{/if}

	<div class="filters">
		<div class="filter-group">
			<label class="filter-label" for="health-filter">Health</label>
			<select
				id="health-filter"
				class="filter-select"
				value={healthFilter}
				onchange={(e) => (healthFilter = e.currentTarget.value)}
			>
				<option value="all">All</option>
				<option value="healthy">Healthy</option>
				<option value="failing">Failing</option>
				<option value="missed">Missed</option>
				<option value="timeout">Timeout</option>
				<option value="unknown">Unknown</option>
			</select>
		</div>
	</div>

	{#if loading}
		<div class="loading">Loading monitors...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if monitors.length === 0}
		<div class="empty-state">
			<h2>No monitors configured</h2>
			<p>Create a monitor to start tracking your scheduled jobs.</p>
			<Button href="/crons/new">Create Monitor</Button>
		</div>
	{:else}
		<MonitorList monitors={filteredMonitors()} onmonitorclick={handleMonitorClick} />
	{/if}
</div>

<style>
	.crons-page {
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

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
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
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		text-align: center;
		padding: var(--space-8);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.empty-state h2 {
		margin: 0;
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
