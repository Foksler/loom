<script lang="ts">
	interface ReleaseData {
		version: string;
		color: string;
		data: { timestamp: string; percentage: number }[];
	}

	interface Props {
		releases: ReleaseData[];
		height?: number;
	}

	let { releases, height = 200 }: Props = $props();

	const defaultColors = [
		'var(--color-accent)',
		'var(--color-success)',
		'var(--color-warning)',
		'var(--color-info)',
		'var(--color-error)',
		'var(--weaver-indigo)',
		'var(--weaver-madder)',
		'var(--weaver-weld)',
	];

	type ProcessedRelease = ReleaseData & { color: string };

	function computeProcessedReleases(inputReleases: ReleaseData[]): ProcessedRelease[] {
		return inputReleases.map((release, index) => ({
			...release,
			color: release.color || defaultColors[index % defaultColors.length],
		}));
	}

	const processedReleases = $derived(computeProcessedReleases(releases));

	function computeChartData(
		inputReleases: ReleaseData[],
		processed: ProcessedRelease[],
		chartHeight: number
	) {
		if (inputReleases.length === 0 || inputReleases[0].data.length === 0) {
			return { areas: [], labels: [] };
		}

		const dataPoints = inputReleases[0].data.length;
		const width = 100;
		const padding = 2;
		const chartWidth = width - padding * 2;
		const actualChartHeight = chartHeight - padding * 2;
		const stepX = chartWidth / Math.max(1, dataPoints - 1);

		const areas: { path: string; color: string; version: string }[] = [];

		for (let r = 0; r < processed.length; r++) {
			const release = processed[r];
			const points: string[] = [];
			const bottomPoints: string[] = [];

			for (let i = 0; i < dataPoints; i++) {
				const x = padding + i * stepX;

				let stackedY = 0;
				for (let j = 0; j <= r; j++) {
					stackedY += processed[j].data[i]?.percentage ?? 0;
				}

				let prevStackedY = 0;
				for (let j = 0; j < r; j++) {
					prevStackedY += processed[j].data[i]?.percentage ?? 0;
				}

				const y = padding + actualChartHeight - (stackedY / 100) * actualChartHeight;
				const prevY = padding + actualChartHeight - (prevStackedY / 100) * actualChartHeight;

				points.push(`${x},${y}`);
				bottomPoints.unshift(`${x},${prevY}`);
			}

			const path = `M${points.join(' L')} L${bottomPoints.join(' L')} Z`;
			areas.push({ path, color: release.color, version: release.version });
		}

		const labels = [
			{ value: '100%', y: padding },
			{ value: '75%', y: padding + actualChartHeight * 0.25 },
			{ value: '50%', y: padding + actualChartHeight * 0.5 },
			{ value: '25%', y: padding + actualChartHeight * 0.75 },
			{ value: '0%', y: padding + actualChartHeight },
		];

		return { areas, labels };
	}

	const chartData = $derived(computeChartData(releases, processedReleases, height));
</script>

<div class="adoption-chart">
	<div class="chart-header">
		<span class="chart-title">Release Adoption</span>
	</div>

	{#if releases.length === 0}
		<div class="chart-empty">No adoption data available</div>
	{:else}
		<div class="chart-container">
			<svg class="chart-svg" viewBox="0 0 100 {height}" preserveAspectRatio="none">
				{#each chartData.areas as area}
					<path d={area.path} fill={area.color} opacity="0.8" />
				{/each}
			</svg>

			<div class="chart-y-labels">
				{#each chartData.labels as label}
					<span class="y-label" style="top: {label.y}px">{label.value}</span>
				{/each}
			</div>
		</div>

		<div class="chart-legend">
			{#each processedReleases as release}
				<div class="legend-item">
					<span class="legend-color" style="background-color: {release.color}"></span>
					<span class="legend-label">{release.version}</span>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.adoption-chart {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		padding: var(--space-4);
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

	.chart-container {
		position: relative;
		padding-left: 40px;
	}

	.chart-svg {
		width: 100%;
		height: 200px;
	}

	.chart-y-labels {
		position: absolute;
		top: 0;
		left: 0;
		width: 35px;
		height: 100%;
	}

	.y-label {
		position: absolute;
		right: 5px;
		transform: translateY(-50%);
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
	}

	.chart-legend {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		margin-top: var(--space-3);
		padding-top: var(--space-3);
		border-top: 1px solid var(--color-border-muted);
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}

	.legend-color {
		width: 12px;
		height: 12px;
		border-radius: 2px;
	}

	.legend-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg);
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
