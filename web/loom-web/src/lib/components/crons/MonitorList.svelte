<script lang="ts">
	import MonitorListItem from './MonitorListItem.svelte';

	type MonitorStatus = 'active' | 'paused' | 'disabled';
	type MonitorHealth = 'healthy' | 'failing' | 'missed' | 'timeout' | 'unknown';

	interface Monitor {
		id: string;
		slug: string;
		name: string;
		description?: string;
		status: MonitorStatus;
		health: MonitorHealth;
		schedule_type: 'cron' | 'interval';
		schedule_value: string;
		last_checkin_at?: string;
		next_expected_at?: string;
		total_checkins: number;
	}

	interface Props {
		monitors: Monitor[];
		loading?: boolean;
		healthFilter?: MonitorHealth | 'all';
		onhealthchange?: (health: MonitorHealth | 'all') => void;
		onmonitorclick?: (monitor: Monitor) => void;
	}

	let {
		monitors,
		loading = false,
		healthFilter = 'all',
		onhealthchange,
		onmonitorclick,
	}: Props = $props();

	const healthOptions: { value: MonitorHealth | 'all'; label: string }[] = [
		{ value: 'all', label: 'All' },
		{ value: 'healthy', label: 'Healthy' },
		{ value: 'failing', label: 'Failing' },
		{ value: 'missed', label: 'Missed' },
		{ value: 'timeout', label: 'Timeout' },
		{ value: 'unknown', label: 'Unknown' },
	];

	function handleHealthChange(event: Event) {
		const target = event.target as HTMLSelectElement;
		onhealthchange?.(target.value as MonitorHealth | 'all');
	}
</script>

<div class="monitor-list">
	<div class="monitor-list-header">
		<div class="monitor-count">
			{monitors.length}
			{monitors.length === 1 ? 'monitor' : 'monitors'}
		</div>
		<div class="monitor-filters">
			<select class="health-filter" value={healthFilter} onchange={handleHealthChange}>
				{#each healthOptions as option}
					<option value={option.value}>{option.label}</option>
				{/each}
			</select>
		</div>
	</div>

	{#if loading}
		<div class="monitor-list-loading">
			<div class="loading-spinner"></div>
			<span>Loading monitors...</span>
		</div>
	{:else if monitors.length === 0}
		<div class="monitor-list-empty">
			<span class="empty-icon">\u23F0</span>
			<span class="empty-text">No monitors found</span>
		</div>
	{:else}
		<div class="monitor-list-items">
			{#each monitors as monitor (monitor.id)}
				<MonitorListItem {monitor} onclick={onmonitorclick} />
			{/each}
		</div>
	{/if}
</div>

<style>
	.monitor-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.monitor-list-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-2) 0;
	}

	.monitor-count {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.monitor-filters {
		display: flex;
		gap: var(--space-2);
	}

	.health-filter {
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: var(--color-bg-muted);
		color: var(--color-fg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
		cursor: pointer;
	}

	.health-filter:hover {
		border-color: var(--color-border);
	}

	.monitor-list-items {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.monitor-list-loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-3);
		padding: var(--space-8);
		color: var(--color-fg-muted);
	}

	.loading-spinner {
		width: 24px;
		height: 24px;
		border: 2px solid var(--color-border-muted);
		border-top-color: var(--color-accent);
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.monitor-list-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		padding: var(--space-8);
		background: var(--color-bg-muted);
		border: 1px dashed var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.empty-icon {
		font-size: var(--text-2xl);
		color: var(--color-fg-muted);
	}

	.empty-text {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
