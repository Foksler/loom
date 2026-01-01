<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { CommitWithDiff } from '$lib/api/repos';
	import { i18n } from '$lib/i18n';

	interface Props {
		commit: CommitWithDiff;
		owner: string;
		repo: string;
	}

	let { commit, owner, repo }: Props = $props();

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
		type: 'context' | 'addition' | 'deletion' | 'header';
		content: string;
		oldLineNum?: number;
		newLineNum?: number;
	}

	const parsedDiff = $derived(parseDiff(commit.diff));

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

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
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
</script>

<div class="space-y-6">
	<div class="bg-bg-muted border border-border rounded-lg p-4">
		<h2 class="text-lg font-bold text-fg mb-2">{getCommitTitle(commit.message)}</h2>

		{#if getCommitBody(commit.message)}
			<pre class="text-sm text-fg-muted whitespace-pre-wrap mb-4 font-mono">{getCommitBody(commit.message)}</pre>
		{/if}

		<div class="flex items-center gap-4 text-sm">
			<div class="flex items-center gap-2">
				<span class="font-medium text-fg">{commit.author_name}</span>
				<span class="text-fg-muted">&lt;{commit.author_email}&gt;</span>
			</div>
			<span class="text-fg-muted">{formatDate(commit.author_date)}</span>
		</div>

		<div class="flex items-center gap-4 mt-3 text-sm">
			<code class="font-mono text-xs bg-bg px-2 py-1 rounded">{commit.sha}</code>
			{#if commit.parent_shas.length > 0}
				<span class="text-fg-muted">
					{commit.parent_shas.length > 1 ? i18n._('client.repos.diff.parents') : i18n._('client.repos.diff.parent')}:
					{#each commit.parent_shas as parent, i}
						<a href="/repos/{owner}/{repo}/commit/{parent}" class="font-mono text-accent hover:underline">
							{parent.slice(0, 7)}
						</a>{i < commit.parent_shas.length - 1 ? ', ' : ''}
					{/each}
				</span>
			{/if}
		</div>
	</div>

	<div class="flex items-center gap-4 text-sm">
		<span class="text-fg-muted">
			{i18n._('client.repos.diff.showing')} <strong class="text-fg">{parsedDiff.length}</strong> {parsedDiff.length !== 1 ? i18n._('client.repos.diff.changed_files') : i18n._('client.repos.diff.changed_file')}
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
</div>
