<script lang="ts">
	import MonitorStatusBadge from './MonitorStatusBadge.svelte';
	import MonitorHealthBadge from './MonitorHealthBadge.svelte';
	import { RelativeTime } from '$lib/components/common';

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
		monitor: Monitor;
		onclick?: (monitor: Monitor) => void;
	}

	let { monitor, onclick }: Props = $props();

	const scheduleDisplay = $derived(
		monitor.schedule_type === 'cron' ? monitor.schedule_value : `Every ${monitor.schedule_value} min`
	);

	function handleClick() {
		onclick?.(monitor);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			onclick?.(monitor);
		}
	}
</script>

<div
	class="monitor-list-item"
	role="button"
	tabindex="0"
	onclick={handleClick}
	onkeydown={handleKeydown}
>
	<div class="monitor-main">
		<div class="monitor-header">
			<span class="monitor-name">{monitor.name}</span>
			<MonitorStatusBadge status={monitor.status} size="sm" />
			<MonitorHealthBadge health={monitor.health} size="sm" />
		</div>
		<span class="monitor-slug">{monitor.slug}</span>
		{#if monitor.description}
			<span class="monitor-description">{monitor.description}</span>
		{/if}
	</div>
	<div class="monitor-schedule">
		<span class="schedule-label">Schedule</span>
		<span class="schedule-value">{scheduleDisplay}</span>
	</div>
	<div class="monitor-timing">
		{#if monitor.last_checkin_at}
			<div class="timing-row">
				<span class="timing-label">Last check-in</span>
				<RelativeTime timestamp={monitor.last_checkin_at} />
			</div>
		{/if}
		{#if monitor.next_expected_at}
			<div class="timing-row">
				<span class="timing-label">Next expected</span>
				<RelativeTime timestamp={monitor.next_expected_at} />
			</div>
		{/if}
	</div>
</div>

<style>
	.monitor-list-item {
		display: grid;
		grid-template-columns: 1fr auto auto;
		gap: var(--space-4);
		align-items: center;
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			border-color 0.15s ease;
	}

	.monitor-list-item:hover {
		background: var(--color-bg-subtle);
		border-color: var(--color-border);
	}

	.monitor-list-item:focus {
		outline: 2px solid var(--color-accent);
		outline-offset: 2px;
	}

	.monitor-main {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 0;
	}

	.monitor-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.monitor-name {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.monitor-slug {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.monitor-description {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-subtle);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.monitor-schedule {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
	}

	.schedule-label {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
		text-transform: uppercase;
	}

	.schedule-value {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg);
	}

	.monitor-timing {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 120px;
	}

	.timing-row {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
	}

	.timing-label {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
		text-transform: uppercase;
	}
</style>
