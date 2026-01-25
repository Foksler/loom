<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { getApiClient } from '$lib/api/client';
	import type { Issue, CrashEvent } from '$lib/api/types';
	import { IssueDetail, CrashEventCard } from '$lib/components/crash';
	import { trackLinkClick, trackAction } from '$lib/analytics';

	const client = getApiClient();
	const projectId = $derived($page.params.projectId);
	const issueId = $derived($page.params.issueId);

	let issue = $state<Issue | null>(null);
	let events = $state<CrashEvent[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	$effect(() => {
		if (projectId && issueId) {
			loadData();
		}
	});

	async function loadData() {
		loading = true;
		error = null;
		try {
			const [issueRes, eventsRes] = await Promise.all([
				client.getIssue(projectId, issueId),
				client.listCrashEvents(projectId, issueId, { limit: 10 }),
			]);
			issue = issueRes;
			events = eventsRes.events;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load issue';
		} finally {
			loading = false;
		}
	}

	async function handleResolve() {
		if (!issue) return;
		trackAction('resolve', 'issue', issueId);
		try {
			issue = await client.resolveIssue(projectId, issueId);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to resolve issue';
		}
	}

	async function handleUnresolve() {
		if (!issue) return;
		trackAction('unresolve', 'issue', issueId);
		try {
			issue = await client.unresolveIssue(projectId, issueId);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to unresolve issue';
		}
	}

	async function handleIgnore() {
		if (!issue) return;
		trackAction('ignore', 'issue', issueId);
		try {
			issue = await client.ignoreIssue(projectId, issueId);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to ignore issue';
		}
	}

	function handleEventClick(event: CrashEvent) {
		trackLinkClick('crash_event', `event_${event.id}`, { event_id: event.id });
		// Could navigate to event detail or show in modal
		console.log('Event clicked:', event.id);
	}
</script>

<div class="issue-detail-page">
	<header class="page-header">
		<a href="/crashes/{projectId}" class="back-link" onclick={() => trackLinkClick('back_to_issues', `/crashes/${projectId}`)}>&larr; Issues</a>
	</header>

	{#if loading}
		<div class="loading">Loading issue...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if issue}
		<IssueDetail
			{issue}
			{events}
			onresolve={handleResolve}
			onunresolve={handleUnresolve}
			onignore={handleIgnore}
		/>

		{#if events.length > 0}
			<section class="events-section">
				<h2 class="section-title">Recent Events</h2>
				<div class="events-list">
					{#each events as event}
						<CrashEventCard {event} onclick={() => handleEventClick(event)} />
					{/each}
				</div>
			</section>
		{/if}
	{/if}
</div>

<style>
	.issue-detail-page {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		max-width: 1200px;
		margin: 0 auto;
		padding: var(--space-6);
	}

	.page-header {
		display: flex;
		align-items: center;
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

	.events-section {
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

	.events-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
</style>
