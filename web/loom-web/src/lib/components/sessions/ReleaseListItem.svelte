<script lang="ts">
	import { RelativeTime } from '$lib/components/common';
	import AdoptionStageBadge from './AdoptionStageBadge.svelte';

	type AdoptionStage = 'new' | 'low' | 'adopted' | 'replaced';

	interface ReleaseHealth {
		version: string;
		crash_free_rate: number;
		crash_free_users: number;
		adoption_percentage: number;
		adoption_stage: AdoptionStage;
		total_sessions: number;
		crashed_sessions: number;
		total_users: number;
		crashed_users: number;
		first_seen?: string;
		last_seen?: string;
	}

	interface Props {
		release: ReleaseHealth;
		onclick?: (release: ReleaseHealth) => void;
	}

	let { release, onclick }: Props = $props();

	const crashFreeClass = $derived(
		release.crash_free_rate >= 99
			? 'crash-free-good'
			: release.crash_free_rate >= 95
				? 'crash-free-warn'
				: 'crash-free-bad'
	);

	function handleClick() {
		onclick?.(release);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			onclick?.(release);
		}
	}
</script>

<div
	class="release-list-item"
	role="button"
	tabindex="0"
	onclick={handleClick}
	onkeydown={handleKeydown}
>
	<div class="release-main">
		<span class="release-version">{release.version}</span>
		<AdoptionStageBadge stage={release.adoption_stage} size="sm" />
	</div>

	<div class="release-crash-free {crashFreeClass}">
		<span class="crash-free-value">{release.crash_free_rate.toFixed(1)}%</span>
		<span class="crash-free-label">crash-free</span>
	</div>

	<div class="release-stats">
		<span class="stat">{release.total_sessions.toLocaleString()} sessions</span>
		<span class="stat">{release.total_users.toLocaleString()} users</span>
	</div>

	<div class="release-adoption">
		<span class="adoption-value">{release.adoption_percentage.toFixed(1)}%</span>
		<span class="adoption-label">adoption</span>
	</div>

	{#if release.last_seen}
		<div class="release-time">
			<RelativeTime timestamp={release.last_seen} />
		</div>
	{/if}
</div>

<style>
	.release-list-item {
		display: grid;
		grid-template-columns: 1fr auto auto auto auto;
		gap: var(--space-4);
		align-items: center;
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			border-color 0.15s ease;
	}

	.release-list-item:hover {
		background: var(--color-bg-subtle);
		border-color: var(--color-border);
	}

	.release-list-item:focus {
		outline: 2px solid var(--color-accent);
		outline-offset: 2px;
	}

	.release-main {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.release-version {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.release-crash-free {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
	}

	.crash-free-value {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 700;
	}

	.crash-free-good .crash-free-value {
		color: var(--color-success);
	}

	.crash-free-warn .crash-free-value {
		color: var(--color-warning);
	}

	.crash-free-bad .crash-free-value {
		color: var(--color-error);
	}

	.crash-free-label {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
		text-transform: uppercase;
	}

	.release-stats {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
	}

	.stat {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.release-adoption {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
	}

	.adoption-value {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.adoption-label {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
		text-transform: uppercase;
	}

	.release-time {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}
</style>
