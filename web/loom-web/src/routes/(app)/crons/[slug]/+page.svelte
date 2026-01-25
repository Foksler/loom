<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { getApiClient } from '$lib/api/client';
	import type { Monitor, CheckIn } from '$lib/api/types';
	import { MonitorDetail, CheckInTimeline, PingUrlDisplay, UptimeChart } from '$lib/components/crons';
	import { Button } from '$lib/ui';
	import { trackLinkClick, trackButtonClick, trackAction } from '$lib/analytics';

	const client = getApiClient();
	const slug = $derived($page.params.slug);
	const orgId = $derived($page.url.searchParams.get('org_id') || '');

	let monitor = $state<Monitor | null>(null);
	let checkins = $state<CheckIn[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	$effect(() => {
		if (slug && orgId) {
			loadData();
		}
	});

	async function loadData() {
		loading = true;
		error = null;
		try {
			const [monitorRes, checkinsRes] = await Promise.all([
				client.getMonitor(orgId, slug),
				client.listCheckIns(orgId, slug, { limit: 50 }),
			]);
			monitor = monitorRes;
			checkins = checkinsRes.checkins;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load monitor';
		} finally {
			loading = false;
		}
	}

	async function handlePause() {
		if (!monitor) return;
		trackAction('pause', 'monitor', slug);
		try {
			monitor = await client.pauseMonitor(orgId, slug);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to pause monitor';
		}
	}

	async function handleResume() {
		if (!monitor) return;
		trackAction('resume', 'monitor', slug);
		try {
			monitor = await client.resumeMonitor(orgId, slug);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to resume monitor';
		}
	}

	async function handleDelete() {
		if (!monitor || !confirm('Are you sure you want to delete this monitor?')) return;
		trackAction('delete', 'monitor', slug);
		try {
			await client.deleteMonitor(orgId, slug);
			window.location.href = '/crons';
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to delete monitor';
		}
	}

	// Generate uptime data for the chart
	function computeUptimeData(checkinsData: CheckIn[]) {
		if (!checkinsData.length) return [];
		const data: { date: string; status: 'ok' | 'error' | 'missed' | 'none' }[] = [];
		const now = new Date();
		for (let i = 29; i >= 0; i--) {
			const date = new Date(now);
			date.setDate(date.getDate() - i);
			const dateStr = date.toISOString().split('T')[0];
			const dayCheckins = checkinsData.filter((c) => c.created_at.startsWith(dateStr));
			if (dayCheckins.length === 0) {
				data.push({ date: dateStr, status: 'none' });
			} else if (dayCheckins.some((c) => c.status === 'error')) {
				data.push({ date: dateStr, status: 'error' });
			} else {
				data.push({ date: dateStr, status: 'ok' });
			}
		}
		return data;
	}

	const uptimeData = $derived(computeUptimeData(checkins));
</script>

<div class="monitor-detail-page">
	<header class="page-header">
		<a href="/crons" class="back-link" onclick={() => trackLinkClick('back_to_monitors', '/crons')}>&larr; Monitors</a>
	</header>

	{#if loading}
		<div class="loading">Loading monitor...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if monitor}
		<MonitorDetail
			{monitor}
			recentCheckins={checkins.slice(0, 10)}
			onpause={handlePause}
			onresume={handleResume}
			ondelete={handleDelete}
		/>

		<section class="detail-section">
			<h2 class="section-title">Ping URLs</h2>
			<PingUrlDisplay pingKey={monitor.ping_key} baseUrl={window.location.origin} />
		</section>

		{#if uptimeData.length > 0}
			<section class="detail-section">
				<h2 class="section-title">Uptime (Last 30 Days)</h2>
				<UptimeChart data={uptimeData} />
			</section>
		{/if}

		{#if checkins.length > 0}
			<section class="detail-section">
				<h2 class="section-title">Check-in History</h2>
				<CheckInTimeline {checkins} />
			</section>
		{/if}
	{/if}
</div>

<style>
	.monitor-detail-page {
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

	.detail-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.section-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-fg);
	}
</style>
