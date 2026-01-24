<script lang="ts">
	import { RelativeTime } from '$lib/components/common';

	type CheckInStatus = 'in_progress' | 'ok' | 'error' | 'missed' | 'timeout';

	interface CheckIn {
		id: string;
		status: CheckInStatus;
		started_at?: string;
		finished_at?: string;
		duration_ms?: number;
		environment?: string;
		release?: string;
		exit_code?: number;
		output?: string;
	}

	interface Props {
		checkin: CheckIn;
		onclick?: (checkin: CheckIn) => void;
	}

	let { checkin, onclick }: Props = $props();

	const statusConfig: Record<CheckInStatus, { label: string; variant: string; icon: string }> = {
		in_progress: { label: 'In Progress', variant: 'info', icon: '\u27F3' },
		ok: { label: 'OK', variant: 'success', icon: '\u2713' },
		error: { label: 'Error', variant: 'error', icon: '\u2716' },
		missed: { label: 'Missed', variant: 'warning', icon: '\u26A0' },
		timeout: { label: 'Timeout', variant: 'error', icon: '\u23F1' },
	};

	const config = $derived(statusConfig[checkin.status]);

	const durationDisplay = $derived(
		checkin.duration_ms != null
			? checkin.duration_ms < 1000
				? `${checkin.duration_ms}ms`
				: `${(checkin.duration_ms / 1000).toFixed(1)}s`
			: null
	);

	function handleClick() {
		onclick?.(checkin);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			onclick?.(checkin);
		}
	}
</script>

<div
	class="checkin-item checkin-item-{config.variant}"
	role="button"
	tabindex="0"
	onclick={handleClick}
	onkeydown={handleKeydown}
>
	<span class="checkin-icon">{config.icon}</span>
	<span class="checkin-status">{config.label}</span>
	{#if durationDisplay}
		<span class="checkin-duration">{durationDisplay}</span>
	{/if}
	{#if checkin.environment}
		<span class="checkin-env">{checkin.environment}</span>
	{/if}
	{#if checkin.finished_at}
		<RelativeTime timestamp={checkin.finished_at} />
	{:else if checkin.started_at}
		<span class="checkin-running">
			Started <RelativeTime timestamp={checkin.started_at} />
		</span>
	{/if}
</div>

<style>
	.checkin-item {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-3);
		border-left: 3px solid var(--color-border-muted);
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			border-color 0.15s ease;
	}

	.checkin-item:hover {
		background: var(--color-bg-subtle);
	}

	.checkin-item-success {
		border-left-color: var(--color-success);
	}

	.checkin-item-error {
		border-left-color: var(--color-error);
	}

	.checkin-item-warning {
		border-left-color: var(--color-warning);
	}

	.checkin-item-info {
		border-left-color: var(--color-info);
	}

	.checkin-icon {
		font-size: var(--text-sm);
	}

	.checkin-item-success .checkin-icon {
		color: var(--color-success);
	}

	.checkin-item-error .checkin-icon {
		color: var(--color-error);
	}

	.checkin-item-warning .checkin-icon {
		color: var(--color-warning);
	}

	.checkin-item-info .checkin-icon {
		color: var(--color-info);
	}

	.checkin-status {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-fg);
	}

	.checkin-duration {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		padding: 2px var(--space-1);
		background: var(--color-bg-subtle);
		border-radius: 2px;
	}

	.checkin-env {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		padding: 2px var(--space-1);
		background: var(--color-bg-subtle);
		border-radius: 2px;
	}

	.checkin-running {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-info);
	}
</style>
