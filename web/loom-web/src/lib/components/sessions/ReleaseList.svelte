<script lang="ts">
	import ReleaseListItem from './ReleaseListItem.svelte';

	type AdoptionStage = 'new' | 'low' | 'adopted' | 'replaced';

	interface ReleaseHealth {
		version: string;
		crash_free_rate: number;
		crash_free_users: number;
		adoption_percentage: number;
		adoption_stage: AdoptionStage;
		total_sessions: number;
		crashed_sessions: number;
		total_users: number;
		crashed_users: number;
		first_seen?: string;
		last_seen?: string;
	}

	interface Props {
		releases: ReleaseHealth[];
		loading?: boolean;
		onreleaseclick?: (release: ReleaseHealth) => void;
	}

	let { releases, loading = false, onreleaseclick }: Props = $props();
</script>

<div class="release-list">
	<div class="release-list-header">
		<div class="release-count">
			{releases.length}
			{releases.length === 1 ? 'release' : 'releases'}
		</div>
	</div>

	{#if loading}
		<div class="release-list-loading">
			<div class="loading-spinner"></div>
			<span>Loading releases...</span>
		</div>
	{:else if releases.length === 0}
		<div class="release-list-empty">
			<span class="empty-icon">&#x1F4E6;</span>
			<span class="empty-text">No releases found</span>
		</div>
	{:else}
		<div class="release-list-items">
			{#each releases as release (release.version)}
				<ReleaseListItem {release} onclick={onreleaseclick} />
			{/each}
		</div>
	{/if}
</div>

<style>
	.release-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.release-list-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-2) 0;
	}

	.release-count {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.release-list-items {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.release-list-loading {
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

	.release-list-empty {
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
	}

	.empty-text {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
