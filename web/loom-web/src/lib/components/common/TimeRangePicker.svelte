<script lang="ts">
	interface TimeRange {
		label: string;
		value: string;
		duration: number;
	}

	interface Props {
		value: string;
		onchange?: (value: string) => void;
		ranges?: TimeRange[];
	}

	const defaultRanges: TimeRange[] = [
		{ label: '1h', value: '1h', duration: 3600000 },
		{ label: '6h', value: '6h', duration: 21600000 },
		{ label: '24h', value: '24h', duration: 86400000 },
		{ label: '7d', value: '7d', duration: 604800000 },
		{ label: '14d', value: '14d', duration: 1209600000 },
		{ label: '30d', value: '30d', duration: 2592000000 },
	];

	let { value, onchange, ranges = defaultRanges }: Props = $props();

	function selectRange(rangeValue: string) {
		if (onchange) {
			onchange(rangeValue);
		}
	}
</script>

<div class="time-range-picker">
	{#each ranges as range}
		<button
			type="button"
			class="range-button"
			class:range-active={value === range.value}
			onclick={() => selectRange(range.value)}
		>
			{range.label}
		</button>
	{/each}
</div>

<style>
	.time-range-picker {
		display: inline-flex;
		gap: 1px;
		background: var(--color-border-muted);
		border-radius: var(--radius-md);
		padding: 1px;
	}

	.range-button {
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 500;
		background: var(--color-bg-muted);
		color: var(--color-fg-muted);
		border: none;
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			color 0.15s ease;
	}

	.range-button:first-child {
		border-radius: var(--radius-sm) 0 0 var(--radius-sm);
	}

	.range-button:last-child {
		border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
	}

	.range-button:hover:not(.range-active) {
		background: var(--color-bg-subtle);
		color: var(--color-fg);
	}

	.range-active {
		background: var(--color-accent-soft);
		color: var(--color-accent);
	}
</style>
