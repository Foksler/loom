<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { getApiClient } from '$lib/api/client';
	import type { AnalyticsEvent, AnalyticsPerson, Org } from '$lib/api/types';
	import { TimeRangePicker } from '$lib/components/common';
	import { trackFilterChange, trackLinkClick } from '$lib/analytics';

	const client = getApiClient();

	let orgs = $state<Org[]>([]);
	let selectedOrgId = $state<string | null>(null);
	let events = $state<AnalyticsEvent[]>([]);
	let persons = $state<AnalyticsPerson[]>([]);
	let eventCount = $state(0);
	let personCount = $state(0);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let timeRange = $state('24h');
	let eventNameFilter = $state('');

	$effect(() => {
		loadOrgs();
	});

	$effect(() => {
		if (selectedOrgId) {
			loadAnalytics();
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

	async function loadAnalytics() {
		if (!selectedOrgId) return;
		loading = true;
		error = null;

		try {
			const params: { limit?: number; event_name?: string; start_date?: string } = { limit: 50 };
			if (eventNameFilter) {
				params.event_name = eventNameFilter;
			}

			// Calculate start date based on time range
			const now = new Date();
			let startDate: Date;
			switch (timeRange) {
				case '1h':
					startDate = new Date(now.getTime() - 60 * 60 * 1000);
					break;
				case '24h':
					startDate = new Date(now.getTime() - 24 * 60 * 60 * 1000);
					break;
				case '7d':
					startDate = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
					break;
				case '30d':
					startDate = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
					break;
				default:
					startDate = new Date(now.getTime() - 24 * 60 * 60 * 1000);
			}
			params.start_date = startDate.toISOString();

			const [eventsResponse, countResponse, personsResponse] = await Promise.all([
				client.listAnalyticsEvents(selectedOrgId, params),
				client.countAnalyticsEvents(selectedOrgId, params),
				client.listAnalyticsPersons(selectedOrgId, { limit: 20 }),
			]);

			events = eventsResponse.events;
			eventCount = countResponse.count;
			persons = personsResponse.persons;
			personCount = personsResponse.persons.length;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load analytics';
		} finally {
			loading = false;
		}
	}

	function formatTimestamp(timestamp: string): string {
		return new Date(timestamp).toLocaleString();
	}

	function getEventTypeColor(eventName: string): string {
		if (eventName.startsWith('$')) return 'var(--color-info)';
		if (eventName.includes('click')) return 'var(--color-success)';
		if (eventName.includes('error')) return 'var(--color-error)';
		return 'var(--color-fg-muted)';
	}

	// Group events by name for summary
	const eventSummary = $derived(() => {
		const summary: Record<string, number> = {};
		for (const event of events) {
			summary[event.event_name] = (summary[event.event_name] || 0) + 1;
		}
		return Object.entries(summary)
			.sort((a, b) => b[1] - a[1])
			.slice(0, 10);
	});
</script>

<div class="analytics-page">
	<header class="page-header">
		<div class="header-title">
			<h1>Analytics</h1>
			<p class="subtitle">Product analytics and user behavior tracking</p>
		</div>

		<div class="header-controls">
			{#if orgs.length > 1}
				<select
					class="org-filter"
					bind:value={selectedOrgId}
					onchange={() => {
						trackFilterChange('org', selectedOrgId);
						loadAnalytics();
					}}
				>
					{#each orgs as org}
						<option value={org.id}>{org.name}</option>
					{/each}
				</select>
			{/if}

			<TimeRangePicker
				value={timeRange}
				onchange={(value) => {
					trackFilterChange('time_range', value);
					timeRange = value;
					loadAnalytics();
				}}
			/>
		</div>
	</header>

	{#if error}
		<div class="error-banner">
			<span class="error-icon">!</span>
			<span>{error}</span>
		</div>
	{/if}

	{#if loading}
		<div class="loading">Loading analytics...</div>
	{:else}
		<div class="stats-grid">
			<div class="stat-card">
				<div class="stat-value">{eventCount.toLocaleString()}</div>
				<div class="stat-label">Total Events</div>
			</div>
			<div class="stat-card">
				<div class="stat-value">{personCount.toLocaleString()}</div>
				<div class="stat-label">Unique Users</div>
			</div>
			<div class="stat-card">
				<div class="stat-value">{eventSummary().length}</div>
				<div class="stat-label">Event Types</div>
			</div>
		</div>

		<div class="content-grid">
			<section class="events-section">
				<div class="section-header">
					<h2>Recent Events</h2>
					<input
						type="text"
						class="event-filter"
						placeholder="Filter by event name..."
						bind:value={eventNameFilter}
						onkeyup={(e) => {
							if (e.key === 'Enter') {
								trackFilterChange('event_name', eventNameFilter);
								loadAnalytics();
							}
						}}
					/>
				</div>

				{#if events.length === 0}
					<div class="empty-state">
						<p>No events found for the selected time range.</p>
					</div>
				{:else}
					<div class="events-list">
						{#each events as event}
							<div class="event-row">
								<div class="event-main">
									<span
										class="event-name"
										style="color: {getEventTypeColor(event.event_name)}"
									>
										{event.event_name}
									</span>
									<span class="event-distinct-id">{event.distinct_id}</span>
								</div>
								<div class="event-meta">
									<span class="event-time">{formatTimestamp(event.timestamp)}</span>
								</div>
								{#if event.properties && Object.keys(event.properties).length > 0}
									<div class="event-properties">
										{#each Object.entries(event.properties).slice(0, 3) as [key, value]}
											<span class="property-tag">
												{key}: {typeof value === 'object' ? JSON.stringify(value) : String(value)}
											</span>
										{/each}
										{#if Object.keys(event.properties).length > 3}
											<span class="property-more">+{Object.keys(event.properties).length - 3} more</span>
										{/if}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</section>

			<aside class="sidebar">
				<section class="summary-section">
					<h3>Top Events</h3>
					<div class="event-summary">
						{#each eventSummary() as [name, count]}
							<div class="summary-row">
								<span class="summary-name">{name}</span>
								<span class="summary-count">{count}</span>
							</div>
						{/each}
					</div>
				</section>

				<section class="persons-section">
					<h3>Recent Users</h3>
					<div class="persons-list">
						{#each persons.slice(0, 10) as person}
							<div class="person-row">
								<span class="person-id">
									{person.identities.find((i) => i.identity_type === 'identified')?.distinct_id ||
										person.identities[0]?.distinct_id ||
										person.id.slice(0, 8)}
								</span>
								<span class="person-time">{formatTimestamp(person.created_at)}</span>
							</div>
						{/each}
					</div>
				</section>
			</aside>
		</div>
	{/if}
</div>

<style>
	.analytics-page {
		max-width: 80rem;
		margin: 0 auto;
		padding: var(--space-6);
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: var(--space-6);
		gap: var(--space-4);
		flex-wrap: wrap;
	}

	.header-title h1 {
		font-size: var(--text-2xl);
		font-weight: 600;
		color: var(--color-fg);
		margin: 0;
		font-family: var(--font-mono);
	}

	.subtitle {
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
		margin: var(--space-1) 0 0;
		font-family: var(--font-mono);
	}

	.header-controls {
		display: flex;
		gap: var(--space-3);
		align-items: center;
	}

	.org-filter {
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-bg);
		color: var(--color-fg);
		font-size: var(--text-sm);
		font-family: var(--font-mono);
	}

	.error-banner {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3);
		background: var(--color-error-bg);
		border: 1px solid var(--color-error);
		border-radius: var(--radius-md);
		color: var(--color-error);
		margin-bottom: var(--space-4);
		font-family: var(--font-mono);
	}

	.error-icon {
		font-weight: bold;
	}

	.loading {
		text-align: center;
		padding: var(--space-8);
		color: var(--color-fg-muted);
		font-family: var(--font-mono);
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: var(--space-4);
		margin-bottom: var(--space-6);
	}

	.stat-card {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: var(--space-4);
		text-align: center;
	}

	.stat-value {
		font-size: var(--text-3xl);
		font-weight: 600;
		color: var(--color-fg);
		font-family: var(--font-mono);
	}

	.stat-label {
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
		margin-top: var(--space-1);
		font-family: var(--font-mono);
	}

	.content-grid {
		display: grid;
		grid-template-columns: 1fr 300px;
		gap: var(--space-6);
	}

	@media (max-width: 1024px) {
		.content-grid {
			grid-template-columns: 1fr;
		}
	}

	.events-section {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: var(--space-4);
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-4);
		gap: var(--space-3);
		flex-wrap: wrap;
	}

	.section-header h2 {
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-fg);
		margin: 0;
		font-family: var(--font-mono);
	}

	.event-filter {
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-bg);
		color: var(--color-fg);
		font-size: var(--text-sm);
		font-family: var(--font-mono);
		width: 200px;
	}

	.empty-state {
		text-align: center;
		padding: var(--space-8);
		color: var(--color-fg-muted);
		font-family: var(--font-mono);
	}

	.events-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.event-row {
		padding: var(--space-3);
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
	}

	.event-main {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-1);
	}

	.event-name {
		font-weight: 500;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
	}

	.event-distinct-id {
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		font-family: var(--font-mono);
	}

	.event-meta {
		margin-bottom: var(--space-1);
	}

	.event-time {
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		font-family: var(--font-mono);
	}

	.event-properties {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}

	.property-tag {
		font-size: var(--text-xs);
		background: var(--color-bg-subtle);
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-sm);
		font-family: var(--font-mono);
		color: var(--color-fg-muted);
		max-width: 200px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.property-more {
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		font-family: var(--font-mono);
	}

	.sidebar {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.summary-section,
	.persons-section {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: var(--space-4);
	}

	.sidebar h3 {
		font-size: var(--text-base);
		font-weight: 600;
		color: var(--color-fg);
		margin: 0 0 var(--space-3);
		font-family: var(--font-mono);
	}

	.event-summary,
	.persons-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.summary-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-size: var(--text-sm);
		font-family: var(--font-mono);
	}

	.summary-name {
		color: var(--color-fg);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 180px;
	}

	.summary-count {
		color: var(--color-fg-muted);
		font-weight: 500;
	}

	.person-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-size: var(--text-sm);
		font-family: var(--font-mono);
	}

	.person-id {
		color: var(--color-fg);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 150px;
	}

	.person-time {
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}
</style>
