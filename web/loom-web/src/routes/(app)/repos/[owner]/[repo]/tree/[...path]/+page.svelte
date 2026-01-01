<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import type { Repository, Branch, TreeEntry } from '$lib/api/repos';
	import { TreeView, BranchSelector, OpenInWeaverButton } from '$lib/components/repos';
	import { Skeleton } from '$lib/ui';

	interface Props {
		data: {
			repo: Repository;
			branches: Branch[];
			entries: TreeEntry[];
			currentRef: string;
			currentPath: string;
		};
	}

	let { data }: Props = $props();

	function handleBranchChange(ref: string) {
		const path = data.currentPath ? `/${data.currentPath}` : '';
		goto(`/repos/${data.repo.owner_id}/${data.repo.name}/tree/${ref}${path}`);
	}
</script>

<svelte:head>
	<title>{data.currentPath || data.repo.name} - {data.repo.owner_id}/{data.repo.name}</title>
</svelte:head>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<div class="flex items-center gap-4">
			<BranchSelector
				branches={data.branches}
				currentRef={data.currentRef}
				onSelect={handleBranchChange}
			/>
		</div>
		<OpenInWeaverButton repo={data.repo} />
	</div>

	<TreeView
		entries={data.entries}
		owner={data.repo.owner_id}
		repo={data.repo.name}
		currentRef={data.currentRef}
		currentPath={data.currentPath}
	/>
</div>
