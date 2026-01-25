<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import type { ClipVisibility, CreateClipRequest } from '$lib/api/clips';
	import type { Org } from '$lib/api/types';
	import { Button } from '$lib/ui';
	import { trackButtonClick, trackFilterChange } from '$lib/analytics';

	interface Props {
		orgs?: Org[];
		onsubmit: (data: CreateClipRequest) => void;
		oncancel?: () => void;
		loading?: boolean;
	}

	let { orgs = [], onsubmit, oncancel, loading = false }: Props = $props();

	let name = $state('');
	let description = $state('');
	let visibility = $state<ClipVisibility>('private');
	let selectedOrgId = $state<string | null>(null);
	let nameError = $state<string | null>(null);

	function validateName(value: string): string | null {
		if (!value) {
			return 'Name is required';
		}
		if (value.length > 100) {
			return 'Name must be 100 characters or less';
		}
		if (!/^[a-zA-Z0-9._-]+$/.test(value)) {
			return 'Name can only contain letters, numbers, dots, hyphens, and underscores';
		}
		return null;
	}

	function handleNameInput() {
		nameError = validateName(name);
	}

	function handleSubmit() {
		nameError = validateName(name);
		if (nameError) {
			return;
		}

		trackButtonClick('create_clip_submit');

		const data: CreateClipRequest = {
			name,
			visibility,
		};

		if (description.trim()) {
			data.description = description.trim();
		}

		if (selectedOrgId) {
			data.org_id = selectedOrgId;
		}

		onsubmit(data);
	}

	function handleCancel() {
		trackButtonClick('create_clip_cancel');
		oncancel?.();
	}

	function handleVisibilityChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		visibility = target.value as ClipVisibility;
		trackFilterChange('clip_visibility', visibility, { page: 'clip_form' });
	}

	function handleOrgChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		selectedOrgId = target.value || null;
		trackFilterChange('clip_org', selectedOrgId || 'personal', { page: 'clip_form' });
	}
</script>

<form class="clip-form" onsubmit={e => { e.preventDefault(); handleSubmit(); }}>
	<div class="form-group">
		<label for="clip-name" class="form-label">Name</label>
		<input
			id="clip-name"
			type="text"
			class="form-input"
			class:error={nameError}
			bind:value={name}
			oninput={handleNameInput}
			placeholder="my-code-snippet"
			disabled={loading}
		/>
		{#if nameError}
			<p class="error-message">{nameError}</p>
		{/if}
	</div>

	<div class="form-group">
		<label for="clip-description" class="form-label">Description (optional)</label>
		<textarea
			id="clip-description"
			class="form-textarea"
			bind:value={description}
			placeholder="A brief description of this clip..."
			rows="3"
			disabled={loading}
		></textarea>
	</div>

	<div class="form-row">
		<div class="form-group">
			<label for="clip-visibility" class="form-label">Visibility</label>
			<select
				id="clip-visibility"
				class="form-select"
				value={visibility}
				onchange={handleVisibilityChange}
				disabled={loading}
			>
				<option value="private">Private</option>
				<option value="internal">Internal (org members)</option>
				<option value="public">Public</option>
			</select>
		</div>

		{#if orgs.length > 0}
			<div class="form-group">
				<label for="clip-owner" class="form-label">Owner</label>
				<select
					id="clip-owner"
					class="form-select"
					value={selectedOrgId || ''}
					onchange={handleOrgChange}
					disabled={loading}
				>
					<option value="">Personal</option>
					{#each orgs as org}
						<option value={org.id}>{org.name}</option>
					{/each}
				</select>
			</div>
		{/if}
	</div>

	<div class="form-actions">
		{#if oncancel}
			<Button variant="secondary" onclick={handleCancel} disabled={loading}>
				Cancel
			</Button>
		{/if}
		<Button type="submit" disabled={loading || !!nameError}>
			{loading ? 'Creating...' : 'Create Clip'}
		</Button>
	</div>
</form>

<style>
	.clip-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.form-row {
		display: flex;
		gap: var(--space-4);
	}

	.form-row .form-group {
		flex: 1;
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
		transition: border-color 0.15s ease;
	}

	.form-input:focus,
	.form-textarea:focus,
	.form-select:focus {
		outline: none;
		border-color: var(--color-accent);
	}

	.form-input.error {
		border-color: var(--color-error);
	}

	.form-textarea {
		resize: vertical;
		min-height: 80px;
	}

	.error-message {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-error);
	}

	.form-actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-3);
		padding-top: var(--space-2);
	}
</style>
