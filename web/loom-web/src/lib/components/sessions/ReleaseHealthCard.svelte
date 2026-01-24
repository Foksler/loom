<script lang="ts">
	import { StatCard, Sparkline } from '$lib/components/common';
	import AdoptionStageBadge from './AdoptionStageBadge.svelte';

	type AdoptionStage = 'new' | 'low' | 'adopted' | 'replaced';

	interface ReleaseHealth {
		version: string;
		crash_free_rate: number;
		crash_free_users: number;
		adoption_percentage: number;
		adoption_stage: AdoptionStage;
		total_sessions: number;
		crashed_sessions: number;
		total_users: number;
		crashed_users: number;
		first_seen?: string;
		last_seen?: string;
	}

	interface Props {
		release: ReleaseHealth;
		sparklineData?: number[];
		onclick?: (release: ReleaseHealth) => void;
	}

	let { release, sparklineData, onclick }: Props = $props();

	const crashFreeVariant = $derived(
		release.crash_free_rate >= 99
			? 'success'
			: release.crash_free_rate >= 95
				? 'warning'
				: 'error'
	);

	function handleClick() {
		onclick?.(release);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			onclick?.(release);
		}
	}
</script>

<div
	class="release-health-card"
	role="button"
	tabindex="0"
	onclick={handleClick}
	onkeydown={handleKeydown}
>
	<div class="card-header">
		<span class="release-version">{release.version}</span>
		<AdoptionStageBadge stage={release.adoption_stage} percentage={Math.round(release.adoption_percentage)} size="sm" />
	</div>

	<div class="card-metrics">
		<div class="metric metric-{crashFreeVariant}">
			<span class="metric-value">{release.crash_free_rate.toFixed(1)}%</span>
			<span class="metric-label">Crash-free sessions</span>
		</div>
		<div class="metric">
			<span class="metric-value">{release.crash_free_users.toFixed(1)}%</span>
			<span class="metric-label">Crash-free users</span>
		</div>
	</div>

	<div class="card-stats">
		<div class="stat">
			<span class="stat-value">{release.total_sessions.toLocaleString()}</span>
			<span class="stat-label">sessions</span>
		</div>
		<div class="stat">
			<span class="stat-value">{release.total_users.toLocaleString()}</span>
			<span class="stat-label">users</span>
		</div>
		<div class="stat stat-error">
			<span class="stat-value">{release.crashed_sessions.toLocaleString()}</span>
			<span class="stat-label">crashes</span>
		</div>
	</div>

	{#if sparklineData && sparklineData.length > 0}
		<div class="card-sparkline">
			<Sparkline data={sparklineData} width={200} height={32} color={crashFreeVariant} filled />
		</div>
	{/if}
</div>

<style>
	.release-health-card {
		padding: var(--space-4);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			border-color 0.15s ease;
	}

	.release-health-card:hover {
		background: var(--color-bg-subtle);
		border-color: var(--color-border);
	}

	.release-health-card:focus {
		outline: 2px solid var(--color-accent);
		outline-offset: 2px;
	}

	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-3);
	}

	.release-version {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.card-metrics {
		display: flex;
		gap: var(--space-4);
		margin-bottom: var(--space-3);
	}

	.metric {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.metric-value {
		font-family: var(--font-mono);
		font-size: var(--text-xl);
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
		font-size: 10px;
		color: var(--color-fg-muted);
		text-transform: uppercase;
	}

	.card-stats {
		display: flex;
		gap: var(--space-4);
		padding-top: var(--space-3);
		border-top: 1px solid var(--color-border-muted);
	}

	.stat {
		display: flex;
		align-items: baseline;
		gap: var(--space-1);
	}

	.stat-value {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.stat-error .stat-value {
		color: var(--color-error);
	}

	.stat-label {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
	}

	.card-sparkline {
		margin-top: var(--space-3);
		padding-top: var(--space-3);
		border-top: 1px solid var(--color-border-muted);
	}
</style>
