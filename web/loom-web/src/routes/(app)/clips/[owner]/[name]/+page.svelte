<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { goto } from '$app/navigation';
	import { getClipsClient, type Clip, type ClipFile } from '$lib/api/clips';
	import { getApiClient } from '$lib/api/client';
	import type { CurrentUser, Org } from '$lib/api/types';
	import { ClipFileView, ClipVisibilityBadge } from '$lib/components/clips';
	import { Button } from '$lib/ui';
	import { showNotification } from '$lib/components/notifications';
	import { trackButtonClick, trackLinkClick } from '$lib/analytics';

	const clipsClient = getClipsClient();
	const apiClient = getApiClient();

	let currentUser = $state<CurrentUser | null>(null);
	let orgs = $state<Org[]>([]);
	let clip = $state<Clip | null>(null);
	let files = $state<ClipFile[]>([]);
	let starred = $state(false);
	let starCount = $state(0);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let deleting = $state(false);
	let showDeleteConfirm = $state(false);
	let showForkModal = $state(false);
	let selectedForkOrgId = $state<string>('');

	const owner = $derived($page.params.owner);
	const name = $derived($page.params.name);

	$effect(() => {
		if (browser) {
			loadCurrentUser();
			loadOrgs();
		}
	});

	$effect(() => {
		if (browser && owner && name) {
			loadClip();
		}
	});

	async function loadCurrentUser() {
		try {
			currentUser = await apiClient.getCurrentUser();
		} catch (e) {
			console.error('Failed to load current user:', e);
		}
	}

	async function loadOrgs() {
		try {
			const response = await apiClient.listOrgs();
			orgs = response.orgs;
			if (orgs.length > 0) {
				selectedForkOrgId = orgs[0].id;
			}
		} catch (e) {
			console.error('Failed to load organizations:', e);
		}
	}

	async function loadClip() {
		loading = true;
		error = null;
		try {
			clip = await clipsClient.getClipByOwnerName(owner, name);
			const filesResponse = await clipsClient.listClipFiles(clip.id);
			files = filesResponse.files;
			starCount = clip.star_count;
			// Check if user has starred this clip
			try {
				const starStatus = await clipsClient.getClipStarStatus(clip.id);
				starred = starStatus.starred;
			} catch {
				starred = false;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load clip';
		} finally {
			loading = false;
		}
	}

	async function handleDelete() {
		if (!clip) return;
		deleting = true;
		try {
			await clipsClient.deleteClip(clip.id);
			showNotification({
				type: 'success',
				title: 'Clip deleted',
				message: `Clip "${clip.name}" has been deleted.`,
			});
			await goto('/clips');
		} catch (e) {
			showNotification({
				type: 'error',
				title: 'Failed to delete clip',
				message: e instanceof Error ? e.message : 'An error occurred',
			});
		} finally {
			deleting = false;
			showDeleteConfirm = false;
		}
	}

	async function handleFork() {
		if (!clip || !selectedForkOrgId) return;
		trackButtonClick('fork_clip', { clip_id: clip.id });
		try {
			const forkedClip = await clipsClient.forkClip(clip.id, {
				target_org_id: selectedForkOrgId,
			});
			showNotification({
				type: 'success',
				title: 'Clip forked',
				message: `Forked to "${forkedClip.name}".`,
			});
			showForkModal = false;
			await goto(`/clips/${forkedClip.owner}/${forkedClip.name}`);
		} catch (e) {
			showNotification({
				type: 'error',
				title: 'Failed to fork clip',
				message: e instanceof Error ? e.message : 'An error occurred',
			});
		}
	}

	async function handleStar() {
		if (!clip) return;
		trackButtonClick(starred ? 'unstar_clip' : 'star_clip', { clip_id: clip.id });
		try {
			const response = starred
				? await clipsClient.unstarClip(clip.id)
				: await clipsClient.starClip(clip.id);
			starred = response.starred;
			starCount = response.star_count;
		} catch (e) {
			showNotification({
				type: 'error',
				title: starred ? 'Failed to unstar clip' : 'Failed to star clip',
				message: e instanceof Error ? e.message : 'An error occurred',
			});
		}
	}

	function formatDate(dateString: string): string {
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
		});
	}
</script>

