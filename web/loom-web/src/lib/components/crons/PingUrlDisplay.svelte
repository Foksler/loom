<script lang="ts">
	import { CopyButton } from '$lib/components/common';

	interface Props {
		pingKey: string;
		baseUrl?: string;
	}

	let { pingKey, baseUrl = '' }: Props = $props();

	const pingUrl = $derived(`${baseUrl}/ping/${pingKey}`);
	const startUrl = $derived(`${baseUrl}/ping/${pingKey}/start`);
</script>

<div class="ping-url-display">
	<h4 class="ping-title">Ping URLs</h4>

	<div class="url-row">
		<span class="url-label">Success</span>
		<code class="url-value">{pingUrl}</code>
		<CopyButton value={pingUrl} size="sm" variant="ghost" />
	</div>

	<div class="url-row">
		<span class="url-label">Start</span>
		<code class="url-value">{startUrl}</code>
		<CopyButton value={startUrl} size="sm" variant="ghost" />
	</div>

	<div class="usage-hint">
		<code class="usage-code">curl {pingUrl}</code>
	</div>
</div>

<style>
	.ping-url-display {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		padding: var(--space-4);
	}

	.ping-title {
		margin: 0 0 var(--space-3) 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.url-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) 0;
		border-bottom: 1px solid var(--color-border-muted);
	}

	.url-row:last-of-type {
		border-bottom: none;
	}

	.url-label {
		width: 60px;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		text-transform: uppercase;
	}

	.url-value {
		flex: 1;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg);
		background: var(--color-bg-subtle);
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-sm);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.usage-hint {
		margin-top: var(--space-3);
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg-subtle);
		border-radius: var(--radius-sm);
	}

	.usage-code {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}
</style>
