<script lang="ts">
	interface UserContextData {
		id?: string;
		email?: string;
		username?: string;
		ip_address?: string;
	}

	interface Props {
		user: UserContextData;
		title?: string;
	}

	let { user, title = 'User' }: Props = $props();

	const displayFields = $derived([
		{ label: 'ID', value: user.id },
		{ label: 'Email', value: user.email },
		{ label: 'Username', value: user.username },
		{ label: 'IP Address', value: user.ip_address },
	].filter(f => f.value));
</script>

<div class="user-context">
	<h3 class="user-title">{title}</h3>

	{#if displayFields.length === 0}
		<div class="user-empty">No user context available</div>
	{:else}
		<dl class="user-fields">
			{#each displayFields as field}
				<div class="field">
					<dt class="field-label">{field.label}</dt>
					<dd class="field-value">{field.value}</dd>
				</div>
			{/each}
		</dl>
	{/if}
</div>

<style>
	.user-context {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.user-title {
		margin: 0;
		padding: var(--space-3) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
		background: var(--color-bg-subtle);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.user-fields {
		margin: 0;
		padding: var(--space-3) var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.field {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.field-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.field-value {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg);
		margin: 0;
	}

	.user-empty {
		padding: var(--space-4);
		text-align: center;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
