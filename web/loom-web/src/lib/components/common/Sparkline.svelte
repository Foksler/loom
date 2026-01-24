<script lang="ts">
	interface Props {
		data: number[];
		width?: number;
		height?: number;
		color?: 'accent' | 'success' | 'warning' | 'error' | 'muted';
		filled?: boolean;
	}

	let { data, width = 80, height = 24, color = 'accent', filled = false }: Props = $props();

	const points = $derived(() => {
		if (data.length === 0) return '';

		const min = Math.min(...data);
		const max = Math.max(...data);
		const range = max - min || 1;
		const padding = 2;
		const chartWidth = width - padding * 2;
		const chartHeight = height - padding * 2;
		const stepX = chartWidth / Math.max(1, data.length - 1);

		return data
			.map((value, index) => {
				const x = padding + index * stepX;
				const y = padding + chartHeight - ((value - min) / range) * chartHeight;
				return `${x},${y}`;
			})
			.join(' ');
	});

	const fillPath = $derived(() => {
		if (data.length === 0) return '';

		const min = Math.min(...data);
		const max = Math.max(...data);
		const range = max - min || 1;
		const padding = 2;
		const chartWidth = width - padding * 2;
		const chartHeight = height - padding * 2;
		const stepX = chartWidth / Math.max(1, data.length - 1);

		const linePoints = data.map((value, index) => {
			const x = padding + index * stepX;
			const y = padding + chartHeight - ((value - min) / range) * chartHeight;
			return `${x},${y}`;
		});

		const startX = padding;
		const endX = padding + (data.length - 1) * stepX;
		const bottomY = padding + chartHeight;

		return `M${startX},${bottomY} L${linePoints.join(' L')} L${endX},${bottomY} Z`;
	});
</script>

<svg class="sparkline sparkline-{color}" {width} {height} viewBox="0 0 {width} {height}">
	{#if filled && data.length > 0}
		<path d={fillPath()} class="sparkline-fill" />
	{/if}
	{#if data.length > 0}
		<polyline points={points()} class="sparkline-line" fill="none" />
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
