<script lang="ts">
	import IssueListItem from './IssueListItem.svelte';

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
		issues: Issue[];
		loading?: boolean;
		statusFilter?: IssueStatus | 'all';
		onstatuschange?: (status: IssueStatus | 'all') => void;
		onissueclick?: (issue: Issue) => void;
	}

	let {
		issues,
		loading = false,
		statusFilter = 'all',
		onstatuschange,
		onissueclick,
	}: Props = $props();

	const statusOptions: { value: IssueStatus | 'all'; label: string }[] = [
		{ value: 'all', label: 'All' },
		{ value: 'unresolved', label: 'Unresolved' },
		{ value: 'resolved', label: 'Resolved' },
		{ value: 'regressed', label: 'Regressed' },
		{ value: 'ignored', label: 'Ignored' },
	];

	function handleStatusChange(event: Event) {
		const target = event.target as HTMLSelectElement;
		onstatuschange?.(target.value as IssueStatus | 'all');
	}
</script>

<div class="issue-list">
	<div class="issue-list-header">
		<div class="issue-count">
			{issues.length}
			{issues.length === 1 ? 'issue' : 'issues'}
		</div>
		<div class="issue-filters">
			<select class="status-filter" value={statusFilter} onchange={handleStatusChange}>
				{#each statusOptions as option}
					<option value={option.value}>{option.label}</option>
				{/each}
			</select>
		</div>
	</div>

	{#if loading}
		<div class="issue-list-loading">
			<div class="loading-spinner"></div>
			<span>Loading issues...</span>
		</div>
	{:else if issues.length === 0}
		<div class="issue-list-empty">
			<span class="empty-icon">&#10003;</span>
			<span class="empty-text">No issues found</span>
		</div>
	{:else}
		<div class="issue-list-items">
			{#each issues as issue (issue.id)}
				<IssueListItem {issue} onclick={onissueclick} />
			{/each}
		</div>
	{/if}
</div>

<style>
	.issue-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.issue-list-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-2) 0;
	}

	.issue-count {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.issue-filters {
		display: flex;
		gap: var(--space-2);
	}

	.status-filter {
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: var(--color-bg-muted);
		color: var(--color-fg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
		cursor: pointer;
	}

	.status-filter:hover {
		border-color: var(--color-border);
	}

	.issue-list-items {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.issue-list-loading {
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

	.issue-list-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		padding: var(--space-8);
		background: var(--color-bg-muted);
		border: 1px dashed var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.empty-icon {
		font-size: var(--text-2xl);
		color: var(--color-success);
	}

	.empty-text {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
