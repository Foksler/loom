<script lang="ts">
	import MonitorStatusBadge from './MonitorStatusBadge.svelte';
	import MonitorHealthBadge from './MonitorHealthBadge.svelte';
	import { RelativeTime, StatCard } from '$lib/components/common';

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
		timezone: string;
		checkin_margin_minutes: number;
		max_runtime_minutes?: number;
		ping_key: string;
		environments: string[];
		last_checkin_at?: string;
		next_expected_at?: string;
		consecutive_failures: number;
		total_checkins: number;
		total_failures: number;
		created_at: string;
	}

	interface Props {
		monitor: Monitor;
		recentCheckins?: { id: string }[];
		onpause?: () => void;
		onresume?: () => void;
		onedit?: () => void;
		ondelete?: () => void;
	}

	let {
		monitor,
		recentCheckins = [],
		onpause,
		onresume,
		onedit,
		ondelete,
	}: Props = $props();

	const scheduleDisplay = $derived(
		monitor.schedule_type === 'cron'
			? monitor.schedule_value
			: `Every ${monitor.schedule_value} minutes`
	);

	const successRate = $derived(
		monitor.total_checkins > 0
			? ((monitor.total_checkins - monitor.total_failures) / monitor.total_checkins) * 100
			: 0
	);
</script>

<div class="monitor-detail">
	<header class="monitor-header">
		<div class="monitor-title-row">
			<h1 class="monitor-name">{monitor.name}</h1>
			<MonitorStatusBadge status={monitor.status} />
			<MonitorHealthBadge health={monitor.health} />
		</div>
		<span class="monitor-slug">{monitor.slug}</span>
		{#if monitor.description}
			<p class="monitor-description">{monitor.description}</p>
		{/if}
	</header>

	<div class="monitor-actions">
		{#if monitor.status === 'active'}
			<button class="action-btn" onclick={onpause}>Pause</button>
		{:else if monitor.status === 'paused'}
			<button class="action-btn action-primary" onclick={onresume}>Resume</button>
		{/if}
		<button class="action-btn" onclick={onedit}>Edit</button>
		<button class="action-btn action-danger" onclick={ondelete}>Delete</button>
	</div>

	<div class="monitor-stats">
		<StatCard
			label="Total Check-ins"
			value={monitor.total_checkins.toLocaleString()}
			size="sm"
		/>
		<StatCard
			label="Success Rate"
			value="{successRate.toFixed(1)}%"
			variant={successRate >= 99 ? 'success' : successRate >= 95 ? 'warning' : 'error'}
			size="sm"
		/>
		<StatCard
			label="Consecutive Failures"
			value={monitor.consecutive_failures.toString()}
			variant={monitor.consecutive_failures > 0 ? 'error' : 'default'}
			size="sm"
		/>
	</div>

	<section class="monitor-section">
		<h2 class="section-title">Schedule</h2>
		<dl class="schedule-info">
			<div class="schedule-item">
				<dt>Expression</dt>
				<dd><code>{scheduleDisplay}</code></dd>
			</div>
			<div class="schedule-item">
				<dt>Timezone</dt>
				<dd>{monitor.timezone}</dd>
			</div>
			<div class="schedule-item">
				<dt>Grace Period</dt>
				<dd>{monitor.checkin_margin_minutes} min</dd>
			</div>
			{#if monitor.max_runtime_minutes}
				<div class="schedule-item">
					<dt>Max Runtime</dt>
					<dd>{monitor.max_runtime_minutes} min</dd>
				</div>
			{/if}
		</dl>
	</section>

	<section class="monitor-section">
		<h2 class="section-title">Timing</h2>
		<div class="timing-info">
			{#if monitor.last_checkin_at}
				<div class="timing-item">
					<span class="timing-label">Last check-in</span>
					<RelativeTime timestamp={monitor.last_checkin_at} />
				</div>
			{/if}
			{#if monitor.next_expected_at}
				<div class="timing-item">
					<span class="timing-label">Next expected</span>
					<RelativeTime timestamp={monitor.next_expected_at} />
				</div>
			{/if}
		</div>
	</section>

	{#if monitor.environments.length > 0}
		<section class="monitor-section">
			<h2 class="section-title">Environments</h2>
			<div class="environments-list">
				{#each monitor.environments as env}
					<span class="environment-badge">{env}</span>
				{/each}
			</div>
		</section>
	{/if}
</div>

<style>
	.monitor-detail {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}

	.monitor-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.monitor-title-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
	}

	.monitor-name {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-xl);
		font-weight: 600;
		color: var(--color-fg);
	}

	.monitor-slug {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.monitor-description {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.monitor-actions {
		display: flex;
		gap: var(--space-2);
	}

	.action-btn {
		padding: var(--space-2) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		background: var(--color-bg-muted);
		color: var(--color-fg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: all 0.15s ease;
	}

	.action-btn:hover {
		background: var(--color-bg-subtle);
	}

	.action-primary {
		background: var(--color-accent);
		color: var(--color-bg);
		border-color: var(--color-accent);
	}

	.action-primary:hover {
		opacity: 0.9;
	}

	.action-danger {
		color: var(--color-error);
	}

	.action-danger:hover {
		background: var(--color-error-soft);
	}

	.monitor-stats {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: var(--space-3);
	}

	.monitor-section {
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

	.schedule-info {
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
	}

	.schedule-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.schedule-item dt {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.schedule-item dd {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
	}

	.schedule-item code {
		padding: var(--space-1) var(--space-2);
		background: var(--color-bg-muted);
		border-radius: var(--radius-sm);
	}

	.timing-info {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
	}

	.timing-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.timing-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.environments-list {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}

	.environment-badge {
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: var(--color-bg-muted);
		color: var(--color-fg);
		border-radius: var(--radius-sm);
	}
</style>
