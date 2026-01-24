<script lang="ts">
	import CronScheduleInput from './CronScheduleInput.svelte';

	interface MonitorFormData {
		name: string;
		slug: string;
		description: string;
		schedule_type: 'cron' | 'interval';
		schedule_value: string;
		timezone: string;
		checkin_margin_minutes: number;
		max_runtime_minutes: number | null;
		environments: string[];
	}

	interface Props {
		initialData?: Partial<MonitorFormData>;
		isEditing?: boolean;
		onsubmit?: (data: MonitorFormData) => void;
		oncancel?: () => void;
	}

	let { initialData, isEditing = false, onsubmit, oncancel }: Props = $props();

	let formData = $state<MonitorFormData>({
		name: initialData?.name ?? '',
		slug: initialData?.slug ?? '',
		description: initialData?.description ?? '',
		schedule_type: initialData?.schedule_type ?? 'cron',
		schedule_value: initialData?.schedule_value ?? '0 * * * *',
		timezone: initialData?.timezone ?? 'UTC',
		checkin_margin_minutes: initialData?.checkin_margin_minutes ?? 5,
		max_runtime_minutes: initialData?.max_runtime_minutes ?? null,
		environments: initialData?.environments ?? [],
	});

	let errors = $state<Record<string, string>>({});
	let newEnvironment = $state('');

	const timezones = [
		'UTC',
		'America/New_York',
		'America/Chicago',
		'America/Denver',
		'America/Los_Angeles',
		'Europe/London',
		'Europe/Paris',
		'Europe/Berlin',
		'Asia/Tokyo',
		'Asia/Shanghai',
		'Australia/Sydney',
	];

	function generateSlug(name: string): string {
		return name
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '');
	}

	function handleNameChange(event: Event) {
		const target = event.target as HTMLInputElement;
		formData.name = target.value;
		if (!isEditing) {
			formData.slug = generateSlug(target.value);
		}
	}

	function handleScheduleChange(value: string) {
		formData.schedule_value = value;
	}

	function addEnvironment() {
		if (newEnvironment.trim() && !formData.environments.includes(newEnvironment.trim())) {
			formData.environments = [...formData.environments, newEnvironment.trim()];
			newEnvironment = '';
		}
	}

	function removeEnvironment(env: string) {
		formData.environments = formData.environments.filter((e) => e !== env);
	}

	function validate(): boolean {
		errors = {};

		if (!formData.name.trim()) {
			errors.name = 'Name is required';
		}

		if (!formData.slug.trim()) {
			errors.slug = 'Slug is required';
		} else if (!/^[a-z0-9-]+$/.test(formData.slug)) {
			errors.slug = 'Slug must contain only lowercase letters, numbers, and hyphens';
		}

		if (!formData.schedule_value.trim()) {
			errors.schedule_value = 'Schedule is required';
		}

		return Object.keys(errors).length === 0;
	}

	function handleSubmit(event: Event) {
		event.preventDefault();
		if (validate()) {
			onsubmit?.(formData);
		}
	}
</script>

