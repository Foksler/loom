<script lang="ts">
	interface Props {
		data: number[];
		width?: number;
		height?: number;
		color?: 'accent' | 'success' | 'warning' | 'error' | 'muted';
		filled?: boolean;
	}

	let { data, width = 80, height = 24, color = 'accent', filled = false }: Props = $props();

	function computePoints(inputData: number[], w: number, h: number): string {
		if (inputData.length === 0) return '';

		const min = Math.min(...inputData);
		const max = Math.max(...inputData);
		const range = max - min || 1;
		const padding = 2;
		const chartWidth = w - padding * 2;
		const chartHeight = h - padding * 2;
		const stepX = chartWidth / Math.max(1, inputData.length - 1);

		return inputData
			.map((value, index) => {
				const x = padding + index * stepX;
				const y = padding + chartHeight - ((value - min) / range) * chartHeight;
				return `${x},${y}`;
			})
			.join(' ');
	}

	const points = $derived(computePoints(data, width, height));

	function computeFillPath(inputData: number[], w: number, h: number): string {
		if (inputData.length === 0) return '';

		const min = Math.min(...inputData);
		const max = Math.max(...inputData);
		const range = max - min || 1;
		const padding = 2;
		const chartWidth = w - padding * 2;
		const chartHeight = h - padding * 2;
		const stepX = chartWidth / Math.max(1, inputData.length - 1);

		const linePoints = inputData.map((value, index) => {
			const x = padding + index * stepX;
			const y = padding + chartHeight - ((value - min) / range) * chartHeight;
			return `${x},${y}`;
		});

		const startX = padding;
		const endX = padding + (inputData.length - 1) * stepX;
		const bottomY = padding + chartHeight;

		return `M${startX},${bottomY} L${linePoints.join(' L')} L${endX},${bottomY} Z`;
	}

	const fillPath = $derived(computeFillPath(data, width, height));
</script>

<svg class="sparkline sparkline-{color}" {width} {height} viewBox="0 0 {width} {height}">
	{#if filled && data.length > 0}
		<path d={fillPath} class="sparkline-fill" />
	{/if}
	{#if data.length > 0}
		<polyline points={points} class="sparkline-line" fill="none" />
	{/if}
</svg>

<style>
	.sparkline {
		display: inline-block;
		vertical-align: middle;
	}

	.sparkline-line {
		stroke-width: 1.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.sparkline-fill {
		opacity: 0.15;
	}

	.sparkline-accent .sparkline-line {
		stroke: var(--color-accent);
	}

	.sparkline-accent .sparkline-fill {
		fill: var(--color-accent);
	}

	.sparkline-success .sparkline-line {
		stroke: var(--color-success);
	}

	.sparkline-success .sparkline-fill {
		fill: var(--color-success);
	}

	.sparkline-warning .sparkline-line {
		stroke: var(--color-warning);
	}

	.sparkline-warning .sparkline-fill {
		fill: var(--color-warning);
	}

	.sparkline-error .sparkline-line {
		stroke: var(--color-error);
	}

	.sparkline-error .sparkline-fill {
		fill: var(--color-error);
	}

	.sparkline-muted .sparkline-line {
		stroke: var(--color-fg-muted);
	}

	.sparkline-muted .sparkline-fill {
		fill: var(--color-fg-muted);
	}
</style>
