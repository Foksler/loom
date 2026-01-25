<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { goto } from '$app/navigation';
	import { getClipsClient, type Clip, type ClipRevision, type ClipFile } from '$lib/api/clips';
	import { ClipRevisionList, ClipFileView } from '$lib/components/clips';
	import { Button } from '$lib/ui';
	import { trackButtonClick, trackLinkClick } from '$lib/analytics';

	const clipsClient = getClipsClient();

	let clip = $state<Clip | null>(null);
	let revisions = $state<ClipRevision[]>([]);
	let selectedRevision = $state<ClipRevision | null>(null);
	let revisionFiles = $state<ClipFile[]>([]);
	let loading = $state(true);
	let loadingFiles = $state(false);
	let error = $state<string | null>(null);

	const owner = $derived($page.params.owner);
	const clipName = $derived($page.params.name);

	$effect(() => {
		if (browser && owner && clipName) {
			loadClipAndRevisions();
		}
	});

	async function loadClipAndRevisions() {
		loading = true;
		error = null;
		try {
			clip = await clipsClient.getClipByOwnerName(owner, clipName);
			const response = await clipsClient.listClipRevisions(clip.id, 50);
			revisions = response.revisions;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load revisions';
		} finally {
			loading = false;
		}
	}

	async function handleSelectRevision(revision: ClipRevision) {
		if (!clip) return;
		trackButtonClick('select_revision', { clip_id: clip.id, sha: revision.sha });
		selectedRevision = revision;
		loadingFiles = true;
		try {
			const response = await clipsClient.listClipFiles(clip.id, revision.sha);
			revisionFiles = response.files;
		} catch (e) {
			console.error('Failed to load revision files:', e);
			revisionFiles = [];
		} finally {
			loadingFiles = false;
		}
	}

	function handleBack() {
		trackLinkClick('back_to_clip', `/clips/${owner}/${clipName}`);
		goto(`/clips/${owner}/${clipName}`);
	}

	function clearSelection() {
		selectedRevision = null;
		revisionFiles = [];
	}
</script>

<div class="revisions-page">
	{#if loading}
		<div class="loading">Loading revisions...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if clip}
		<header class="page-header">
			<div class="header-content">
				<div class="header-title">
					<a
						href="/clips/{owner}/{clipName}"
						class="breadcrumb"
						onclick={() => trackLinkClick('clip_breadcrumb', `/clips/${owner}/${clipName}`)}
					>
						{clip.owner}/{clip.name}
					</a>
					<h1 class="page-title">Revisions</h1>
				</div>
				<Button variant="secondary" onclick={handleBack}>Back to Clip</Button>
			</div>
		</header>

		<div class="content-layout">
			<div class="revisions-panel">
				<h2 class="panel-title">Commit History</h2>
				{#if revisions.length === 0}
					<div class="empty-state">
						<p>No revisions yet. Push some commits to see history.</p>
					</div>
				{:else}
					<ClipRevisionList {revisions} onselect={handleSelectRevision} />
				{/if}
			</div>

			{#if selectedRevision}
				<div class="files-panel">
					<div class="panel-header">
						<h2 class="panel-title">
							Files at <code>{selectedRevision.sha.slice(0, 7)}</code>
						</h2>
						<Button variant="secondary" size="sm" onclick={clearSelection}>
							Close
						</Button>
					</div>
					{#if loadingFiles}
						<div class="loading">Loading files...</div>
					{:else if revisionFiles.length === 0}
						<div class="empty-state">
							<p>No files in this revision.</p>
						</div>
					{:else}
						<div class="file-list">
							{#each revisionFiles as file (file.path)}
								<ClipFileView {file} />
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.revisions-page {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		max-width: 1400px;
		margin: 0 auto;
		padding: var(--space-6);
	}

	.loading,
	.error {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		text-align: center;
		padding: var(--space-8);
	}

	.error {
		color: var(--color-error);
	}

	.page-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}

	.header-title {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.breadcrumb {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-accent);
		text-decoration: none;
	}

	.breadcrumb:hover {
		text-decoration: underline;
	}

	.page-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-xl);
		font-weight: 700;
		color: var(--color-fg);
	}

	.content-layout {
		display: grid;
		grid-template-columns: 400px 1fr;
		gap: var(--space-6);
	}

	@media (max-width: 900px) {
		.content-layout {
			grid-template-columns: 1fr;
		}
	}

	.revisions-panel,
	.files-panel {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.panel-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.panel-title code {
		padding: var(--space-1) var(--space-2);
		background: var(--color-bg-muted);
		border-radius: var(--radius-sm);
		color: var(--color-accent);
	}

	.empty-state {
		padding: var(--space-8);
		text-align: center;
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.empty-state p {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.file-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
</style>
