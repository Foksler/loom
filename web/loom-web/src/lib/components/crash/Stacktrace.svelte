<script lang="ts">
	import StacktraceFrame from './StacktraceFrame.svelte';

	interface Frame {
		function?: string;
		module?: string;
		filename?: string;
		abs_path?: string;
		lineno?: number;
		colno?: number;
		context_line?: string;
		pre_context?: string[];
		post_context?: string[];
		in_app: boolean;
	}

	interface Stacktrace {
		frames: Frame[];
	}

	interface Props {
		stacktrace: Stacktrace;
		title?: string;
		raw?: boolean;
	}

	let { stacktrace, title = 'Stacktrace', raw = false }: Props = $props();

	let expandedFrames = $state<Set<number>>(new Set());
	let showAllFrames = $state(false);

	const frames = $derived(stacktrace?.frames ?? []);
	const inAppFrames = $derived(frames.filter((f) => f.in_app));
	const displayedFrames = $derived(showAllFrames ? frames : inAppFrames);

	function toggleFrame(index: number) {
		const newSet = new Set(expandedFrames);
		if (newSet.has(index)) {
			newSet.delete(index);
		} else {
			newSet.add(index);
		}
		expandedFrames = newSet;
	}

	function expandAll() {
		const indices = displayedFrames.map((_, i) => i).filter((i) => displayedFrames[i].context_line);
		expandedFrames = new Set(indices);
	}

	function collapseAll() {
		expandedFrames = new Set();
	}
</script>

<div class="stacktrace" class:raw>
	<div class="stacktrace-header">
		<h3 class="stacktrace-title">{title}</h3>
		<div class="stacktrace-controls">
			<span class="frame-count">
				{displayedFrames.length} of {frames.length} frames
			</span>
			<button class="control-btn" onclick={() => (showAllFrames = !showAllFrames)}>
				{showAllFrames ? 'App only' : 'All frames'}
			</button>
			<button class="control-btn" onclick={expandAll}>Expand</button>
			<button class="control-btn" onclick={collapseAll}>Collapse</button>
		</div>
	</div>

	{#if displayedFrames.length === 0}
		<div class="stacktrace-empty">No frames to display</div>
	{:else}
		<ol class="frames">
			{#each displayedFrames as frame, index (index)}
				<StacktraceFrame
					{frame}
					expanded={expandedFrames.has(index)}
					ontoggle={() => toggleFrame(index)}
				/>
			{/each}
		</ol>
	{/if}
</div>

<style>
	.stacktrace {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.stacktrace.raw {
		opacity: 0.7;
	}

	.stacktrace-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-subtle);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.stacktrace-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.stacktrace-controls {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.frame-count {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.control-btn {
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: var(--color-bg-muted);
		color: var(--color-fg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			color 0.15s ease;
	}

	.control-btn:hover {
		background: var(--color-bg);
		color: var(--color-fg);
	}

	.frames {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.stacktrace-empty {
		padding: var(--space-6);
		text-align: center;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
