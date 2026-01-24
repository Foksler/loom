<script lang="ts">
	interface Props {
		value: string;
		label?: string;
		size?: 'sm' | 'md';
		variant?: 'default' | 'ghost';
	}

	let { value, label, size = 'md', variant = 'default' }: Props = $props();

	let copied = $state(false);
	let timeoutId: ReturnType<typeof setTimeout> | undefined;

	async function copyToClipboard() {
		try {
			await navigator.clipboard.writeText(value);
			copied = true;

			if (timeoutId) {
				clearTimeout(timeoutId);
			}

			timeoutId = setTimeout(() => {
				copied = false;
			}, 2000);
		} catch {
			console.error('Failed to copy to clipboard');
		}
	}
</script>

<button
	type="button"
	class="copy-button copy-button-{size} copy-button-{variant}"
	onclick={copyToClipboard}
	title={copied ? 'Copied!' : 'Copy to clipboard'}
>
	{#if copied}
		<svg
			class="copy-icon"
			xmlns="http://www.w3.org/2000/svg"
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<polyline points="20 6 9 17 4 12"></polyline>
		</svg>
	{:else}
		<svg
			class="copy-icon"
			xmlns="http://www.w3.org/2000/svg"
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
			<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
		</svg>
	{/if}
	{#if label}
		<span class="copy-label">{label}</span>
	{/if}
</button>

<style>
	.copy-button {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		border: none;
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			color 0.15s ease;
	}

	.copy-button-sm {
		padding: var(--space-1);
	}

	.copy-button-md {
		padding: var(--space-1) var(--space-2);
	}

	.copy-button-default {
		background: var(--color-bg-subtle);
		color: var(--color-fg-muted);
	}

	.copy-button-default:hover {
		background: var(--color-bg-muted);
		color: var(--color-fg);
	}

	.copy-button-ghost {
		background: transparent;
		color: var(--color-fg-muted);
	}

	.copy-button-ghost:hover {
		background: var(--color-bg-subtle);
		color: var(--color-fg);
	}

	.copy-icon {
		width: 14px;
		height: 14px;
	}

	.copy-button-sm .copy-icon {
		width: 12px;
		height: 12px;
	}

	.copy-label {
		font-size: var(--text-xs);
	}
</style>
