<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { marked } from 'marked';

	interface Props {
		content: string;
	}

	let { content }: Props = $props();

	const html = $derived.by(() => {
		marked.setOptions({
			gfm: true,
			breaks: true
		});
		return marked.parse(content) as string;
	});
</script>

<div class="markdown-content prose">
	{@html html}
</div>

<style>
	.markdown-content {
		padding: var(--space-6);
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
	}

	.prose :global(h1) {
		font-family: var(--font-mono);
		font-size: var(--text-2xl);
		font-weight: 600;
		color: var(--color-fg);
		margin: 0 0 var(--space-4) 0;
		padding-bottom: var(--space-2);
		border-bottom: 1px solid var(--color-border);
	}

	.prose :global(h2) {
		font-family: var(--font-mono);
		font-size: var(--text-xl);
		font-weight: 600;
		color: var(--color-fg);
		margin: var(--space-6) 0 var(--space-3) 0;
		padding-bottom: var(--space-2);
		border-bottom: 1px solid var(--color-border);
	}

	.prose :global(h3) {
		font-family: var(--font-mono);
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-fg);
		margin: var(--space-5) 0 var(--space-2) 0;
	}

	.prose :global(h4),
	.prose :global(h5),
	.prose :global(h6) {
		font-family: var(--font-mono);
		font-size: var(--text-base);
		font-weight: 600;
		color: var(--color-fg);
		margin: var(--space-4) 0 var(--space-2) 0;
	}

	.prose :global(p) {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
		line-height: 1.7;
		margin: 0 0 var(--space-4) 0;
	}

	.prose :global(ul),
	.prose :global(ol) {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
		line-height: 1.7;
		margin: 0 0 var(--space-4) 0;
		padding-left: var(--space-6);
	}

	.prose :global(li) {
		margin-bottom: var(--space-1);
	}

	.prose :global(li > ul),
	.prose :global(li > ol) {
		margin: var(--space-1) 0;
	}

	.prose :global(pre) {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: var(--space-4);
		margin: 0 0 var(--space-4) 0;
		overflow-x: auto;
	}

	.prose :global(code) {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}

	.prose :global(:not(pre) > code) {
		background: var(--color-bg-muted);
		padding: 2px 6px;
		border-radius: var(--radius-sm);
		color: var(--color-accent);
	}

	.prose :global(a) {
		color: var(--color-accent);
		text-decoration: none;
	}

	.prose :global(a:hover) {
		text-decoration: underline;
	}

	.prose :global(blockquote) {
		border-left: 3px solid var(--color-accent);
		padding-left: var(--space-4);
		margin: 0 0 var(--space-4) 0;
		color: var(--color-fg-muted);
		font-style: italic;
	}

	.prose :global(blockquote p) {
		margin-bottom: 0;
	}

	.prose :global(table) {
		width: 100%;
		border-collapse: collapse;
		margin: 0 0 var(--space-4) 0;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}

	.prose :global(th),
	.prose :global(td) {
		border: 1px solid var(--color-border);
		padding: var(--space-2) var(--space-3);
		text-align: left;
	}

	.prose :global(th) {
		background: var(--color-bg-muted);
		font-weight: 600;
	}

	.prose :global(hr) {
		border: none;
		border-top: 1px solid var(--color-border);
		margin: var(--space-6) 0;
	}

	.prose :global(img) {
		max-width: 100%;
		height: auto;
		border-radius: var(--radius-md);
	}

	.prose :global(input[type='checkbox']) {
		margin-right: var(--space-2);
	}

	.prose :global(del) {
		color: var(--color-fg-muted);
	}
</style>
