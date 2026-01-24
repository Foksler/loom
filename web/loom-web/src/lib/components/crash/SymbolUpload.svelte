<script lang="ts">
	interface Props {
		projectId: string;
		onupload?: (files: File[], release: string) => Promise<void>;
	}

	let { projectId, onupload }: Props = $props();

	let release = $state('');
	let files = $state<File[]>([]);
	let uploading = $state(false);
	let dragOver = $state(false);
	let error = $state<string | null>(null);

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		dragOver = true;
	}

	function handleDragLeave() {
		dragOver = false;
	}

	function handleDrop(event: DragEvent) {
		event.preventDefault();
		dragOver = false;

		const droppedFiles = Array.from(event.dataTransfer?.files ?? []);
		const validFiles = droppedFiles.filter(
			(f) => f.name.endsWith('.map') || f.name.endsWith('.js')
		);
		files = [...files, ...validFiles];
	}

	function handleFileSelect(event: Event) {
		const input = event.target as HTMLInputElement;
		const selectedFiles = Array.from(input.files ?? []);
		files = [...files, ...selectedFiles];
	}

	function removeFile(index: number) {
		files = files.filter((_, i) => i !== index);
	}

	async function handleSubmit() {
		if (!release.trim() || files.length === 0) {
			error = 'Please provide a release version and at least one file';
			return;
		}

		error = null;
		uploading = true;

		try {
			await onupload?.(files, release.trim());
			files = [];
			release = '';
		} catch (e) {
			error = e instanceof Error ? e.message : 'Upload failed';
		} finally {
			uploading = false;
		}
	}

	function formatFileSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}
</script>

<div class="symbol-upload">
	<h3 class="upload-title">Upload Source Maps</h3>

	<div class="upload-form">
		<div class="form-group">
			<label class="form-label" for="release">Release Version</label>
			<input
				id="release"
				type="text"
				class="form-input"
				placeholder="e.g., 1.2.3 or commit SHA"
				bind:value={release}
			/>
		</div>

		<div
			class="drop-zone"
			class:drag-over={dragOver}
			role="button"
			tabindex="0"
			ondragover={handleDragOver}
			ondragleave={handleDragLeave}
			ondrop={handleDrop}
		>
			<span class="drop-icon">\u2191</span>
			<span class="drop-text">Drop .map and .js files here</span>
			<span class="drop-text-alt">or</span>
			<label class="file-input-label">
				Browse files
				<input
					type="file"
					class="file-input"
					accept=".map,.js"
					multiple
					onchange={handleFileSelect}
				/>
			</label>
		</div>

		{#if files.length > 0}
			<ul class="file-list">
				{#each files as file, index (index)}
					<li class="file-item">
						<span class="file-name">{file.name}</span>
						<span class="file-size">{formatFileSize(file.size)}</span>
						<button
							type="button"
							class="remove-btn"
							onclick={() => removeFile(index)}
							title="Remove file"
						>
							\u2716
						</button>
					</li>
				{/each}
			</ul>
		{/if}

		{#if error}
			<div class="error-message">{error}</div>
		{/if}

		<button
			type="button"
			class="upload-btn"
			disabled={uploading || files.length === 0 || !release.trim()}
			onclick={handleSubmit}
		>
			{#if uploading}
				Uploading...
			{:else}
				Upload {files.length} file{files.length !== 1 ? 's' : ''}
			{/if}
		</button>
	</div>
</div>

<style>
	.symbol-upload {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.upload-title {
		margin: 0;
		padding: var(--space-3) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
		background: var(--color-bg-subtle);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.upload-form {
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.form-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.form-input {
		padding: var(--space-2) var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		background: var(--color-bg);
		color: var(--color-fg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
	}

	.form-input:focus {
		outline: 2px solid var(--color-accent);
		outline-offset: -1px;
	}

	.drop-zone {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		padding: var(--space-6);
		border: 2px dashed var(--color-border-muted);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition:
			border-color 0.15s ease,
			background-color 0.15s ease;
	}

	.drop-zone:hover,
	.drop-zone.drag-over {
		border-color: var(--color-accent);
		background: var(--color-accent-soft);
	}

	.drop-icon {
		font-size: var(--text-2xl);
		color: var(--color-fg-muted);
	}

	.drop-text {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.drop-text-alt {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-subtle);
	}

	.file-input-label {
		padding: var(--space-2) var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		background: var(--color-accent);
		color: var(--color-bg);
		border-radius: var(--radius-sm);
		cursor: pointer;
	}

	.file-input {
		display: none;
	}

	.file-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.file-item {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg-subtle);
		border-radius: var(--radius-sm);
	}

	.file-name {
		flex: 1;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-size {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.remove-btn {
		padding: var(--space-1);
		font-size: var(--text-xs);
		background: transparent;
		color: var(--color-fg-muted);
		border: none;
		cursor: pointer;
	}

	.remove-btn:hover {
		color: var(--color-error);
	}

	.error-message {
		padding: var(--space-2) var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		background: var(--color-error-soft);
		color: var(--color-error);
		border-radius: var(--radius-sm);
	}

	.upload-btn {
		padding: var(--space-2) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		background: var(--color-accent);
		color: var(--color-bg);
		border: none;
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: opacity 0.15s ease;
	}

	.upload-btn:hover:not(:disabled) {
		opacity: 0.9;
	}

	.upload-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
