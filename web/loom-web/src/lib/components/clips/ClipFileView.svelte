<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { ClipFile } from '$lib/api/clips';

	interface Props {
		file: ClipFile;
	}

	let { file }: Props = $props();

	function formatBytes(bytes: number): string {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
	}
</script>

<div class="file-view">
	<div class="file-header">
		<div class="file-path">
			<span class="path-text">{file.path}</span>
			{#if file.language}
				<span class="language-badge">{file.language}</span>
			{/if}
		</div>
		<div class="file-meta">
			<span class="meta-item">{formatBytes(file.size)}</span>
			{#if file.is_redacted}
				<span class="redacted-badge">Contains redacted content</span>
			{/if}
		</div>
	</div>
	<div class="file-content">
		<pre><code>{file.content}</code></pre>
	</div>
</div>

<style>
	.file-view {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.file-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-muted);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.file-path {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.path-text {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-fg);
	}

	.language-badge {
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: color-mix(in srgb, var(--color-accent) 15%, transparent);
		color: var(--color-accent);
		border-radius: var(--radius-sm);
	}

	.file-meta {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.meta-item {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.redacted-badge {
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: color-mix(in srgb, var(--color-warning) 15%, transparent);
		color: var(--color-warning);
		border-radius: var(--radius-sm);
	}

	.file-content {
		padding: var(--space-4);
		background: var(--color-bg);
		overflow-x: auto;
	}

	.file-content pre {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		line-height: 1.6;
		color: var(--color-fg);
		white-space: pre;
	}

	.file-content code {
		display: block;
	}
</style>
