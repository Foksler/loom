<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { CompareResult } from '$lib/api/repos';
	import CommitList from './CommitList.svelte';

	interface Props {
		result: CompareResult;
		owner: string;
		repo: string;
	}

	let { result, owner, repo }: Props = $props();

	interface DiffFile {
		header: string;
		oldPath: string;
		newPath: string;
		hunks: DiffHunk[];
		additions: number;
		deletions: number;
	}

	interface DiffHunk {
		header: string;
		lines: DiffLine[];
	}

	interface DiffLine {
		type: 'context' | 'addition' | 'deletion';
		content: string;
		oldLineNum?: number;
		newLineNum?: number;
	}

	const parsedDiff = $derived(parseDiff(result.diff));

	function parseDiff(diff: string): DiffFile[] {
		const files: DiffFile[] = [];
		const fileChunks = diff.split(/^diff --git /m).filter(Boolean);

		for (const chunk of fileChunks) {
			const lines = chunk.split('\n');
			const headerMatch = lines[0]?.match(/a\/(.+) b\/(.+)/);
			if (!headerMatch) continue;

			const file: DiffFile = {
				header: `diff --git ${lines[0]}`,
				oldPath: headerMatch[1],
				newPath: headerMatch[2],
				hunks: [],
				additions: 0,
				deletions: 0,
			};

			let currentHunk: DiffHunk | null = null;
			let oldLine = 0;
			let newLine = 0;

			for (const line of lines.slice(1)) {
				if (line.startsWith('@@')) {
					if (currentHunk) file.hunks.push(currentHunk);
					const match = line.match(/@@ -(\d+),?\d* \+(\d+),?\d* @@/);
					oldLine = match ? parseInt(match[1]) : 0;
					newLine = match ? parseInt(match[2]) : 0;
					currentHunk = { header: line, lines: [] };
					continue;
				}

				if (!currentHunk) continue;

				if (line.startsWith('+') && !line.startsWith('+++')) {
					currentHunk.lines.push({ type: 'addition', content: line.slice(1), newLineNum: newLine++ });
					file.additions++;
				} else if (line.startsWith('-') && !line.startsWith('---')) {
					currentHunk.lines.push({ type: 'deletion', content: line.slice(1), oldLineNum: oldLine++ });
					file.deletions++;
				} else if (line.startsWith(' ')) {
					currentHunk.lines.push({ type: 'context', content: line.slice(1), oldLineNum: oldLine++, newLineNum: newLine++ });
				}
			}

			if (currentHunk) file.hunks.push(currentHunk);
			files.push(file);
		}

		return files;
	}

	const totalAdditions = $derived(parsedDiff.reduce((sum, f) => sum + f.additions, 0));
	const totalDeletions = $derived(parsedDiff.reduce((sum, f) => sum + f.deletions, 0));
</script>

<div class="space-y-6">
	<div class="bg-bg-muted border border-border rounded-lg p-4">
		<div class="flex items-center gap-2 text-lg font-medium text-fg">
			<code class="font-mono text-sm bg-bg px-2 py-1 rounded">{result.base_ref}</code>
			<svg class="w-5 h-5 text-fg-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3" />
			</svg>
			<code class="font-mono text-sm bg-bg px-2 py-1 rounded">{result.head_ref}</code>
		</div>

		<div class="flex items-center gap-4 mt-3 text-sm text-fg-muted">
			{#if result.ahead_by > 0}
				<span>
					<strong class="text-success">{result.ahead_by}</strong> commit{result.ahead_by !== 1 ? 's' : ''} ahead
				</span>
			{/if}
			{#if result.behind_by > 0}
				<span>
					<strong class="text-error">{result.behind_by}</strong> commit{result.behind_by !== 1 ? 's' : ''} behind
				</span>
			{/if}
		</div>
	</div>

	{#if result.commits.length > 0}
		<div>
			<h3 class="text-lg font-medium text-fg mb-3">Commits</h3>
			<CommitList commits={result.commits} {owner} {repo} />
		</div>
	{/if}

	<div class="flex items-center gap-4 text-sm">
		<span class="text-fg-muted">
			Showing <strong class="text-fg">{parsedDiff.length}</strong> changed file{parsedDiff.length !== 1 ? 's' : ''}
		</span>
		<span class="text-success">+{totalAdditions}</span>
		<span class="text-error">-{totalDeletions}</span>
	</div>

	{#each parsedDiff as file}
		<div class="border border-border rounded-lg overflow-hidden">
			<div class="flex items-center justify-between px-4 py-2 bg-bg-muted border-b border-border">
				<span class="font-mono text-sm text-fg">{file.newPath}</span>
				<div class="flex items-center gap-2 text-sm">
					<span class="text-success">+{file.additions}</span>
					<span class="text-error">-{file.deletions}</span>
				</div>
			</div>

			<div class="overflow-x-auto">
				<table class="w-full text-sm font-mono">
					<tbody>
						{#each file.hunks as hunk}
							<tr class="bg-accent/10">
								<td colspan="3" class="px-4 py-1 text-fg-muted text-xs">{hunk.header}</td>
							</tr>
							{#each hunk.lines as line}
								<tr class="{line.type === 'addition' ? 'bg-success/10' : line.type === 'deletion' ? 'bg-error/10' : ''}">
									<td class="w-12 px-2 py-0 text-right text-fg-muted select-none border-r border-border text-xs">
										{line.oldLineNum ?? ''}
									</td>
									<td class="w-12 px-2 py-0 text-right text-fg-muted select-none border-r border-border text-xs">
										{line.newLineNum ?? ''}
									</td>
									<td class="px-4 py-0 whitespace-pre {line.type === 'addition' ? 'text-success' : line.type === 'deletion' ? 'text-error' : 'text-fg'}">
										<span class="inline-block w-4">{line.type === 'addition' ? '+' : line.type === 'deletion' ? '-' : ' '}</span>{line.content}
									</td>
								</tr>
							{/each}
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	{/each}

	{#if parsedDiff.length === 0 && result.commits.length === 0}
		<div class="text-center text-fg-muted py-8">
			These branches are identical
		</div>
	{/if}
</div>
