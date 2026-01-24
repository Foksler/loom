<script lang="ts">
	interface Props {
		preContext?: string[];
		contextLine: string;
		postContext?: string[];
		lineNumber: number;
	}

	let { preContext = [], contextLine, postContext = [], lineNumber }: Props = $props();

	const startLine = $derived(lineNumber - preContext.length);
</script>

<div class="source-context">
	<table>
		<tbody>
			{#each preContext as line, i}
				<tr class="context-line">
					<td class="line-number">{startLine + i}</td>
					<td class="line-content"><code>{line}</code></td>
				</tr>
			{/each}

			<tr class="error-line">
				<td class="line-number">{lineNumber}</td>
				<td class="line-content"><code>{contextLine}</code></td>
			</tr>

			{#each postContext as line, i}
				<tr class="context-line">
					<td class="line-number">{lineNumber + 1 + i}</td>
					<td class="line-content"><code>{line}</code></td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>

<style>
	.source-context {
		background: var(--color-bg-subtle);
		border-radius: var(--radius-sm);
		overflow-x: auto;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}

	table {
		width: 100%;
		border-collapse: collapse;
	}

	tr {
		line-height: 1.6;
	}

	.line-number {
		width: 50px;
		text-align: right;
		padding: 0 var(--space-3) 0 var(--space-2);
		color: var(--color-fg-muted);
		user-select: none;
		border-right: 1px solid var(--color-border-muted);
	}

	.line-content {
		padding-left: var(--space-3);
		white-space: pre;
	}

	.line-content code {
		font-family: inherit;
	}

	.context-line {
		color: var(--color-fg-muted);
	}

	.error-line {
		background: var(--color-error-soft);
	}

	.error-line .line-number {
		color: var(--color-error);
		font-weight: 600;
		border-right-color: var(--color-error);
	}

	.error-line .line-content {
		color: var(--color-fg);
	}
</style>
