<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { browser } from '$app/environment';
	import { onDestroy } from 'svelte';
	import { getApiClient } from '$lib/api/client';
	import type { Monitor, Org } from '$lib/api/types';
	import { MonitorList, MonitorHealthBadge } from '$lib/components/crons';
	import { Button } from '$lib/ui';
	import { CronsSSEClient, type CronEvent } from '$lib/realtime';
	import { showNotification } from '$lib/components/notifications';
	import { trackLinkClick, trackFilterChange, trackButtonClick } from '$lib/analytics';

	const client = getApiClient();

	let orgs = $state<Org[]>([]);
	let selectedOrgId = $state<string | null>(null);
	let monitors = $state<Monitor[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let healthFilter = $state<string>('all');
	let sseConnected = $state(false);

	// SSE client managed outside of $state to avoid reactivity issues
	let currentSseClient: CronsSSEClient | null = null;
	let sseEventCleanup: (() => void) | null = null;
	let sseStatusCleanup: (() => void) | null = null;

	// Load orgs on mount (runs once)
	$effect(() => {
		loadOrgs();
	});

	// Handle org selection changes
	$effect(() => {
		const orgId = selectedOrgId;
		if (orgId) {
			loadMonitors();
			connectSSE(orgId);
		}
	});

	// Cleanup SSE on component destroy
	onDestroy(() => {
		disconnectSSE();
	});

	function disconnectSSE() {
		if (sseEventCleanup) {
			sseEventCleanup();
			sseEventCleanup = null;
		}
		if (sseStatusCleanup) {
			sseStatusCleanup();
			sseStatusCleanup = null;
		}
		if (currentSseClient) {
			currentSseClient.disconnect();
			currentSseClient = null;
		}
		sseConnected = false;
	}

	function connectSSE(orgId: string) {
		if (!browser) return;

		// Disconnect existing connection first
		disconnectSSE();

		// Create new SSE client
		const serverUrl = window.location.origin;
		currentSseClient = new CronsSSEClient(serverUrl, orgId);

		// Handle events - store cleanup function
		sseEventCleanup = currentSseClient.onEvent((event: CronEvent) => {
			handleSSEEvent(event);
		});

		// Handle status changes - store cleanup function
		sseStatusCleanup = currentSseClient.onStatus((status) => {
			sseConnected = status === 'connected';
		});

		// Connect
		currentSseClient.connect();
	}

	function handleSSEEvent(event: CronEvent) {
		switch (event.type) {
			case 'init':
				// Update monitors from init event
				monitors = event.monitors.map((m) => ({
					id: m.id,
					slug: m.slug,
					name: m.name,
					status: m.status,
					health: m.health,
					last_checkin_at: m.last_checkin_at,
					next_expected_at: m.next_expected_at,
					consecutive_failures: 0, // Not provided in SSE init
				}));
				break;

			case 'checkin_ok':
				// Update monitor health to healthy
				monitors = monitors.map((m) =>
					m.slug === event.monitor_slug ? { ...m, health: 'healthy' as const } : m
				);
				break;

			case 'checkin_error':
				// Update monitor health to failing
				monitors = monitors.map((m) =>
					m.slug === event.monitor_slug ? { ...m, health: 'failing' as const } : m
				);
				showNotification({
					type: 'error',
					title: 'Check-in Failed',
					message: `Monitor "${event.monitor_slug}" reported an error`,
					link: `/crons/${event.monitor_slug}`,
				});
				break;

			case 'monitor_missed':
				// Update monitor health to missed
				monitors = monitors.map((m) =>
					m.slug === event.monitor_slug ? { ...m, health: 'missed' as const } : m
				);
				showNotification({
					type: 'warning',
					title: 'Monitor Missed',
					message: `Monitor "${event.monitor_slug}" missed its expected check-in`,
					link: `/crons/${event.monitor_slug}`,
				});
				break;

			case 'monitor_created':
				// Reload monitors to get the new one
				loadMonitors();
				break;

			case 'monitor_deleted':
				// Remove the deleted monitor
				monitors = monitors.filter((m) => m.slug !== event.monitor_slug);
				break;

			case 'monitor_updated':
				// Reload monitors to get the updated data
				loadMonitors();
				break;
		}
	}

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

	const filteredMonitors = $derived(
		healthFilter === 'all' ? monitors : monitors.filter((m) => m.health === healthFilter)
	);

	function handleMonitorClick(monitor: Monitor) {
		trackLinkClick('cron_monitor', `/crons/${monitor.slug}`, { monitor_slug: monitor.slug, monitor_health: monitor.health });
		window.location.href = `/crons/${monitor.slug}?org_id=${selectedOrgId}`;
	}
</script>

<div class="crons-page">
	<header class="page-header">
		<div class="header-content">
			<div>
				<h1 class="page-title">
					Cron Monitors
					{#if sseConnected}
						<span class="live-badge" title="Real-time updates active">LIVE</span>
					{/if}
				</h1>
				<p class="page-description">Monitor scheduled jobs and receive alerts for failures</p>
			</div>
			<Button href="/crons/new" onclick={() => trackButtonClick('new_monitor')}>New Monitor</Button>
		</div>
	</header>

	{#if orgs.length > 1}
		<div class="org-selector">
			<label class="org-label" for="org-select">Organization</label>
			<select
				id="org-select"
				class="org-select"
				value={selectedOrgId}
				onchange={(e) => { trackFilterChange('org', e.currentTarget.value, { page: 'crons' }); selectedOrgId = e.currentTarget.value; }}
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
				onchange={(e) => { trackFilterChange('health', e.currentTarget.value, { page: 'crons' }); healthFilter = e.currentTarget.value; }}
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
			<Button href="/crons/new" onclick={() => trackButtonClick('create_monitor_empty_state')}>Create Monitor</Button>
		</div>
	{:else}
		<MonitorList monitors={filteredMonitors} onmonitorclick={handleMonitorClick} />
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
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xl);
		font-weight: 700;
		color: var(--color-fg);
	}

	.live-badge {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-1) var(--space-2);
		font-size: var(--text-xs);
		font-weight: 500;
		color: var(--color-success);
		background: color-mix(in srgb, var(--color-success) 15%, transparent);
		border-radius: var(--radius-sm);
		animation: pulse 2s infinite;
	}

	.live-badge::before {
		content: '';
		display: inline-block;
		width: 6px;
		height: 6px;
		background: var(--color-success);
		border-radius: 50%;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.7;
		}
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
