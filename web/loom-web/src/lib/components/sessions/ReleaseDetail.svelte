<script lang="ts">
	import AdoptionStageBadge from './AdoptionStageBadge.svelte';
	import CrashFreeChart from './CrashFreeChart.svelte';
	import SessionList from './SessionList.svelte';
	import { StatCard, RelativeTime } from '$lib/components/common';

	type AdoptionStage = 'new' | 'low' | 'adopted' | 'replaced';
	type SessionStatus = 'active' | 'exited' | 'crashed' | 'abnormal' | 'errored';

	interface Session {
		id: string;
		status: SessionStatus;
		distinct_id: string;
		release?: string;
		environment: string;
		platform: string;
		crashed: boolean;
		error_count: number;
		duration_ms?: number;
		started_at: string;
		ended_at?: string;
	}

	interface CrashFreeDataPoint {
		timestamp: string;
		crash_free_rate: number;
		total_sessions: number;
		crashed_sessions: number;
	}

	interface ReleaseHealth {
		version: string;
		short_version?: string;
		url?: string;
		crash_free_rate: number;
		crash_free_users: number;
		adoption_percentage: number;
		adoption_stage: AdoptionStage;
		total_sessions: number;
		crashed_sessions: number;
		abnormal_sessions: number;
		errored_sessions: number;
		total_users: number;
		crashed_users: number;
		first_seen?: string;
		last_seen?: string;
		date_released?: string;
	}

	interface Props {
		release: ReleaseHealth;
		crashFreeData: CrashFreeDataPoint[];
		recentSessions: Session[];
		onsessionclick?: (session: Session) => void;
	}

	let { release, crashFreeData, recentSessions, onsessionclick }: Props = $props();

	const crashFreeVariant = $derived(
		release.crash_free_rate >= 99
			? 'success'
			: release.crash_free_rate >= 95
				? 'warning'
				: 'error'
	);

	const healthySessions = $derived(
		release.total_sessions -
			release.crashed_sessions -
			release.abnormal_sessions -
			release.errored_sessions
	);
</script>

<div class="release-detail">
	<header class="release-header">
		<div class="release-title-row">
			<h1 class="release-version">{release.version}</h1>
			<AdoptionStageBadge
				stage={release.adoption_stage}
				percentage={Math.round(release.adoption_percentage)}
			/>
		</div>
		{#if release.short_version}
			<span class="release-short">{release.short_version}</span>
		{/if}
		{#if release.url}
			<a class="release-link" href={release.url} target="_blank" rel="noopener">
				View release notes &rarr;
			</a>
		{/if}
	</header>

	<div class="release-metrics">
		<div class="primary-metrics">
			<div class="metric metric-{crashFreeVariant}">
				<span class="metric-value">{release.crash_free_rate.toFixed(2)}%</span>
				<span class="metric-label">Crash-free sessions</span>
			</div>
			<div class="metric metric-{crashFreeVariant}">
				<span class="metric-value">{release.crash_free_users.toFixed(2)}%</span>
				<span class="metric-label">Crash-free users</span>
			</div>
			<div class="metric">
				<span class="metric-value">{release.adoption_percentage.toFixed(1)}%</span>
				<span class="metric-label">Adoption</span>
			</div>
		</div>
	</div>

	<div class="release-stats">
		<StatCard label="Total Sessions" value={release.total_sessions.toLocaleString()} size="sm" />
		<StatCard label="Healthy" value={healthySessions.toLocaleString()} variant="success" size="sm" />
		<StatCard
			label="Crashed"
			value={release.crashed_sessions.toLocaleString()}
			variant={release.crashed_sessions > 0 ? 'error' : 'default'}
			size="sm"
		/>
		<StatCard
			label="Errored"
			value={release.errored_sessions.toLocaleString()}
			variant={release.errored_sessions > 0 ? 'warning' : 'default'}
			size="sm"
		/>
		<StatCard label="Total Users" value={release.total_users.toLocaleString()} size="sm" />
		<StatCard
			label="Crashed Users"
			value={release.crashed_users.toLocaleString()}
			variant={release.crashed_users > 0 ? 'error' : 'default'}
			size="sm"
		/>
	</div>

	<section class="release-section">
		<h2 class="section-title">Timeline</h2>
		<div class="timeline-info">
			{#if release.date_released}
				<div class="timeline-item">
					<span class="timeline-label">Released</span>
					<RelativeTime timestamp={release.date_released} />
				</div>
			{/if}
			{#if release.first_seen}
				<div class="timeline-item">
					<span class="timeline-label">First session</span>
					<RelativeTime timestamp={release.first_seen} />
				</div>
			{/if}
			{#if release.last_seen}
				<div class="timeline-item">
					<span class="timeline-label">Last session</span>
					<RelativeTime timestamp={release.last_seen} />
				</div>
			{/if}
		</div>
	</section>

	{#if crashFreeData.length > 0}
		<section class="release-section">
			<CrashFreeChart data={crashFreeData} />
		</section>
	{/if}

	{#if recentSessions.length > 0}
		<section class="release-section">
			<h2 class="section-title">Recent Sessions</h2>
			<SessionList sessions={recentSessions} onsessionclick={onsessionclick} />
		</section>
	{/if}
</div>

<style>
	.release-detail {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}

	.release-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.release-title-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.release-version {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xl);
		font-weight: 700;
		color: var(--color-fg);
	}

	.release-short {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.release-link {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-accent);
		text-decoration: none;
	}

	.release-link:hover {
		text-decoration: underline;
	}

	.release-metrics {
		padding: var(--space-4);
		background: var(--color-bg-muted);
		border-radius: var(--radius-md);
	}

	.primary-metrics {
		display: flex;
		gap: var(--space-6);
	}

	.metric {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.metric-value {
		font-family: var(--font-mono);
		font-size: var(--text-2xl);
		font-weight: 700;
		color: var(--color-fg);
	}

	.metric-success .metric-value {
		color: var(--color-success);
	}

	.metric-warning .metric-value {
		color: var(--color-warning);
	}

	.metric-error .metric-value {
		color: var(--color-error);
	}

	.metric-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		text-transform: uppercase;
	}

	.release-stats {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
		gap: var(--space-3);
	}

	.release-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.section-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.timeline-info {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
	}

	.timeline-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.timeline-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}
</style>
