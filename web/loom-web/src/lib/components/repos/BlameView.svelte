<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { BlameLine } from '$lib/api/repos';

	interface Props {
		blameLines: BlameLine[];
		path: string;
		owner: string;
		repo: string;
	}

	let { blameLines, path, owner, repo }: Props = $props();

	const basePath = $derived(`/repos/${owner}/${repo}`);
	const fileName = $derived(path.split('/').pop() ?? '');

	interface BlameBlock {
		sha: string;
		authorName: string;
		authorDate: string;
		lines: BlameLine[];
		startLine: number;
	}

	const blameBlocks = $derived(() => {
		const blocks: BlameBlock[] = [];
		let currentBlock: BlameBlock | null = null;

		for (const line of blameLines) {
			if (!currentBlock || currentBlock.sha !== line.commit_sha) {
				if (currentBlock) blocks.push(currentBlock);
				currentBlock = {
					sha: line.commit_sha,
					authorName: line.author_name,
					authorDate: line.author_date,
					lines: [line],
					startLine: line.line_number,
				};
			} else {
				currentBlock.lines.push(line);
			}
		}

		if (currentBlock) blocks.push(currentBlock);
		return blocks;
	});

	function formatDate(dateStr: string): string {
		const date = new Date(dateStr);
		const now = new Date();
		const diff = now.getTime() - date.getTime();

		const days = Math.floor(diff / 86400000);
		if (days < 1) return 'today';
		if (days < 30) return `${days}d ago`;
		if (days < 365) return `${Math.floor(days / 30)}mo ago`;
		return `${Math.floor(days / 365)}y ago`;
	}

	const colors = [
		'bg-accent/5',
		'bg-success/5',
		'bg-warning/5',
		'bg-error/5',
		'bg-fg/5',
	];

	function getBlockColor(index: number): string {
		return colors[index % colors.length];
	}
</script>

<div class="border border-border rounded-lg overflow-hidden">
	<div class="flex items-center justify-between px-4 py-2 bg-bg-muted border-b border-border">
		<span class="text-sm font-medium text-fg">{fileName}</span>
		<span class="text-xs text-fg-muted">{blameLines.length} lines</span>
	</div>

	<div class="overflow-x-auto">
		<table class="w-full text-sm font-mono">
			<tbody>
				{#each blameBlocks() as block, blockIndex}
					{#each block.lines as line, lineIndex}
						<tr class="hover:bg-bg-muted group {getBlockColor(blockIndex)}">
							{#if lineIndex === 0}
								<td
									rowspan={block.lines.length}
									class="w-64 px-3 py-1 text-xs text-fg-muted border-r border-border align-top bg-bg-muted/50"
								>
									<div class="flex flex-col gap-0.5">
										<a
											href="{basePath}/commit/{block.sha}"
											class="font-mono text-accent hover:underline"
											title={block.sha}
										>
											{block.sha.slice(0, 7)}
										</a>
										<span class="truncate" title={block.authorName}>{block.authorName}</span>
										<span class="text-fg-subtle" title={block.authorDate}>{formatDate(block.authorDate)}</span>
									</div>
								</td>
							{/if}
							<td class="w-12 px-3 py-0.5 text-right text-fg-muted select-none border-r border-border">
								<a href="#{line.line_number}" id={String(line.line_number)} class="hover:text-accent">
									{line.line_number}
								</a>
							</td>
							<td class="px-4 py-0.5 whitespace-pre text-fg">
								{line.content || ' '}
							</td>
						</tr>
					{/each}
				{/each}
			</tbody>
		</table>
	</div>

	{#if blameLines.length === 0}
		<div class="px-4 py-8 text-center text-fg-muted">
			No blame information available
		</div>
	{/if}
</div>
