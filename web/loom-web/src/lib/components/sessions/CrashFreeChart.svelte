<script lang="ts">
	interface DataPoint {
		timestamp: string;
		crash_free_rate: number;
		total_sessions: number;
		crashed_sessions: number;
	}

	interface Props {
		data: DataPoint[];
		height?: number;
		showTarget?: boolean;
		target?: number;
	}

	let { data, height = 200, showTarget = true, target = 99 }: Props = $props();

	type ChartDataResult = { points: string; area: string; min: number; max: number };

	function computeChartData(inputData: DataPoint[], chartHeight: number): ChartDataResult {
		if (inputData.length === 0) return { points: '', area: '', min: 95, max: 100 };

		const rates = inputData.map((d) => d.crash_free_rate);
		const minRate = Math.min(...rates);
		const maxRate = Math.max(...rates);
		const min = Math.max(0, Math.floor(minRate - 2));
		const max = Math.min(100, Math.ceil(maxRate + 2));
		const range = max - min || 1;

		const width = 100;
		const padding = 2;
		const chartWidth = width - padding * 2;
		const actualChartHeight = chartHeight - padding * 2;
		const stepX = chartWidth / Math.max(1, inputData.length - 1);

		const points = inputData
			.map((d, i) => {
				const x = padding + i * stepX;
				const y = padding + actualChartHeight - ((d.crash_free_rate - min) / range) * actualChartHeight;
				return `${x},${y}`;
			})
			.join(' ');

		const linePoints = inputData.map((d, i) => {
			const x = padding + i * stepX;
			const y = padding + actualChartHeight - ((d.crash_free_rate - min) / range) * actualChartHeight;
			return `${x},${y}`;
		});
		const startX = padding;
		const endX = padding + (inputData.length - 1) * stepX;
		const bottomY = padding + actualChartHeight;
		const areaPath = `M${startX},${bottomY} L${linePoints.join(' L')} L${endX},${bottomY} Z`;

		return { points, area: areaPath, min, max };
	}

	const chartData = $derived(computeChartData(data, height));

	function computeTargetY(chartResult: ChartDataResult, chartHeight: number, targetValue: number): number {
		const { min, max } = chartResult;
		const range = max - min || 1;
		const padding = 2;
		const actualChartHeight = chartHeight - padding * 2;
		return padding + actualChartHeight - ((targetValue - min) / range) * actualChartHeight;
	}

	const targetY = $derived(computeTargetY(chartData, height, target));

	const averageRate = $derived(
		data.length > 0
			? data.reduce((sum, d) => sum + d.crash_free_rate, 0) / data.length
			: 0
	);

	const rateClass = $derived(
		averageRate >= 99 ? 'rate-good' : averageRate >= 95 ? 'rate-warn' : 'rate-bad'
	);
</script>

<div class="crash-free-chart">
	<div class="chart-header">
		<span class="chart-title">Crash-Free Rate</span>
		<span class="chart-average {rateClass}">{averageRate.toFixed(2)}%</span>
	</div>

	{#if data.length === 0}
		<div class="chart-empty">No data available</div>
	{:else}
		<svg class="chart-svg" viewBox="0 0 100 {height}" preserveAspectRatio="none">
			<path d={chartData.area} class="chart-area" />
			<polyline points={chartData.points} class="chart-line" fill="none" />

			{#if showTarget}
				<line
					x1="2"
					y1={targetY}
					x2="98"
					y2={targetY}
					class="target-line"
					stroke-dasharray="2,2"
				/>
			{/if}
		</svg>

		<div class="chart-labels">
			<span class="label-y">{chartData.max}%</span>
			<span class="label-y label-min">{chartData.min}%</span>
		</div>
	{/if}
</div>

<style>
	.crash-free-chart {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		padding: var(--space-4);
		position: relative;
	}

	.chart-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-3);
	}

	.chart-title {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.chart-average {
		font-family: var(--font-mono);
		font-size: var(--text-lg);
		font-weight: 700;
	}

	.rate-good {
		color: var(--color-success);
	}

	.rate-warn {
		color: var(--color-warning);
	}

	.rate-bad {
		color: var(--color-error);
	}

	.chart-svg {
		width: 100%;
		height: 150px;
	}

	.chart-area {
		fill: var(--color-success);
		opacity: 0.15;
	}

	.chart-line {
		stroke: var(--color-success);
		stroke-width: 2;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.target-line {
		stroke: var(--color-fg-muted);
		stroke-width: 0.5;
	}

	.chart-labels {
		position: absolute;
		top: calc(var(--space-4) + var(--space-6));
		right: var(--space-4);
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		height: 150px;
		pointer-events: none;
	}

	.label-y {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
	}

	.chart-empty {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 150px;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
