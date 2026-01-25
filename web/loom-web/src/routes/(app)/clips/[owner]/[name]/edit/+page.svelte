<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { goto } from '$app/navigation';
	import {
		getClipsClient,
		type Clip,
		type ClipFile,
		type ClipVisibility,
		type UpdateClipRequest,
		type CreateClipFile,
	} from '$lib/api/clips';
	import { Button } from '$lib/ui';
	import { showNotification } from '$lib/components/notifications';
	import { trackButtonClick, trackFilterChange } from '$lib/analytics';

	const clipsClient = getClipsClient();

	let clip = $state<Clip | null>(null);
	let files = $state<ClipFile[]>([]);
	let loading = $state(true);
	let saving = $state(false);
	let error = $state<string | null>(null);

	// Editable fields
	let name = $state('');
	let description = $state('');
	let visibility = $state<ClipVisibility>('private');
	let commitMessage = $state('Update files');

	// File editor state
	interface EditableFile {
		path: string;
		content: string;
		isNew: boolean;
		isDeleted: boolean;
	}
	let editableFiles = $state<EditableFile[]>([]);
	let selectedFileIndex = $state(0);
	let newFileName = $state('');
	let showNewFileInput = $state(false);

	const owner = $derived($page.params.owner);
	const clipName = $derived($page.params.name);

	$effect(() => {
		if (browser && owner && clipName) {
			loadClip();
		}
	});

	async function loadClip() {
		loading = true;
		error = null;
		try {
			clip = await clipsClient.getClipByOwnerName(owner, clipName);
			name = clip.name;
			description = clip.description || '';
			visibility = clip.visibility;

			const filesResponse = await clipsClient.listClipFiles(clip.id);
			files = filesResponse.files;
			editableFiles = files.map((f) => ({
				path: f.path,
				content: f.content,
				isNew: false,
				isDeleted: false,
			}));
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load clip';
		} finally {
			loading = false;
		}
	}

	async function handleSaveMetadata() {
		if (!clip) return;
		saving = true;
		try {
			const updates: UpdateClipRequest = {};
			if (name !== clip.name) updates.name = name;
			if (description !== (clip.description || '')) updates.description = description;
			if (visibility !== clip.visibility) updates.visibility = visibility;

			if (Object.keys(updates).length > 0) {
				trackButtonClick('save_clip_metadata', { clip_id: clip.id });
				const updated = await clipsClient.updateClip(clip.id, updates);
				clip = updated;
				showNotification({
					type: 'success',
					title: 'Clip updated',
					message: 'Clip metadata has been saved.',
				});
				// If name changed, redirect to new URL
				if (updates.name) {
					await goto(`/clips/${clip.owner}/${clip.name}/edit`);
				}
			}
		} catch (e) {
			showNotification({
				type: 'error',
				title: 'Failed to save',
				message: e instanceof Error ? e.message : 'An error occurred',
			});
		} finally {
			saving = false;
		}
	}

	async function handleSaveFiles() {
		if (!clip) return;
		saving = true;
		try {
			const filesToSave: CreateClipFile[] = editableFiles
				.filter((f) => !f.isDeleted)
				.map((f) => ({
					path: f.path,
					content: f.content,
				}));

			trackButtonClick('save_clip_files', { clip_id: clip.id, file_count: filesToSave.length });
			await clipsClient.updateClipFiles(clip.id, {
				files: filesToSave,
				message: commitMessage,
			});

			showNotification({
				type: 'success',
				title: 'Files saved',
				message: `${filesToSave.length} file(s) committed.`,
			});

			// Reload files to get updated state
			const filesResponse = await clipsClient.listClipFiles(clip.id);
			files = filesResponse.files;
			editableFiles = files.map((f) => ({
				path: f.path,
				content: f.content,
				isNew: false,
				isDeleted: false,
			}));
			commitMessage = 'Update files';
		} catch (e) {
			showNotification({
				type: 'error',
				title: 'Failed to save files',
				message: e instanceof Error ? e.message : 'An error occurred',
			});
		} finally {
			saving = false;
		}
	}

	function addNewFile() {
		const trimmedName = newFileName.trim();
		if (!trimmedName) return;

		if (editableFiles.some((f) => f.path === trimmedName && !f.isDeleted)) {
			showNotification({
				type: 'error',
				title: 'File exists',
				message: `A file named "${trimmedName}" already exists.`,
			});
			return;
		}

		trackButtonClick('add_clip_file', { clip_id: clip?.id });
		editableFiles = [
			...editableFiles,
			{
				path: trimmedName,
				content: '',
				isNew: true,
				isDeleted: false,
			},
		];
		selectedFileIndex = editableFiles.length - 1;
		newFileName = '';
		showNewFileInput = false;
	}

	function deleteFile(index: number) {
		const file = editableFiles[index];
		trackButtonClick('delete_clip_file', { clip_id: clip?.id, path: file.path });

		if (file.isNew) {
			// New file - just remove it
			editableFiles = editableFiles.filter((_, i) => i !== index);
			if (selectedFileIndex >= editableFiles.length) {
				selectedFileIndex = Math.max(0, editableFiles.length - 1);
			}
		} else {
			// Existing file - mark as deleted
			editableFiles[index] = { ...file, isDeleted: true };
			editableFiles = [...editableFiles];
		}
	}

	function restoreFile(index: number) {
		const file = editableFiles[index];
		trackButtonClick('restore_clip_file', { clip_id: clip?.id, path: file.path });
		editableFiles[index] = { ...file, isDeleted: false };
		editableFiles = [...editableFiles];
	}

	function handleCancel() {
		trackButtonClick('cancel_edit_clip', { clip_id: clip?.id });
		goto(`/clips/${owner}/${clipName}`);
	}

	function handleVisibilityChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		visibility = target.value as ClipVisibility;
		trackFilterChange('clip_visibility', visibility, { page: 'clip_edit' });
	}

	const hasMetadataChanges = $derived(
		clip !== null &&
			(name !== clip.name ||
				description !== (clip.description || '') ||
				visibility !== clip.visibility)
	);

	const hasFileChanges = $derived(
		editableFiles.some(
			(f, i) =>
				f.isNew || f.isDeleted || (files[i] && f.content !== files[i].content)
		)
	);

	const activeFiles = $derived(editableFiles.filter((f) => !f.isDeleted));
