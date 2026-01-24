<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { getApiClient } from '$lib/api/client';
	import type { ReleaseHealth, AppSession } from '$lib/api/types';
	import { ReleaseDetail, CrashFreeChart, SessionList } from '$lib/components/sessions';

	const client = getApiClient();
	const version = $derived(decodeURIComponent($page.params.version));
	const projectId = $derived($page.url.searchParams.get('project_id') || '');

	let release = $state<ReleaseHealth | null>(null);
	let sessions = $state<AppSession[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	$effect(() => {
		if (version && projectId) {
			loadData();
		}
	});

	async function loadData() {
		loading = true;
		error = null;
		try {
			const [releaseRes, sessionsRes] = await Promise.all([
				client.getReleaseHealth(projectId, version),
				client.listAppSessions(projectId, { release: version, limit: 20 }),
			]);
			release = releaseRes;
			sessions = sessionsRes.sessions;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load release data';
		} finally {
			loading = false;
		}
	}

	// Convert ReleaseHealth to the format expected by ReleaseDetail
	const releaseDetailData = $derived(() => {
		if (!release) return null;
		return {
			version: release.release,
			crash_free_rate: release.crash_free_session_rate,
			crash_free_users: release.crash_free_user_rate,
			adoption_percentage: release.adoption_rate,
			adoption_stage: release.adoption_stage as 'new' | 'low' | 'adopted' | 'replaced',
			total_sessions: release.total_sessions,
			crashed_sessions: release.crashed_sessions,
			abnormal_sessions: 0,
			errored_sessions: release.errored_sessions,
			total_users: release.total_users,
			crashed_users: release.crashed_users,
			first_seen: release.first_seen,
			last_seen: release.last_seen,
		};
	});

	// Convert AppSession to the format expected by SessionList
	const sessionListData = $derived(() => {
		return sessions.map((s) => ({
			id: s.id,
			status: s.status as 'active' | 'exited' | 'crashed' | 'abnormal' | 'errored',
			distinct_id: s.distinct_id,
			release: s.release,
			environment: s.environment,
			platform: s.platform,
			crashed: s.crashed,
			error_count: s.error_count,
			duration_ms: s.duration_ms,
			started_at: s.started_at,
			ended_at: s.ended_at,
		}));
	});

	// Generate mock crash-free data
	const crashFreeData = $derived(() => {
		if (!release) return [];
		const data: { timestamp: string; crash_free_rate: number; total_sessions: number; crashed_sessions: number }[] = [];
		const now = new Date();
		for (let i = 23; i >= 0; i--) {
			const timestamp = new Date(now);
			timestamp.setHours(timestamp.getHours() - i);
			data.push({
				timestamp: timestamp.toISOString(),
				crash_free_rate: release.crash_free_session_rate + (Math.random() - 0.5) * 2,
				total_sessions: Math.floor(release.total_sessions / 24),
				crashed_sessions: Math.floor(release.crashed_sessions / 24),
			});
		}
		return data;
	});

	function handleSessionClick(session: typeof sessionListData[0]) {
		console.log('Session clicked:', session.id);
	}
</script>

<div class="release-detail-page">
	<header class="page-header">
		<a href="/sessions" class="back-link">&larr; Release Health</a>
	</header>

	{#if loading}
		<div class="loading">Loading release data...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if releaseDetailData()}
		<ReleaseDetail
			release={releaseDetailData()}
			crashFreeData={crashFreeData()}
			recentSessions={sessionListData()}
			onsessionclick={handleSessionClick}
		/>
	{/if}
</div>

<style>
	.release-detail-page {
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
</style>
