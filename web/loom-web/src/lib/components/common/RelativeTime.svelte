<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		timestamp: string | Date;
		live?: boolean;
		showTooltip?: boolean;
	}

	let { timestamp, live = true, showTooltip = true }: Props = $props();

	let now = $state(Date.now());
	let intervalId: ReturnType<typeof setInterval> | undefined;

	const date = $derived(timestamp instanceof Date ? timestamp : new Date(timestamp));
	const absoluteTime = $derived(date.toLocaleString());

	function computeRelativeTime(currentTime: number, targetDate: Date): string {
		const diff = currentTime - targetDate.getTime();
		const seconds = Math.floor(diff / 1000);
		const minutes = Math.floor(seconds / 60);
		const hours = Math.floor(minutes / 60);
		const days = Math.floor(hours / 24);
		const weeks = Math.floor(days / 7);
		const months = Math.floor(days / 30);
		const years = Math.floor(days / 365);

		if (seconds < 5) return 'just now';
		if (seconds < 60) return `${seconds}s ago`;
		if (minutes < 60) return `${minutes}m ago`;
		if (hours < 24) return `${hours}h ago`;
		if (days < 7) return `${days}d ago`;
		if (weeks < 4) return `${weeks}w ago`;
		if (months < 12) return `${months}mo ago`;
		return `${years}y ago`;
	}

	const relativeTime = $derived(computeRelativeTime(now, date));

	onMount(() => {
		if (live) {
			intervalId = setInterval(() => {
				now = Date.now();
			}, 10000);
		}

		return () => {
			if (intervalId) {
				clearInterval(intervalId);
			}
		};
	});
</script>

<time class="relative-time" datetime={date.toISOString()} title={showTooltip ? absoluteTime : undefined}>
	{relativeTime}
</time>

<style>
	.relative-time {
		font-family: var(--font-mono);
		font-size: inherit;
		color: var(--color-fg-muted);
		cursor: default;
	}
</style>
