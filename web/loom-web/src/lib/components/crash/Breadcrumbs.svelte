<script lang="ts">
	import { RelativeTime } from '$lib/components/common';

	type BreadcrumbLevel = 'debug' | 'info' | 'warning' | 'error';

	interface Breadcrumb {
		timestamp: string;
		category: string;
		message?: string;
		level: BreadcrumbLevel;
		data?: Record<string, unknown>;
	}

	interface Props {
		breadcrumbs: Breadcrumb[];
		maxItems?: number;
	}

	let { breadcrumbs, maxItems = 50 }: Props = $props();

	let showAll = $state(false);

	const displayedBreadcrumbs = $derived(
		showAll ? breadcrumbs : breadcrumbs.slice(-maxItems)
	);

	const levelIcons: Record<BreadcrumbLevel, string> = {
		debug: '\u2022',
		info: '\u2139',
		warning: '\u26A0',
		error: '\u2716',
	};
</script>

<div class="breadcrumbs">
	<div class="breadcrumbs-header">
		<h3 class="breadcrumbs-title">Breadcrumbs</h3>
		<span class="breadcrumbs-count">{breadcrumbs.length} events</span>
		{#if breadcrumbs.length > maxItems}
			<button class="show-more-btn" onclick={() => (showAll = !showAll)}>
				{showAll ? `Show last ${maxItems}` : 'Show all'}
			</button>
		{/if}
	</div>

	{#if displayedBreadcrumbs.length === 0}
		<div class="breadcrumbs-empty">No breadcrumbs recorded</div>
	{:else}
		<ul class="breadcrumbs-list">
			{#each displayedBreadcrumbs as crumb, index (index)}
				<li class="breadcrumb breadcrumb-{crumb.level}">
					<span class="breadcrumb-icon">{levelIcons[crumb.level]}</span>
					<span class="breadcrumb-category">{crumb.category}</span>
					{#if crumb.message}
						<span class="breadcrumb-message">{crumb.message}</span>
					{/if}
					<RelativeTime timestamp={crumb.timestamp} />
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.breadcrumbs {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.breadcrumbs-header {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-subtle);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.breadcrumbs-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.breadcrumbs-count {
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

	.breadcrumbs-list {
		list-style: none;
		padding: 0;
		margin: 0;
		max-height: 400px;
		overflow-y: auto;
	}

	.breadcrumb {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid var(--color-border-muted);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}

	.breadcrumb:last-child {
		border-bottom: none;
	}

	.breadcrumb-icon {
		width: 16px;
		text-align: center;
	}

	.breadcrumb-debug .breadcrumb-icon {
		color: var(--color-fg-muted);
	}

	.breadcrumb-info .breadcrumb-icon {
		color: var(--color-info);
	}

	.breadcrumb-warning .breadcrumb-icon {
		color: var(--color-warning);
	}

	.breadcrumb-error .breadcrumb-icon {
		color: var(--color-error);
	}

	.breadcrumb-category {
		padding: 1px var(--space-1);
		background: var(--color-bg-subtle);
		border-radius: 2px;
		color: var(--color-fg-muted);
	}

	.breadcrumb-message {
		flex: 1;
		color: var(--color-fg);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.breadcrumbs-empty {
		padding: var(--space-6);
		text-align: center;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
