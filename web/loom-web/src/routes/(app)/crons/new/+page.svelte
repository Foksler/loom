<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { getApiClient } from '$lib/api/client';
	import type { Org } from '$lib/api/types';
	import { MonitorForm } from '$lib/components/crons';
	import { Card } from '$lib/ui';
	import { trackFormSubmit, trackButtonClick, trackFilterChange } from '$lib/analytics';

	const client = getApiClient();

	let orgs = $state<Org[]>([]);
	let selectedOrgId = $state<string | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let submitting = $state(false);

	$effect(() => {
		loadOrgs();
	});

	async function loadOrgs() {
		try {
			const response = await client.listOrgs();
			orgs = response.orgs;
			if (orgs.length > 0) {
				selectedOrgId = orgs[0].id;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load organizations';
		} finally {
			loading = false;
		}
	}

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

	async function handleSubmit(data: MonitorFormData) {
		if (!selectedOrgId) return;

		trackFormSubmit('create_monitor', { monitor_slug: data.slug, schedule_type: data.schedule_type });
		submitting = true;
		error = null;

		try {
			await client.createMonitor(selectedOrgId, {
				name: data.name,
				slug: data.slug,
				description: data.description || undefined,
				schedule_type: data.schedule_type,
				schedule_value: data.schedule_value,
				timezone: data.timezone,
				checkin_margin_minutes: data.checkin_margin_minutes,
				max_runtime_minutes: data.max_runtime_minutes || undefined,
				environments: data.environments.length > 0 ? data.environments : undefined,
			});

			await goto(`/crons/${data.slug}?org_id=${selectedOrgId}`);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create monitor';
		} finally {
			submitting = false;
		}
	}

	function handleCancel() {
		trackButtonClick('cancel_create_monitor');
		goto('/crons');
	}
</script>

<div class="new-monitor-page">
	<header class="page-header">
		<h1 class="page-title">Create Monitor</h1>
		<p class="page-description">Set up a new cron job monitor</p>
	</header>

	{#if orgs.length > 1}
		<div class="org-selector">
			<label class="org-label" for="org-select">Organization</label>
			<select
				id="org-select"
				class="org-select"
				value={selectedOrgId}
				onchange={(e) => { trackFilterChange('org', e.currentTarget.value, { page: 'crons_new' }); selectedOrgId = e.currentTarget.value; }}
			>
				{#each orgs as org}
					<option value={org.id}>{org.name}</option>
				{/each}
			</select>
		</div>
	{/if}

	{#if loading}
		<div class="loading">Loading...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else}
		<Card>
			<MonitorForm
				onsubmit={handleSubmit}
				oncancel={handleCancel}
			/>
		</Card>
	{/if}
</div>

<style>
	.new-monitor-page {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		max-width: 800px;
		margin: 0 auto;
		padding: var(--space-6);
	}

	.page-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.page-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xl);
		font-weight: 700;
		color: var(--color-fg);
	}

	.page-description {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.org-selector {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.org-label {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.org-select {
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
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
</style>
