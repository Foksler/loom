<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { Clip } from '$lib/api/clips';
	import ClipVisibilityBadge from './ClipVisibilityBadge.svelte';
	import { trackLinkClick } from '$lib/analytics';

	interface Props {
		clip: Clip;
		onclipclick?: (clip: Clip) => void;
	}

	let { clip, onclipclick }: Props = $props();

	function handleClick() {
		trackLinkClick('clip_card', `/clips/${clip.owner}/${clip.name}`, {
			clip_id: clip.id,
			owner: clip.owner,
			name: clip.name,
		});
		onclipclick?.(clip);
	}

	function formatBytes(bytes: number): string {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
	}

	function formatDate(dateStr: string): string {
		const date = new Date(dateStr);
		return date.toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
		});
	}
</script>

<button class="clip-card" onclick={handleClick}>
	<div class="clip-header">
		<div class="clip-title">
			<span class="clip-owner">{clip.owner}</span>
			<span class="separator">/</span>
			<span class="clip-name">{clip.name}</span>
		</div>
		<ClipVisibilityBadge visibility={clip.visibility} />
	</div>

	{#if clip.description}
		<p class="clip-description">{clip.description}</p>
	{/if}

	<div class="clip-meta">
		{#if clip.language}
			<span class="meta-item language">{clip.language}</span>
		{/if}
		<span class="meta-item">{clip.file_count} files</span>
		<span class="meta-item">{formatBytes(clip.size_bytes)}</span>
		<span class="meta-item">Updated {formatDate(clip.updated_at)}</span>
		{#if clip.is_fork}
			<span class="meta-item fork-badge">Forked</span>
		{/if}
	</div>
</button>

<style>
	.clip-card {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-4);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		text-align: left;
		cursor: pointer;
		transition: all 0.15s ease;
		width: 100%;
	}

	.clip-card:hover {
		border-color: var(--color-border);
		background: var(--color-bg-subtle);
	}

	.clip-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--space-2);
	}

	.clip-title {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
		overflow: hidden;
	}

	.clip-owner {
		color: var(--color-fg-muted);
	}

	.separator {
		color: var(--color-fg-muted);
	}

	.clip-name {
		color: var(--color-accent);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.clip-description {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
	}

	.clip-meta {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
	}

	.meta-item {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.meta-item.language {
		padding: var(--space-1) var(--space-2);
		background: color-mix(in srgb, var(--color-accent) 15%, transparent);
		color: var(--color-accent);
		border-radius: var(--radius-sm);
	}

	.fork-badge {
		padding: var(--space-1) var(--space-2);
		background: color-mix(in srgb, var(--color-info) 15%, transparent);
		color: var(--color-info);
		border-radius: var(--radius-sm);
	}
</style>
