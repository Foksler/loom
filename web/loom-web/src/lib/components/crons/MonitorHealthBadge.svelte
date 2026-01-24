<script lang="ts">
	type MonitorHealth = 'healthy' | 'failing' | 'missed' | 'timeout' | 'unknown';

	interface Props {
		health: MonitorHealth;
		size?: 'sm' | 'md';
		showIcon?: boolean;
	}

	let { health, size = 'md', showIcon = true }: Props = $props();

	const healthConfig: Record<MonitorHealth, { label: string; variant: string; icon: string }> = {
		healthy: { label: 'Healthy', variant: 'success', icon: '\u2713' },
		failing: { label: 'Failing', variant: 'error', icon: '\u2716' },
		missed: { label: 'Missed', variant: 'warning', icon: '\u26A0' },
		timeout: { label: 'Timeout', variant: 'error', icon: '\u23F1' },
		unknown: { label: 'Unknown', variant: 'muted', icon: '?' },
	};

	const config = $derived(healthConfig[health]);
</script>

<span class="monitor-health monitor-health-{config.variant} monitor-health-{size}">
	{#if showIcon}
		<span class="health-icon">{config.icon}</span>
	{/if}
	{config.label}
</span>

<style>
	.monitor-health {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		font-family: var(--font-mono);
		font-weight: 500;
		border-radius: var(--radius-sm);
	}

	.monitor-health-sm {
		padding: 2px var(--space-1);
		font-size: 10px;
	}

	.monitor-health-md {
		padding: var(--space-1) var(--space-2);
		font-size: var(--text-xs);
	}

	.health-icon {
		font-size: 0.9em;
	}

	.monitor-health-success {
		background: var(--color-success-soft);
		color: var(--color-success);
	}

	.monitor-health-error {
		background: var(--color-error-soft);
		color: var(--color-error);
	}

	.monitor-health-warning {
		background: var(--color-warning-soft);
		color: var(--color-warning);
	}

	.monitor-health-muted {
		background: var(--color-bg-subtle);
		color: var(--color-fg-muted);
	}
</style>