<div class="clip-detail-page">
	{#if loading}
		<div class="loading">Loading clip...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if clip}
		<header class="page-header">
			<div class="header-top">
				<div class="clip-identity">
					<a
						href="/clips"
						class="breadcrumb"
						onclick={() => trackLinkClick('clips_breadcrumb', '/clips')}
					>
						Clips
					</a>
					<span class="breadcrumb-separator">/</span>
					<span class="owner">{clip.owner}</span>
					<span class="breadcrumb-separator">/</span>
					<h1 class="clip-name">{clip.name}</h1>
				</div>
				<div class="header-actions">
					<Button variant="secondary" onclick={handleStar}>
						{starred ? '★' : '☆'} {starCount}
					</Button>
					<Button variant="secondary" onclick={() => (showForkModal = true)}>Fork</Button>
					<Button
						href={`/clips/${owner}/${name}/revisions`}
						variant="secondary"
						onclick={() => trackLinkClick('clip_revisions', `/clips/${owner}/${name}/revisions`)}
					>
						History
					</Button>
					<Button href={`/clips/${owner}/${name}/edit`} variant="secondary">Edit</Button>
					<Button
						variant="danger"
						onclick={() => {
							trackButtonClick('delete_clip_start', { clip_id: clip?.id });
							showDeleteConfirm = true;
						}}
					>
						Delete
					</Button>
				</div>
			</div>

			<div class="clip-meta">
				<ClipVisibilityBadge visibility={clip.visibility} />
				{#if clip.language}
					<span class="language-badge">{clip.language}</span>
				{/if}
				<span class="meta-item">Created {formatDate(clip.created_at)}</span>
				{#if clip.forked_from}
					<span class="meta-item">Forked from {clip.forked_from}</span>
				{/if}
			</div>

			{#if clip.description}
				<p class="clip-description">{clip.description}</p>
			{/if}
		</header>

		<section class="clone-section">
			<h2 class="section-title">Clone</h2>
			<div class="clone-url">
				<code>{clip.clone_url}</code>
				<Button
					variant="secondary"
					size="sm"
					onclick={() => {
						trackButtonClick('copy_clone_url', { clip_id: clip?.id });
						navigator.clipboard.writeText(clip?.clone_url ?? '');
						showNotification({
							type: 'success',
							title: 'Copied',
							message: 'Clone URL copied to clipboard',
						});
					}}
				>
					Copy
				</Button>
			</div>
		</section>

		<section class="files-section">
			<h2 class="section-title">Files ({files.length})</h2>
			{#if files.length === 0}
				<div class="empty-files">
					<p>No files yet. Push some code to get started:</p>
					<pre><code>git clone {clip.clone_url}
cd {clip.name}
# Add your files
git add .
git commit -m "Add initial files"
git push origin main</code></pre>
				</div>
			{:else}
				<div class="file-list">
					{#each files as file (file.path)}
						<ClipFileView {file} />
					{/each}
				</div>
			{/if}
		</section>

		{#if showDeleteConfirm}
			<div class="modal-overlay" onclick={() => (showDeleteConfirm = false)}>
				<div class="modal" onclick={(e) => e.stopPropagation()}>
					<h3 class="modal-title">Delete Clip</h3>
					<p class="modal-message">
						Are you sure you want to delete "{clip.name}"? This action cannot be undone.
					</p>
					<div class="modal-actions">
						<Button variant="secondary" onclick={() => (showDeleteConfirm = false)} disabled={deleting}>
							Cancel
						</Button>
						<Button variant="danger" onclick={handleDelete} disabled={deleting}>
							{deleting ? 'Deleting...' : 'Delete'}
						</Button>
					</div>
				</div>
			</div>
		{/if}

		{#if showForkModal}
			<div class="modal-overlay" onclick={() => (showForkModal = false)}>
				<div class="modal" onclick={(e) => e.stopPropagation()}>
					<h3 class="modal-title">Fork Clip</h3>
					<p class="modal-message">
						Select an organization to fork "{clip.name}" to:
					</p>
					<div class="form-group">
						<label for="fork-org" class="form-label">Organization</label>
						<select
							id="fork-org"
							class="form-select"
							bind:value={selectedForkOrgId}
						>
							{#each orgs as org (org.id)}
								<option value={org.id}>{org.display_name || org.slug}</option>
							{/each}
						</select>
					</div>
					<div class="modal-actions">
						<Button variant="secondary" onclick={() => (showForkModal = false)}>
							Cancel
						</Button>
						<Button variant="primary" onclick={handleFork} disabled={!selectedForkOrgId}>
							Fork
						</Button>
					</div>
				</div>
			</div>
		{/if}
	{/if}
</div>

<style>
	.clip-detail-page {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		max-width: 1000px;
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
		gap: var(--space-3);
	}

	.header-top {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--space-4);
	}

	.clip-identity {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
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

	.breadcrumb-separator {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.owner {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.clip-name {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-xl);
		font-weight: 700;
		color: var(--color-fg);
	}

	.header-actions {
		display: flex;
		gap: var(--space-2);
	}

	.clip-meta {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
	}

	.language-badge {
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: color-mix(in srgb, var(--color-accent) 15%, transparent);
		color: var(--color-accent);
		border-radius: var(--radius-sm);
	}

	.meta-item {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.clip-description {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.clone-section,
	.files-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.section-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.clone-url {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.clone-url code {
		flex: 1;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
		overflow-x: auto;
	}

	.empty-files {
		padding: var(--space-4);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.empty-files p {
		margin: 0 0 var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.empty-files pre {
		margin: 0;
		padding: var(--space-3);
		background: var(--color-bg);
		border-radius: var(--radius-sm);
		overflow-x: auto;
	}

	.empty-files code {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
	}

	.file-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: var(--space-6);
		max-width: 400px;
		width: 90%;
	}

	.modal-title {
		margin: 0 0 var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-fg);
	}

	.modal-message {
		margin: 0 0 var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-3);
	}

	.form-group {
		margin-bottom: var(--space-4);
	}

	.form-label {
		display: block;
		margin-bottom: var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-fg);
	}

	.form-select {
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
	}
</style>
