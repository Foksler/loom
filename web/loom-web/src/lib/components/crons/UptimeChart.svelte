<script lang="ts">
	type CheckInStatus = 'ok' | 'error' | 'missed' | 'timeout' | 'in_progress';

	interface DayData {
		date: string;
		status: CheckInStatus | 'none';
		count: number;
	}

	interface Props {
		data: DayData[];
		days?: number;
	}

	let { data, days = 30 }: Props = $props();

	const statusColors: Record<CheckInStatus | 'none', string> = {
		ok: 'var(--color-success)',
		error: 'var(--color-error)',
		missed: 'var(--color-warning)',
		timeout: 'var(--color-error)',
		in_progress: 'var(--color-info)',
		none: 'var(--color-bg-subtle)',
	};

	function computePaddedData(inputData: DayData[], numDays: number): DayData[] {
		const result: DayData[] = [];
		const today = new Date();

		for (let i = numDays - 1; i >= 0; i--) {
			const date = new Date(today);
			date.setDate(date.getDate() - i);
			const dateStr = date.toISOString().split('T')[0];

			const existing = inputData.find((d) => d.date === dateStr);
			result.push(existing ?? { date: dateStr, status: 'none', count: 0 });
		}

		return result;
	}

	const paddedData = $derived(computePaddedData(data, days));

	function computeUptimePercentage(inputData: DayData[]): number | null {
		const activeData = inputData.filter((d) => d.status !== 'none');
		if (activeData.length === 0) return null;

		const okCount = activeData.filter((d) => d.status === 'ok').length;
		return Math.round((okCount / activeData.length) * 100);
	}

	const uptimePercentage = $derived(computeUptimePercentage(data));
</script>

<div class="uptime-chart">
	<div class="chart-header">
		<span class="chart-title">Uptime</span>
		{#if uptimePercentage !== null}
			<span class="uptime-percentage">{uptimePercentage}%</span>
		{/if}
	</div>
	<div class="chart-bars">
		{#each paddedData as day}
			<div
				class="chart-bar"
				style="background-color: {statusColors[day.status]}"
				title="{day.date}: {day.count} check-ins ({day.status})"
			></div>
		{/each}
	</div>
	<div class="chart-legend">
		<span class="legend-label">{days} days ago</span>
		<span class="legend-label">Today</span>
	</div>
</div>

<style>
	.uptime-chart {
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

	.uptime-percentage {
		font-family: var(--font-mono);
		font-size: var(--text-lg);
		font-weight: 700;
		color: var(--color-success);
	}

	.chart-bars {
		display: flex;
		gap: 2px;
		height: 32px;
	}

	.chart-bar {
		flex: 1;
		border-radius: 2px;
		transition: transform 0.15s ease;
		cursor: pointer;
	}

	.chart-bar:hover {
		transform: scaleY(1.1);
	}

	.chart-legend {
		display: flex;
		justify-content: space-between;
		margin-top: var(--space-2);
	}

	.legend-label {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
	}
</style>
