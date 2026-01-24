<script lang="ts">
	type MonitorStatus = 'active' | 'paused' | 'disabled';

	interface Props {
		status: MonitorStatus;
		size?: 'sm' | 'md';
	}

	let { status, size = 'md' }: Props = $props();

	const statusConfig: Record<MonitorStatus, { label: string; variant: string }> = {
		active: { label: 'Active', variant: 'success' },
		paused: { label: 'Paused', variant: 'warning' },
		disabled: { label: 'Disabled', variant: 'muted' },
	};

	const config = $derived(statusConfig[status]);
</script>

<span class="monitor-status monitor-status-{config.variant} monitor-status-{size}">
	{config.label}
</span>

<style>
	.monitor-status {
		display: inline-flex;
		align-items: center;
		font-family: var(--font-mono);
		font-weight: 500;
		border-radius: var(--radius-sm);
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.monitor-status-sm {
		padding: 2px var(--space-1);
		font-size: 10px;
	}

	.monitor-status-md {
		padding: var(--space-1) var(--space-2);
		font-size: var(--text-xs);
	}

	.monitor-status-success {
		background: var(--color-success-soft);
		color: var(--color-success);
	}

	.monitor-status-warning {
		background: var(--color-warning-soft);
		color: var(--color-warning);
	}

	.monitor-status-muted {
		background: var(--color-bg-subtle);
		color: var(--color-fg-muted);
	}
</style>
