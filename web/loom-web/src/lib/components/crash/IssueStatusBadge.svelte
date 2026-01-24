<script lang="ts">
	type IssueStatus = 'unresolved' | 'resolved' | 'ignored' | 'regressed';

	interface Props {
		status: IssueStatus;
		size?: 'sm' | 'md';
	}

	let { status, size = 'md' }: Props = $props();

	const statusConfig: Record<IssueStatus, { label: string; variant: string }> = {
		unresolved: { label: 'Unresolved', variant: 'error' },
		resolved: { label: 'Resolved', variant: 'success' },
		ignored: { label: 'Ignored', variant: 'muted' },
		regressed: { label: 'Regressed', variant: 'warning' },
	};

	const config = $derived(statusConfig[status]);
</script>

<span class="issue-status issue-status-{config.variant} issue-status-{size}">
	{config.label}
</span>

<style>
	.issue-status {
		display: inline-flex;
		align-items: center;
		font-family: var(--font-mono);
		font-weight: 500;
		border-radius: var(--radius-sm);
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.issue-status-sm {
		padding: 2px var(--space-1);
		font-size: 10px;
	}

	.issue-status-md {
		padding: var(--space-1) var(--space-2);
		font-size: var(--text-xs);
	}

	.issue-status-error {
		background: var(--color-error-soft);
		color: var(--color-error);
	}

	.issue-status-success {
		background: var(--color-success-soft);
		color: var(--color-success);
	}

	.issue-status-warning {
		background: var(--color-warning-soft);
		color: var(--color-warning);
	}

	.issue-status-muted {
		background: var(--color-bg-subtle);
		color: var(--color-fg-muted);
	}
</style>
