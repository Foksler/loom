<script lang="ts">
	import IssueStatusBadge from './IssueStatusBadge.svelte';
	import { RelativeTime } from '$lib/components/common';

	type IssueStatus = 'unresolved' | 'resolved' | 'ignored' | 'regressed';

	interface Issue {
		id: string;
		short_id: string;
		title: string;
		culprit?: string;
		status: IssueStatus;
		event_count: number;
		user_count: number;
		first_seen: string;
		last_seen: string;
	}

	interface Props {
		issue: Issue;
		onclick?: (issue: Issue) => void;
	}

	let { issue, onclick }: Props = $props();

	function handleClick() {
		onclick?.(issue);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			onclick?.(issue);
		}
	}
</script>

<div
	class="issue-list-item"
	role="button"
	tabindex="0"
	onclick={handleClick}
	onkeydown={handleKeydown}
>
	<div class="issue-main">
		<div class="issue-header">
			<span class="issue-id">{issue.short_id}</span>
			<IssueStatusBadge status={issue.status} size="sm" />
		</div>
		<h4 class="issue-title">{issue.title}</h4>
		{#if issue.culprit}
			<span class="issue-culprit">{issue.culprit}</span>
		{/if}
	</div>
	<div class="issue-stats">
		<div class="stat">
			<span class="stat-value">{issue.event_count.toLocaleString()}</span>
			<span class="stat-label">events</span>
		</div>
		<div class="stat">
			<span class="stat-value">{issue.user_count.toLocaleString()}</span>
			<span class="stat-label">users</span>
		</div>
	</div>
	<div class="issue-time">
		<RelativeTime timestamp={issue.last_seen} />
	</div>
</div>

<style>
	.issue-list-item {
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

	.issue-list-item:hover {
		background: var(--color-bg-subtle);
		border-color: var(--color-border);
	}

	.issue-list-item:focus {
		outline: 2px solid var(--color-accent);
		outline-offset: 2px;
	}

	.issue-main {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 0;
	}

	.issue-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.issue-id {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.issue-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-fg);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.issue-culprit {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.issue-stats {
		display: flex;
		gap: var(--space-4);
	}

	.stat {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
	}

	.stat-value {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.stat-label {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
		text-transform: uppercase;
	}

	.issue-time {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		white-space: nowrap;
	}
</style>
