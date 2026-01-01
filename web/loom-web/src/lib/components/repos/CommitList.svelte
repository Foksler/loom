<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { CommitInfo } from '$lib/api/repos';

	interface Props {
		commits: CommitInfo[];
		owner: string;
		repo: string;
	}

	let { commits, owner, repo }: Props = $props();

	const basePath = $derived(`/repos/${owner}/${repo}`);

	function formatDate(dateStr: string): string {
		const date = new Date(dateStr);
		const now = new Date();
		const diff = now.getTime() - date.getTime();

		const minutes = Math.floor(diff / 60000);
		if (minutes < 60) return `${minutes} minutes ago`;

		const hours = Math.floor(diff / 3600000);
		if (hours < 24) return `${hours} hours ago`;

		const days = Math.floor(diff / 86400000);
		if (days < 30) return `${days} days ago`;

		return date.toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
		});
	}

	function getCommitTitle(message: string): string {
		return message.split('\n')[0];
	}

	function getCommitBody(message: string): string | null {
		const lines = message.split('\n');
		if (lines.length <= 1) return null;
		return lines.slice(1).join('\n').trim();
	}

	function getAvatarUrl(email: string): string {
		return `https://www.gravatar.com/avatar/${email}?d=identicon&s=40`;
	}
</script>

<div class="border border-border rounded-lg overflow-hidden divide-y divide-border">
	{#each commits as commit}
		<div class="flex items-start gap-4 px-4 py-3 hover:bg-bg-muted">
			<img
				src={getAvatarUrl(commit.author_email)}
				alt={commit.author_name}
				class="w-10 h-10 rounded-full flex-shrink-0"
			/>

			<div class="flex-1 min-w-0">
				<a
					href="{basePath}/commit/{commit.sha}"
					class="font-medium text-fg hover:text-accent line-clamp-1"
				>
					{getCommitTitle(commit.message)}
				</a>

				{#if getCommitBody(commit.message)}
					<button
						type="button"
						class="text-xs text-fg-muted hover:text-fg mt-0.5"
					>
						...
					</button>
				{/if}

				<div class="flex items-center gap-2 mt-1 text-sm text-fg-muted">
					<span class="font-medium text-fg">{commit.author_name}</span>
					<span>committed</span>
					<span title={commit.author_date}>{formatDate(commit.author_date)}</span>
				</div>
			</div>

			<div class="flex items-center gap-2 flex-shrink-0">
				<a
					href="{basePath}/commit/{commit.sha}"
					class="font-mono text-sm text-accent hover:underline"
					title={commit.sha}
				>
					{commit.sha.slice(0, 7)}
				</a>
				<button
					type="button"
					onclick={() => navigator.clipboard.writeText(commit.sha)}
					class="p-1 hover:bg-bg-subtle rounded"
					title="Copy SHA"
				>
					<svg class="w-4 h-4 text-fg-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
					</svg>
				</button>
			</div>
		</div>
	{/each}

	{#if commits.length === 0}
		<div class="px-4 py-8 text-center text-fg-muted">
			No commits found
		</div>
	{/if}
</div>
