<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import type { Repository, Branch, CommitInfo } from '$lib/api/repos';
	import { CommitList, BranchSelector } from '$lib/components/repos';
	import { Button } from '$lib/ui';

	interface Props {
		data: {
			repo: Repository;
			branches: Branch[];
			commits: CommitInfo[];
			currentRef: string;
			total: number;
			offset: number;
			limit: number;
		};
	}

	let { data }: Props = $props();

	const hasMore = $derived(data.offset + data.limit < data.total);
	const hasPrev = $derived(data.offset > 0);

	function handleBranchChange(ref: string) {
		goto(`/repos/${data.repo.owner_id}/${data.repo.name}/commits/${ref}`);
	}

	function nextPage() {
		const url = new URL(window.location.href);
		url.searchParams.set('offset', String(data.offset + data.limit));
		goto(url.toString());
	}

	function prevPage() {
		const url = new URL(window.location.href);
		const newOffset = Math.max(0, data.offset - data.limit);
		if (newOffset === 0) {
			url.searchParams.delete('offset');
		} else {
			url.searchParams.set('offset', String(newOffset));
		}
		goto(url.toString());
	}
</script>

<svelte:head>
	<title>Commits - {data.repo.owner_id}/{data.repo.name}</title>
</svelte:head>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<div class="flex items-center gap-4">
			<BranchSelector
				branches={data.branches}
				currentRef={data.currentRef}
				onSelect={handleBranchChange}
			/>
			<span class="text-sm text-fg-muted">
				<strong class="text-fg">{data.total}</strong> commits
			</span>
		</div>
	</div>

	<CommitList
		commits={data.commits}
		owner={data.repo.owner_id}
		repo={data.repo.name}
	/>

	{#if hasMore || hasPrev}
		<div class="flex justify-between items-center pt-4">
			<Button variant="secondary" size="sm" disabled={!hasPrev} onclick={prevPage}>
				Newer
			</Button>
			<span class="text-sm text-fg-muted">
				{data.offset + 1}-{Math.min(data.offset + data.limit, data.total)} of {data.total}
			</span>
			<Button variant="secondary" size="sm" disabled={!hasMore} onclick={nextPage}>
				Older
			</Button>
		</div>
	{/if}
</div>
