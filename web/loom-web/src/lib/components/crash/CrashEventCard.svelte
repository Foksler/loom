<script lang="ts">
	import { RelativeTime } from '$lib/components/common';

	interface CrashEvent {
		id: string;
		exception_type: string;
		exception_value: string;
		release?: string;
		environment: string;
		platform: string;
		timestamp: string;
		user_context?: {
			id?: string;
			email?: string;
		};
	}

	interface Props {
		event: CrashEvent;
		onclick?: (event: CrashEvent) => void;
	}

	let { event, onclick }: Props = $props();

	function handleClick() {
		onclick?.(event);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			onclick?.(event);
		}
	}
</script>

<div
	class="crash-event-card"
	role="button"
	tabindex="0"
	onclick={handleClick}
	onkeydown={handleKeydown}
>
	<div class="event-header">
		<span class="event-type">{event.exception_type}</span>
		<RelativeTime timestamp={event.timestamp} />
	</div>
	<p class="event-message">{event.exception_value}</p>
	<div class="event-meta">
		<span class="meta-item">
			<span class="meta-label">Platform</span>
			<span class="meta-value">{event.platform}</span>
		</span>
		<span class="meta-item">
			<span class="meta-label">Env</span>
			<span class="meta-value">{event.environment}</span>
		</span>
		{#if event.release}
			<span class="meta-item">
				<span class="meta-label">Release</span>
				<span class="meta-value">{event.release}</span>
			</span>
		{/if}
		{#if event.user_context?.email || event.user_context?.id}
			<span class="meta-item">
				<span class="meta-label">User</span>
				<span class="meta-value">{event.user_context?.email ?? event.user_context?.id}</span>
			</span>
		{/if}
	</div>
</div>

<style>
	.crash-event-card {
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			border-color 0.15s ease;
	}

	.crash-event-card:hover {
		background: var(--color-bg-subtle);
		border-color: var(--color-border);
	}

	.crash-event-card:focus {
		outline: 2px solid var(--color-accent);
		outline-offset: 2px;
	}

	.event-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-2);
	}

	.event-type {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-error);
	}

	.event-message {
		margin: 0 0 var(--space-3) 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.event-meta {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}

	.meta-item {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}

	.meta-label {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
		text-transform: uppercase;
	}

	.meta-value {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg);
	}
</style>
