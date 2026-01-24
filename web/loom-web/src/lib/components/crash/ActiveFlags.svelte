<script lang="ts">
	interface Props {
		flags: Record<string, string>;
		title?: string;
	}

	let { flags, title = 'Active Feature Flags' }: Props = $props();

	const flagEntries = $derived(Object.entries(flags));
</script>

<div class="active-flags">
	<h3 class="flags-title">{title}</h3>

	{#if flagEntries.length === 0}
		<div class="flags-empty">No feature flags were active</div>
	{:else}
		<ul class="flags-list">
			{#each flagEntries as [key, value] (key)}
				<li class="flag-item">
					<span class="flag-key">{key}</span>
					<span class="flag-value">{value}</span>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.active-flags {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.flags-title {
		margin: 0;
		padding: var(--space-3) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
		background: var(--color-bg-subtle);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.flags-list {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.flag-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid var(--color-border-muted);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}

	.flag-item:last-child {
		border-bottom: none;
	}

	.flag-key {
		color: var(--color-fg);
	}

	.flag-value {
		padding: 2px var(--space-2);
		background: var(--color-accent-soft);
		color: var(--color-accent);
		border-radius: var(--radius-sm);
	}

	.flags-empty {
		padding: var(--space-4);
		text-align: center;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
