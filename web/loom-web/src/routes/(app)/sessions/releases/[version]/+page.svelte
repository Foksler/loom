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
	type ReleaseDetailFormat = {
		version: string;
		crash_free_rate: number;
		crash_free_users: number;
		adoption_percentage: number;
		adoption_stage: 'new' | 'low' | 'adopted' | 'replaced';
		total_sessions: number;
		crashed_sessions: number;
		abnormal_sessions: number;
		errored_sessions: number;
		total_users: number;
		crashed_users: number;
		first_seen: string;
		last_seen: string;
	};

	function computeReleaseDetailData(releaseData: ReleaseHealth | null): ReleaseDetailFormat | null {
		if (!releaseData) return null;
		return {
			version: releaseData.release,
			crash_free_rate: releaseData.crash_free_session_rate,
			crash_free_users: releaseData.crash_free_user_rate,
			adoption_percentage: releaseData.adoption_rate,
			adoption_stage: releaseData.adoption_stage as 'new' | 'low' | 'adopted' | 'replaced',
			total_sessions: releaseData.total_sessions,
			crashed_sessions: releaseData.crashed_sessions,
			abnormal_sessions: 0,
			errored_sessions: releaseData.errored_sessions,
			total_users: releaseData.total_users,
			crashed_users: releaseData.crashed_users,
			first_seen: releaseData.first_seen,
			last_seen: releaseData.last_seen,
		};
	}

	const releaseDetailData = $derived(computeReleaseDetailData(release));

	// Convert AppSession to the format expected by SessionList
	type SessionListFormat = {
		id: string;
		status: 'active' | 'exited' | 'crashed' | 'abnormal' | 'errored';
		distinct_id: string;
		release: string;
		environment: string;
		platform: string;
		crashed: boolean;
		error_count: number;
		duration_ms: number | null;
		started_at: string;
		ended_at: string | null;
	};

	function computeSessionListData(sessionsData: AppSession[]): SessionListFormat[] {
		return sessionsData.map((s) => ({
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
	}

	const sessionListData = $derived(computeSessionListData(sessions));

	// Generate mock crash-free data
	function computeCrashFreeData(releaseData: ReleaseHealth | null) {
		if (!releaseData) return [];
		const data: { timestamp: string; crash_free_rate: number; total_sessions: number; crashed_sessions: number }[] = [];
		const now = new Date();
		for (let i = 23; i >= 0; i--) {
			const timestamp = new Date(now);
			timestamp.setHours(timestamp.getHours() - i);
			data.push({
				timestamp: timestamp.toISOString(),
				crash_free_rate: releaseData.crash_free_session_rate + (Math.random() - 0.5) * 2,
				total_sessions: Math.floor(releaseData.total_sessions / 24),
				crashed_sessions: Math.floor(releaseData.crashed_sessions / 24),
			});
		}
		return data;
	}

	const crashFreeData = $derived(computeCrashFreeData(release));

	function handleSessionClick(session: SessionListFormat) {
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
	{:else if releaseDetailData}
		<ReleaseDetail
			release={releaseDetailData}
			crashFreeData={crashFreeData}
			recentSessions={sessionListData}
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
