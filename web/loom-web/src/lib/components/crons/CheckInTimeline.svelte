<script lang="ts">
	import CheckInItem from './CheckInItem.svelte';

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
		checkins: CheckIn[];
		loading?: boolean;
		maxItems?: number;
		oncheckinclick?: (checkin: CheckIn) => void;
	}

	let { checkins, loading = false, maxItems = 50, oncheckinclick }: Props = $props();

	let showAll = $state(false);

	const displayedCheckins = $derived(showAll ? checkins : checkins.slice(0, maxItems));
</script>

<div class="checkin-timeline">
	<div class="timeline-header">
		<h3 class="timeline-title">Check-in History</h3>
		<span class="timeline-count">{checkins.length} check-ins</span>
		{#if checkins.length > maxItems}
			<button class="show-more-btn" onclick={() => (showAll = !showAll)}>
				{showAll ? `Show last ${maxItems}` : 'Show all'}
			</button>
		{/if}
	</div>

	{#if loading}
		<div class="timeline-loading">
			<div class="loading-spinner"></div>
			<span>Loading check-ins...</span>
		</div>
	{:else if displayedCheckins.length === 0}
		<div class="timeline-empty">No check-ins recorded yet</div>
	{:else}
		<div class="timeline-items">
			{#each displayedCheckins as checkin (checkin.id)}
				<CheckInItem {checkin} onclick={oncheckinclick} />
			{/each}
		</div>
	{/if}
</div>

<style>
	.checkin-timeline {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.timeline-header {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-subtle);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.timeline-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.timeline-count {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.show-more-btn {
		margin-left: auto;
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: var(--color-bg-muted);
		color: var(--color-fg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
		cursor: pointer;
	}

	.show-more-btn:hover {
		color: var(--color-fg);
	}

	.timeline-items {
		max-height: 400px;
		overflow-y: auto;
	}

	.timeline-loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-3);
		padding: var(--space-6);
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

	.timeline-empty {
		padding: var(--space-6);
		text-align: center;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
