<script lang="ts">
	interface Props {
		value: string;
		onchange?: (value: string) => void;
		error?: string;
	}

	let { value, onchange, error }: Props = $props();

	const presets = [
		{ label: 'Every minute', value: '* * * * *' },
		{ label: 'Every 5 minutes', value: '*/5 * * * *' },
		{ label: 'Every 15 minutes', value: '*/15 * * * *' },
		{ label: 'Every hour', value: '0 * * * *' },
		{ label: 'Every day at midnight', value: '0 0 * * *' },
		{ label: 'Every day at noon', value: '0 12 * * *' },
		{ label: 'Every week (Sunday)', value: '0 0 * * 0' },
		{ label: 'Every month (1st)', value: '0 0 1 * *' },
	];

	function parseCronParts(cronValue: string) {
		const parts = cronValue.split(' ');
		if (parts.length !== 5) return null;
		return {
			minute: parts[0],
			hour: parts[1],
			dayOfMonth: parts[2],
			month: parts[3],
			dayOfWeek: parts[4],
		};
	}

	const cronParts = $derived(parseCronParts(value));

	function handleInputChange(event: Event) {
		const target = event.target as HTMLInputElement;
		onchange?.(target.value);
	}

	function selectPreset(presetValue: string) {
		onchange?.(presetValue);
	}
</script>

<div class="cron-schedule-input">
	<div class="input-group">
		<label class="input-label" for="cron-expression">Cron Expression</label>
		<input
			id="cron-expression"
			type="text"
			class="cron-input"
			class:has-error={!!error}
			{value}
			oninput={handleInputChange}
			placeholder="* * * * *"
		/>
		{#if error}
			<span class="input-error">{error}</span>
		{/if}
	</div>

	<div class="cron-legend">
		<div class="legend-row">
			<span class="legend-item">
				<span class="legend-symbol">*</span>
				<span class="legend-desc">minute (0-59)</span>
			</span>
			<span class="legend-item">
				<span class="legend-symbol">*</span>
				<span class="legend-desc">hour (0-23)</span>
			</span>
			<span class="legend-item">
				<span class="legend-symbol">*</span>
				<span class="legend-desc">day (1-31)</span>
			</span>
			<span class="legend-item">
				<span class="legend-symbol">*</span>
				<span class="legend-desc">month (1-12)</span>
			</span>
			<span class="legend-item">
				<span class="legend-symbol">*</span>
				<span class="legend-desc">weekday (0-6)</span>
			</span>
		</div>
	</div>

	{#if cronParts}
		<div class="cron-parsed">
			<span class="parsed-label">Parsed:</span>
			<span class="parsed-part" title="Minute">{cronParts.minute}</span>
			<span class="parsed-part" title="Hour">{cronParts.hour}</span>
			<span class="parsed-part" title="Day of Month">{cronParts.dayOfMonth}</span>
			<span class="parsed-part" title="Month">{cronParts.month}</span>
			<span class="parsed-part" title="Day of Week">{cronParts.dayOfWeek}</span>
		</div>
	{/if}

	<div class="presets">
		<span class="presets-label">Quick presets:</span>
		<div class="presets-list">
			{#each presets as preset}
				<button
					type="button"
					class="preset-btn"
					class:active={value === preset.value}
					onclick={() => selectPreset(preset.value)}
				>
					{preset.label}
				</button>
			{/each}
		</div>
	</div>
</div>

<style>
	.cron-schedule-input {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.input-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.input-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.cron-input {
		padding: var(--space-2) var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-lg);
		letter-spacing: 0.1em;
		text-align: center;
		background: var(--color-bg);
		color: var(--color-fg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
	}

	.cron-input:focus {
		outline: 2px solid var(--color-accent);
		outline-offset: -1px;
	}

	.cron-input.has-error {
		border-color: var(--color-error);
	}

	.input-error {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-error);
	}

	.cron-legend {
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg-muted);
		border-radius: var(--radius-sm);
	}

	.legend-row {
		display: flex;
		justify-content: space-around;
		gap: var(--space-2);
	}

	.legend-item {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
	}

	.legend-symbol {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.legend-desc {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-muted);
	}

	.cron-parsed {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.parsed-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.parsed-part {
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		background: var(--color-accent-soft);
		color: var(--color-accent);
		border-radius: var(--radius-sm);
	}

	.presets {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.presets-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.presets-list {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}

	.preset-btn {
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: var(--color-bg-muted);
		color: var(--color-fg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: all 0.15s ease;
	}

	.preset-btn:hover {
		background: var(--color-bg-subtle);
		color: var(--color-fg);
	}

	.preset-btn.active {
		background: var(--color-accent-soft);
		color: var(--color-accent);
		border-color: var(--color-accent);
	}
</style>
