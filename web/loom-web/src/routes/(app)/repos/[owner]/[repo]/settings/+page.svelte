<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { Repository } from '$lib/api/repos';
	import { Card, Button, Input, Badge } from '$lib/ui';

	interface Props {
		data: {
			repo: Repository;
		};
	}

	let { data }: Props = $props();

	let repoName = $state('');
	let defaultBranch = $state('');
	let visibility = $state<'private' | 'public'>('private');

	$effect(() => {
		repoName = data.repo.name;
		defaultBranch = data.repo.default_branch;
		visibility = data.repo.visibility;
	});
	let saving = $state(false);
	let deleting = $state(false);
	let showDeleteConfirm = $state(false);
</script>

<svelte:head>
	<title>Settings - {data.repo.owner_id}/{data.repo.name}</title>
</svelte:head>

<div class="space-y-8 max-w-2xl">
	<Card>
		<h2 class="text-lg font-medium text-fg mb-4">General</h2>

		<div class="space-y-4">
			<Input
				label="Repository name"
				bind:value={repoName}
			/>

			<div>
				<label for="default-branch" class="block text-sm font-medium text-fg mb-1.5">
					Default branch
				</label>
				<select
					id="default-branch"
					bind:value={defaultBranch}
					class="w-full h-10 px-3 rounded-md border border-border bg-bg text-fg"
				>
					<option value={data.repo.default_branch}>{data.repo.default_branch}</option>
				</select>
			</div>

			<fieldset>
				<legend class="block text-sm font-medium text-fg mb-1.5">
					Visibility
				</legend>
				<div class="flex gap-4">
					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="radio"
							name="visibility"
							value="private"
							bind:group={visibility}
							class="text-accent"
						/>
						<span class="text-sm text-fg">Private</span>
					</label>
					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="radio"
							name="visibility"
							value="public"
							bind:group={visibility}
							class="text-accent"
						/>
						<span class="text-sm text-fg">Public</span>
					</label>
				</div>
				<p class="text-xs text-fg-muted mt-1">
					{visibility === 'public'
						? 'Anyone can view and clone this repository.'
						: 'Only users with explicit access can view and clone.'}
				</p>
			</fieldset>

			<div class="pt-4">
				<Button variant="primary" loading={saving} disabled={saving}>
					Save changes
				</Button>
			</div>
		</div>
	</Card>

	<Card>
		<h2 class="text-lg font-medium text-fg mb-4">Clone URL</h2>
		<div class="flex gap-2">
			<input
				type="text"
				readonly
				value={data.repo.clone_url}
				class="flex-1 font-mono text-sm bg-bg-muted border border-border rounded px-3 py-2"
			/>
			<Button variant="secondary" onclick={() => navigator.clipboard.writeText(data.repo.clone_url)}>
				Copy
			</Button>
		</div>
	</Card>

	<Card>
		<h2 class="text-lg font-medium text-error mb-4">Danger Zone</h2>

		<div class="border border-error/30 rounded-lg p-4">
			<div class="flex items-start justify-between gap-4">
				<div>
					<h3 class="font-medium text-fg">Delete this repository</h3>
					<p class="text-sm text-fg-muted mt-1">
						Once you delete a repository, there is no going back. Please be certain.
					</p>
				</div>
				<Button variant="danger" onclick={() => (showDeleteConfirm = true)}>
					Delete repository
				</Button>
			</div>
		</div>

		{#if showDeleteConfirm}
			<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
				<div class="bg-bg border border-border rounded-lg p-6 max-w-md w-full mx-4">
					<h3 class="text-lg font-medium text-fg mb-4">Delete repository?</h3>
					<p class="text-sm text-fg-muted mb-4">
						This will permanently delete <strong class="text-fg">{data.repo.owner_id}/{data.repo.name}</strong> and all of its contents including commits, branches, and settings.
					</p>
					<div class="flex justify-end gap-2">
						<Button variant="secondary" onclick={() => (showDeleteConfirm = false)}>
							Cancel
						</Button>
						<Button variant="danger" loading={deleting} disabled={deleting}>
							Yes, delete this repository
						</Button>
					</div>
				</div>
			</div>
		{/if}
	</Card>
</div>
