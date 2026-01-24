<script lang="ts">
	import SourceContext from './SourceContext.svelte';

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

	interface Props {
		frame: Frame;
		expanded?: boolean;
		ontoggle?: () => void;
	}

	let { frame, expanded = false, ontoggle }: Props = $props();

	const hasSource = $derived(!!frame.context_line);
	const location = $derived(
		[frame.filename, frame.lineno, frame.colno].filter(Boolean).join(':')
	);

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			ontoggle?.();
		}
	}
</script>

<li class="frame" class:in-app={frame.in_app} class:expanded>
	<div
		class="frame-header"
		role="button"
		tabindex="0"
		onclick={ontoggle}
		onkeydown={handleKeydown}
		aria-expanded={expanded}
	>
		<span class="frame-indicator" class:has-source={hasSource}>
			{#if hasSource}
				<span class="expand-icon">{expanded ? '\u25BC' : '\u25B6'}</span>
			{/if}
		</span>
		<span class="frame-function">{frame.function || '<anonymous>'}</span>
		{#if frame.module}
			<span class="frame-module">in {frame.module}</span>
		{/if}
		<span class="frame-location">{location}</span>
		{#if frame.in_app}
			<span class="frame-app-badge">app</span>
		{/if}
	</div>

	{#if expanded && hasSource}
		<div class="frame-source">
			<SourceContext
				preContext={frame.pre_context ?? []}
				contextLine={frame.context_line ?? ''}
				postContext={frame.post_context ?? []}
				lineNumber={frame.lineno ?? 0}
			/>
		</div>
	{/if}
</li>

<style>
	.frame {
		border-left: 2px solid var(--color-border-muted);
		margin-left: var(--space-2);
	}

	.frame.in-app {
		border-left-color: var(--color-accent);
	}

	.frame-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		cursor: pointer;
		transition: background-color 0.15s ease;
	}

	.frame-header:hover {
		background: var(--color-bg-subtle);
	}

	.frame-indicator {
		width: 16px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.expand-icon {
		font-size: 10px;
		color: var(--color-fg-muted);
		transition: transform 0.15s ease;
	}

	.frame.expanded .expand-icon {
		color: var(--color-fg);
	}

	.frame-function {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-fg);
	}

	.frame:not(.in-app) .frame-function {
		color: var(--color-fg-muted);
	}

	.frame-module {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.frame-location {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-subtle);
		margin-left: auto;
	}

	.frame-app-badge {
		padding: 1px var(--space-1);
		font-family: var(--font-mono);
		font-size: 9px;
		font-weight: 600;
		text-transform: uppercase;
		background: var(--color-accent-soft);
		color: var(--color-accent);
		border-radius: 2px;
	}

	.frame-source {
		margin: var(--space-2) var(--space-3);
		border-radius: var(--radius-sm);
		overflow: hidden;
	}
</style>
