<script lang="ts">
	import { RelativeTime } from '$lib/components/common';

	type SessionStatus = 'active' | 'exited' | 'crashed' | 'abnormal' | 'errored';

	interface Session {
		id: string;
		status: SessionStatus;
		distinct_id: string;
		release?: string;
		environment: string;
		platform: string;
		crashed: boolean;
		error_count: number;
		duration_ms?: number;
		started_at: string;
		ended_at?: string;
	}

	interface Props {
		sessions: Session[];
		loading?: boolean;
		onsessionclick?: (session: Session) => void;
	}

	let { sessions, loading = false, onsessionclick }: Props = $props();

	const statusConfig: Record<SessionStatus, { label: string; variant: string }> = {
		active: { label: 'Active', variant: 'info' },
		exited: { label: 'Exited', variant: 'success' },
		crashed: { label: 'Crashed', variant: 'error' },
		abnormal: { label: 'Abnormal', variant: 'warning' },
		errored: { label: 'Errored', variant: 'warning' },
	};

	function formatDuration(ms?: number): string {
		if (ms == null) return '-';
		if (ms < 1000) return `${ms}ms`;
		if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
		return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
	}
</script>

<div class="session-list">
	<div class="session-list-header">
		<div class="session-count">
			{sessions.length}
			{sessions.length === 1 ? 'session' : 'sessions'}
		</div>
	</div>

	{#if loading}
		<div class="session-list-loading">
			<div class="loading-spinner"></div>
			<span>Loading sessions...</span>
		</div>
	{:else if sessions.length === 0}
		<div class="session-list-empty">
			<span class="empty-text">No sessions found</span>
		</div>
	{:else}
		<table class="session-table">
			<thead>
				<tr>
					<th>Status</th>
					<th>User</th>
					<th>Release</th>
					<th>Duration</th>
					<th>Errors</th>
					<th>Started</th>
				</tr>
			</thead>
			<tbody>
				{#each sessions as session (session.id)}
					<tr
						class="session-row"
						onclick={() => onsessionclick?.(session)}
						onkeydown={(e) => {
							if (e.key === 'Enter') onsessionclick?.(session);
						}}
						tabindex="0"
						role="button"
					>
						<td>
							<span class="status-badge status-{statusConfig[session.status].variant}">
								{statusConfig[session.status].label}
							</span>
						</td>
						<td class="user-cell">{session.distinct_id.substring(0, 8)}...</td>
						<td class="release-cell">{session.release ?? '-'}</td>
						<td class="duration-cell">{formatDuration(session.duration_ms)}</td>
						<td class="errors-cell" class:has-errors={session.error_count > 0}>
							{session.error_count}
						</td>
						<td class="time-cell">
							<RelativeTime timestamp={session.started_at} />
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</div>

<style>
	.session-list {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.session-list-header {
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-subtle);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.session-count {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.session-table {
		width: 100%;
		border-collapse: collapse;
	}

	th {
		padding: var(--space-2) var(--space-3);
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		color: var(--color-fg-muted);
		text-transform: uppercase;
		text-align: left;
		background: var(--color-bg-subtle);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.session-row {
		cursor: pointer;
		transition: background-color 0.15s ease;
	}

	.session-row:hover {
		background: var(--color-bg-subtle);
	}

	.session-row:focus {
		outline: 2px solid var(--color-accent);
		outline-offset: -2px;
	}

	td {
		padding: var(--space-2) var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.status-badge {
		padding: 2px var(--space-1);
		font-size: 10px;
		font-weight: 500;
		border-radius: 2px;
		text-transform: uppercase;
	}

	.status-info {
		background: var(--color-info-soft);
		color: var(--color-info);
	}

	.status-success {
		background: var(--color-success-soft);
		color: var(--color-success);
	}

	.status-error {
		background: var(--color-error-soft);
		color: var(--color-error);
	}

	.status-warning {
		background: var(--color-warning-soft);
		color: var(--color-warning);
	}

	.user-cell {
		color: var(--color-fg-muted);
	}

	.release-cell {
		color: var(--color-fg-muted);
	}

	.duration-cell {
		color: var(--color-fg-muted);
	}

	.errors-cell {
		color: var(--color-fg-muted);
	}

	.errors-cell.has-errors {
		color: var(--color-error);
		font-weight: 600;
	}

	.time-cell {
		color: var(--color-fg-muted);
	}

	.session-list-loading {
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

	.session-list-empty {
		padding: var(--space-8);
		text-align: center;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
