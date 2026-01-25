<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { ClipRevision } from '$lib/api/clips';

	interface Props {
		revisions: ClipRevision[];
		onselect?: (revision: ClipRevision) => void;
	}

	let { revisions, onselect }: Props = $props();

	function formatDate(dateString: string): string {
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
		});
	}

	function shortenSha(sha: string): string {
		return sha.slice(0, 7);
	}
</script>

<div class="revision-list">
	{#each revisions as revision (revision.sha)}
		<button
			class="revision-item"
			onclick={() => onselect?.(revision)}
			type="button"
		>
			<div class="revision-header">
				<code class="revision-sha">{shortenSha(revision.sha)}</code>
				<span class="revision-date">{formatDate(revision.timestamp)}</span>
			</div>
			<p class="revision-message">{revision.message}</p>
			<div class="revision-author">
				<span class="author-name">{revision.author_name}</span>
				<span class="author-email">&lt;{revision.author_email}&gt;</span>
			</div>
		</button>
	{/each}
</div>

<style>
	.revision-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.revision-item {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-4);
		background: var(--color-bg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		cursor: pointer;
		text-align: left;
		transition: border-color 0.15s ease;
	}

	.revision-item:hover {
		border-color: var(--color-border);
	}

	.revision-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.revision-sha {
		padding: var(--space-1) var(--space-2);
		background: var(--color-bg-muted);
		border-radius: var(--radius-sm);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-accent);
	}

	.revision-date {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.revision-message {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
		line-height: 1.4;
	}

	.revision-author {
		display: flex;
		gap: var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.author-name {
		font-weight: 500;
	}
</style>
