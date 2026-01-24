<script lang="ts">
	import { StatCard } from '$lib/components/common';

	interface OverviewStats {
		total_sessions: number;
		total_users: number;
		crash_free_sessions: number;
		crash_free_users: number;
		active_releases: number;
		sessions_trend?: {
			direction: 'up' | 'down' | 'flat';
			value: string;
			positive?: boolean;
		};
	}

	interface Props {
		stats: OverviewStats;
	}

	let { stats }: Props = $props();

	const sessionsCrashFree = $derived(
		stats.total_sessions > 0 ? stats.crash_free_sessions : 0
	);

	const usersCrashFree = $derived(stats.total_users > 0 ? stats.crash_free_users : 0);

	const crashFreeVariant = $derived(
		sessionsCrashFree >= 99 ? 'success' : sessionsCrashFree >= 95 ? 'warning' : 'error'
	);
</script>

<div class="release-health-overview">
	<div class="overview-grid">
		<StatCard
			label="Total Sessions"
			value={stats.total_sessions.toLocaleString()}
			trend={stats.sessions_trend}
		/>
		<StatCard label="Total Users" value={stats.total_users.toLocaleString()} />
		<StatCard
			label="Crash-Free Sessions"
			value="{sessionsCrashFree.toFixed(1)}%"
			variant={crashFreeVariant}
		/>
		<StatCard
			label="Crash-Free Users"
			value="{usersCrashFree.toFixed(1)}%"
			variant={crashFreeVariant}
		/>
		<StatCard
			label="Active Releases"
			value={stats.active_releases.toString()}
		/>
	</div>
</div>

<style>
	.release-health-overview {
		width: 100%;
	}

	.overview-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
		gap: var(--space-3);
	}
</style>
