<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		label: string;
		value: string | number;
		trend?: {
			direction: 'up' | 'down' | 'flat';
			value: string;
			positive?: boolean;
		};
		variant?: 'default' | 'success' | 'warning' | 'error';
		size?: 'sm' | 'md' | 'lg';
		icon?: Snippet;
	}

	let { label, value, trend, variant = 'default', size = 'md', icon }: Props = $props();

	const trendIcon = $derived(
		trend?.direction === 'up' ? '\u2191' : trend?.direction === 'down' ? '\u2193' : '\u2192'
	);

	const trendClass = $derived(
		trend
			? trend.positive === true
				? 'trend-positive'
				: trend.positive === false
					? 'trend-negative'
					: 'trend-neutral'
			: ''
	);
</script>

<div class="stat-card stat-card-{variant} stat-card-{size}">
	{#if icon}
		<div class="stat-icon">
			{@render icon()}
		</div>
	{/if}
	<div class="stat-content">
		<span class="stat-label">{label}</span>
		<span class="stat-value">{value}</span>
		{#if trend}
			<span class="stat-trend {trendClass}">
				<span class="trend-icon">{trendIcon}</span>
				{trend.value}
			</span>
		{/if}
	</div>
</div>

<style>
	.stat-card {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		font-family: var(--font-mono);
	}

	.stat-card-sm {
		padding: var(--space-2) var(--space-3);
	}

	.stat-card-md {
		padding: var(--space-3) var(--space-4);
	}

	.stat-card-lg {
		padding: var(--space-4) var(--space-5);
	}

	.stat-card-success {
		border-left: 3px solid var(--color-success);
	}

	.stat-card-warning {
		border-left: 3px solid var(--color-warning);
	}

	.stat-card-error {
		border-left: 3px solid var(--color-error);
	}

	.stat-icon {
		color: var(--color-fg-muted);
		font-size: var(--text-lg);
	}

	.stat-content {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.stat-label {
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.stat-value {
		font-size: var(--text-xl);
		font-weight: 600;
		color: var(--color-fg);
	}

	.stat-card-sm .stat-value {
		font-size: var(--text-lg);
	}

	.stat-card-lg .stat-value {
		font-size: var(--text-2xl);
	}

	.stat-trend {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		font-size: var(--text-xs);
	}

	.trend-icon {
		font-size: var(--text-sm);
	}

	.trend-positive {
		color: var(--color-success);
	}

	.trend-negative {
		color: var(--color-error);
	}

	.trend-neutral {
		color: var(--color-fg-muted);
	}
</style>