<form class="monitor-form" onsubmit={handleSubmit}>
	<div class="form-group">
		<label class="form-label" for="name">Name</label>
		<input
			id="name"
			type="text"
			class="form-input"
			class:has-error={!!errors.name}
			value={formData.name}
			oninput={handleNameChange}
			placeholder="Daily Cleanup Job"
		/>
		{#if errors.name}
			<span class="form-error">{errors.name}</span>
		{/if}
	</div>

	<div class="form-group">
		<label class="form-label" for="slug">Slug</label>
		<input
			id="slug"
			type="text"
			class="form-input"
			class:has-error={!!errors.slug}
			bind:value={formData.slug}
			placeholder="daily-cleanup"
			disabled={isEditing}
		/>
		{#if errors.slug}
			<span class="form-error">{errors.slug}</span>
		{/if}
		<span class="form-hint">URL-safe identifier</span>
	</div>

	<div class="form-group">
		<label class="form-label" for="description">Description (optional)</label>
		<textarea
			id="description"
			class="form-textarea"
			bind:value={formData.description}
			placeholder="Describe what this job does..."
			rows="3"
		></textarea>
	</div>

	<div class="form-group">
		<label class="form-label">Schedule Type</label>
		<div class="radio-group">
			<label class="radio-label">
				<input type="radio" bind:group={formData.schedule_type} value="cron" />
				<span>Cron expression</span>
			</label>
			<label class="radio-label">
				<input type="radio" bind:group={formData.schedule_type} value="interval" />
				<span>Fixed interval</span>
			</label>
		</div>
	</div>

	{#if formData.schedule_type === 'cron'}
		<div class="form-group">
			<CronScheduleInput
				value={formData.schedule_value}
				onchange={handleScheduleChange}
				error={errors.schedule_value}
			/>
		</div>
	{:else}
		<div class="form-group">
			<label class="form-label" for="interval">Interval (minutes)</label>
			<input
				id="interval"
				type="number"
				class="form-input"
				bind:value={formData.schedule_value}
				min="1"
				placeholder="30"
			/>
		</div>
	{/if}

	<div class="form-group">
		<label class="form-label" for="timezone">Timezone</label>
		<select id="timezone" class="form-select" bind:value={formData.timezone}>
			{#each timezones as tz}
				<option value={tz}>{tz}</option>
			{/each}
		</select>
	</div>

	<div class="form-row">
		<div class="form-group">
			<label class="form-label" for="margin">Grace Period (minutes)</label>
			<input
				id="margin"
				type="number"
				class="form-input"
				bind:value={formData.checkin_margin_minutes}
				min="1"
				max="60"
			/>
			<span class="form-hint">Time after expected before marking as missed</span>
		</div>

		<div class="form-group">
			<label class="form-label" for="runtime">Max Runtime (minutes, optional)</label>
			<input
				id="runtime"
				type="number"
				class="form-input"
				bind:value={formData.max_runtime_minutes}
				min="1"
				placeholder="No limit"
			/>
			<span class="form-hint">Alert if job exceeds this duration</span>
		</div>
	</div>

	<div class="form-group">
		<label class="form-label">Environments (optional)</label>
		<div class="environments-input">
			<input
				type="text"
				class="form-input"
				bind:value={newEnvironment}
				placeholder="production"
				onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addEnvironment())}
			/>
			<button type="button" class="add-btn" onclick={addEnvironment}>Add</button>
		</div>
		{#if formData.environments.length > 0}
			<div class="environments-list">
				{#each formData.environments as env}
					<span class="environment-tag">
						{env}
						<button type="button" class="remove-tag" onclick={() => removeEnvironment(env)}>
							&times;
						</button>
					</span>
				{/each}
			</div>
		{/if}
		<span class="form-hint">Only check-ins from these environments will be accepted</span>
	</div>

	<div class="form-actions">
		<button type="button" class="cancel-btn" onclick={oncancel}>Cancel</button>
		<button type="submit" class="submit-btn">
			{isEditing ? 'Save Changes' : 'Create Monitor'}
		</button>
	</div>
</form>

<style>
	.monitor-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-4);
	}

	.form-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.form-input,
	.form-textarea,
	.form-select {
		padding: var(--space-2) var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		background: var(--color-bg);
		color: var(--color-fg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
	}

	.form-input:focus,
	.form-textarea:focus,
	.form-select:focus {
		outline: 2px solid var(--color-accent);
		outline-offset: -1px;
	}

	.form-input.has-error {
		border-color: var(--color-error);
	}

	.form-input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.form-textarea {
		resize: vertical;
	}

	.form-error {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-error);
	}

	.form-hint {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--color-fg-subtle);
	}

	.radio-group {
		display: flex;
		gap: var(--space-4);
	}

	.radio-label {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
		cursor: pointer;
	}

	.environments-input {
		display: flex;
		gap: var(--space-2);
	}

	.add-btn {
		padding: var(--space-2) var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		background: var(--color-bg-muted);
		color: var(--color-fg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
		cursor: pointer;
	}

	.add-btn:hover {
		background: var(--color-bg-subtle);
	}

	.environments-list {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-top: var(--space-2);
	}

	.environment-tag {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: var(--color-bg-muted);
		color: var(--color-fg);
		border-radius: var(--radius-sm);
	}

	.remove-tag {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 14px;
		height: 14px;
		font-size: 12px;
		background: transparent;
		color: var(--color-fg-muted);
		border: none;
		cursor: pointer;
	}

	.remove-tag:hover {
		color: var(--color-error);
	}

	.form-actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
		margin-top: var(--space-4);
		padding-top: var(--space-4);
		border-top: 1px solid var(--color-border-muted);
	}

	.cancel-btn {
		padding: var(--space-2) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		background: var(--color-bg-muted);
		color: var(--color-fg);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
		cursor: pointer;
	}

	.cancel-btn:hover {
		background: var(--color-bg-subtle);
	}

	.submit-btn {
		padding: var(--space-2) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		background: var(--color-accent);
		color: var(--color-bg);
		border: none;
		border-radius: var(--radius-sm);
		cursor: pointer;
	}

	.submit-btn:hover {
		opacity: 0.9;
	}
</style>