</script>

<div class="edit-page">
	{#if loading}
		<div class="loading">Loading clip...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if clip}
		<header class="page-header">
			<div class="header-content">
				<div>
					<h1 class="page-title">Edit: {clip.name}</h1>
					<p class="page-description">Modify clip settings and files</p>
				</div>
				<Button variant="secondary" onclick={handleCancel}>Back to Clip</Button>
			</div>
		</header>

		<section class="metadata-section">
			<h2 class="section-title">Metadata</h2>
			<div class="metadata-form">
				<div class="form-group">
					<label for="edit-name" class="form-label">Name</label>
					<input
						id="edit-name"
						type="text"
						class="form-input"
						bind:value={name}
						disabled={saving}
					/>
				</div>

				<div class="form-group">
					<label for="edit-description" class="form-label">Description</label>
					<textarea
						id="edit-description"
						class="form-textarea"
						bind:value={description}
						rows="3"
						disabled={saving}
					></textarea>
				</div>

				<div class="form-group">
					<label for="edit-visibility" class="form-label">Visibility</label>
					<select
						id="edit-visibility"
						class="form-select"
						value={visibility}
						onchange={handleVisibilityChange}
						disabled={saving}
					>
						<option value="private">Private</option>
						<option value="internal">Internal (org members)</option>
						<option value="public">Public</option>
					</select>
				</div>

				<div class="form-actions">
					<Button
						variant="primary"
						onclick={handleSaveMetadata}
						disabled={saving || !hasMetadataChanges}
					>
						{saving ? 'Saving...' : 'Save Metadata'}
					</Button>
				</div>
			</div>
		</section>

		<section class="files-section">
			<div class="files-header">
				<h2 class="section-title">Files</h2>
				<Button
					variant="secondary"
					size="sm"
					onclick={() => (showNewFileInput = true)}
					disabled={saving}
				>
					+ Add File
				</Button>
			</div>

			{#if showNewFileInput}
				<div class="new-file-input">
					<input
						type="text"
						class="form-input"
						placeholder="filename.ext"
						bind:value={newFileName}
						onkeydown={(e) => e.key === 'Enter' && addNewFile()}
					/>
					<Button size="sm" onclick={addNewFile}>Add</Button>
					<Button variant="secondary" size="sm" onclick={() => (showNewFileInput = false)}>
						Cancel
					</Button>
				</div>
			{/if}

			<div class="files-editor">
				<div class="file-tabs">
					{#each editableFiles as file, index (file.path)}
						<button
							class="file-tab"
							class:active={selectedFileIndex === index}
							class:deleted={file.isDeleted}
							class:new={file.isNew}
							onclick={() => (selectedFileIndex = index)}
						>
							<span class="tab-name">{file.path}</span>
							{#if file.isNew}
								<span class="tab-badge new">new</span>
							{/if}
							{#if file.isDeleted}
								<span class="tab-badge deleted">deleted</span>
							{/if}
						</button>
					{/each}
				</div>

				{#if editableFiles.length > 0 && editableFiles[selectedFileIndex]}
					{@const currentFile = editableFiles[selectedFileIndex]}
					<div class="editor-container">
						<div class="editor-header">
							<span class="editor-path">{currentFile.path}</span>
							<div class="editor-actions">
								{#if currentFile.isDeleted}
									<Button
										variant="secondary"
										size="sm"
										onclick={() => restoreFile(selectedFileIndex)}
									>
										Restore
									</Button>
								{:else}
									<Button
										variant="danger"
										size="sm"
										onclick={() => deleteFile(selectedFileIndex)}
									>
										Delete
									</Button>
								{/if}
							</div>
						</div>
						{#if currentFile.isDeleted}
							<div class="deleted-overlay">
								<p>This file will be deleted on save.</p>
							</div>
						{:else}
							<textarea
								class="editor-textarea"
								bind:value={editableFiles[selectedFileIndex].content}
								disabled={saving}
								spellcheck="false"
							></textarea>
						{/if}
					</div>
				{:else}
					<div class="no-files">
						<p>No files yet. Add a file to get started.</p>
					</div>
				{/if}
			</div>

			<div class="commit-section">
				<div class="form-group">
					<label for="commit-message" class="form-label">Commit message</label>
					<input
						id="commit-message"
						type="text"
						class="form-input"
						bind:value={commitMessage}
						placeholder="Describe your changes"
						disabled={saving}
					/>
				</div>
				<div class="form-actions">
					<Button
						variant="primary"
						onclick={handleSaveFiles}
						disabled={saving || !hasFileChanges}
					>
						{saving ? 'Saving...' : 'Commit Changes'}
					</Button>
				</div>
			</div>
		</section>
	{/if}
</div>

<style>
	.edit-page {
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
		gap: var(--space-2);
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}

	.page-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-xl);
		font-weight: 700;
		color: var(--color-fg);
	}

	.page-description {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.metadata-section,
	.files-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-6);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.section-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.metadata-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.form-label {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-fg);
	}

	.form-input,
	.form-textarea,
	.form-select {
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
	}

	.form-input:focus,
	.form-textarea:focus,
	.form-select:focus {
		outline: none;
		border-color: var(--color-accent);
	}

	.form-textarea {
		resize: vertical;
		min-height: 80px;
	}

	.form-actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-3);
	}

	.files-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.new-file-input {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3);
		background: var(--color-bg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.new-file-input .form-input {
		flex: 1;
	}

	.files-editor {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		overflow: hidden;
		min-height: 400px;
	}

	.file-tabs {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		padding: var(--space-2);
		background: var(--color-bg-subtle);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.file-tab {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-2) var(--space-3);
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
		cursor: pointer;
		transition: all 0.15s ease;
	}

	.file-tab:hover {
		background: var(--color-bg);
		color: var(--color-fg);
	}

	.file-tab.active {
		background: var(--color-bg);
		border-color: var(--color-border-muted);
		color: var(--color-fg);
	}

	.file-tab.deleted {
		text-decoration: line-through;
		opacity: 0.6;
	}

	.tab-badge {
		padding: 0 var(--space-1);
		font-size: var(--text-xs);
		border-radius: var(--radius-sm);
	}

	.tab-badge.new {
		background: color-mix(in srgb, var(--color-success) 20%, transparent);
		color: var(--color-success);
	}

	.tab-badge.deleted {
		background: color-mix(in srgb, var(--color-error) 20%, transparent);
		color: var(--color-error);
	}

	.editor-container {
		flex: 1;
		display: flex;
		flex-direction: column;
		background: var(--color-bg);
	}

	.editor-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg-muted);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.editor-path {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
	}

	.editor-actions {
		display: flex;
		gap: var(--space-2);
	}

	.editor-textarea {
		flex: 1;
		width: 100%;
		padding: var(--space-4);
		background: var(--color-bg);
		border: none;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		line-height: 1.6;
		color: var(--color-fg);
		resize: none;
		tab-size: 4;
	}

	.editor-textarea:focus {
		outline: none;
	}

	.deleted-overlay {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		background: color-mix(in srgb, var(--color-error) 5%, var(--color-bg));
	}

	.deleted-overlay p {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-error);
	}

	.no-files {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-8);
		background: var(--color-bg);
	}

	.no-files p {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.commit-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding-top: var(--space-4);
		border-top: 1px solid var(--color-border-muted);
	}
</style>
