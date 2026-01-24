<script lang="ts">
	type AdoptionStage = 'new' | 'low' | 'adopted' | 'replaced';

	interface Props {
		stage: AdoptionStage;
		percentage?: number;
		size?: 'sm' | 'md';
	}

	let { stage, percentage, size = 'md' }: Props = $props();

	const stageConfig: Record<AdoptionStage, { label: string; variant: string }> = {
		new: { label: 'New', variant: 'info' },
		low: { label: 'Low Adoption', variant: 'warning' },
		adopted: { label: 'Adopted', variant: 'success' },
		replaced: { label: 'Replaced', variant: 'muted' },
	};

	const config = $derived(stageConfig[stage]);
</script>

<span class="adoption-stage adoption-stage-{config.variant} adoption-stage-{size}">
	{config.label}
	{#if percentage !== undefined}
		<span class="percentage">({percentage}%)</span>
	{/if}
</span>

<style>
	.adoption-stage {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		font-family: var(--font-mono);
		font-weight: 500;
		border-radius: var(--radius-sm);
	}

	.adoption-stage-sm {
		padding: 2px var(--space-1);
		font-size: 10px;
	}

	.adoption-stage-md {
		padding: var(--space-1) var(--space-2);
		font-size: var(--text-xs);
	}

	.adoption-stage-info {
		background: var(--color-info-soft);
		color: var(--color-info);
	}

	.adoption-stage-warning {
		background: var(--color-warning-soft);
		color: var(--color-warning);
	}

	.adoption-stage-success {
		background: var(--color-success-soft);
		color: var(--color-success);
	}

	.adoption-stage-muted {
		background: var(--color-bg-subtle);
		color: var(--color-fg-muted);
	}

	.percentage {
		opacity: 0.8;
	}
</style>
