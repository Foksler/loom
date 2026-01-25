<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { i18n } from '$lib/i18n';
	import { getApiClient } from '$lib/api/client';
	import type { WhatsAppConfig, WhatsAppGroup } from '$lib/api/types';
	import { Card, Badge, Button, Input } from '$lib/ui';
	import { trackFormSubmit, trackButtonClick, trackAction } from '$lib/analytics';

	const orgId = $derived($page.params.orgId!);
	const client = getApiClient();

	let config = $state<WhatsAppConfig | null>(null);
	let groups = $state<WhatsAppGroup[]>([]);

	let loading = $state(true);
	let error = $state<string | null>(null);
	let saving = $state(false);
	let saveSuccess = $state(false);

	// Form fields
	let phoneNumberId = $state('');
	let accessToken = $state('');
	let appSecret = $state('');
	let verifyToken = $state('');

	// Delete modal
	let showDeleteModal = $state(false);

	async function loadConfig() {
		loading = true;
		error = null;
		try {
			config = await client.getWhatsAppConfig(orgId);
			phoneNumberId = config.phone_number_id;
		} catch (e: unknown) {
			if (e && typeof e === 'object' && 'status' in e && (e as { status: number }).status === 404) {
				config = null;
			} else {
				error = e instanceof Error ? e.message : i18n._('general.error');
			}
		} finally {
			loading = false;
		}
	}

	async function loadGroups() {
		try {
			const response = await client.listWhatsAppGroups(orgId);
			groups = response.groups;
		} catch {
			// Ignore if not configured
		}
	}

	async function saveConfig() {
		trackFormSubmit('whatsapp_config', { org_id: orgId });
		saving = true;
		saveSuccess = false;
		error = null;
		try {
			config = await client.createOrUpdateWhatsAppConfig(orgId, {
				phone_number_id: phoneNumberId,
				access_token: accessToken,
				app_secret: appSecret,
				verify_token: verifyToken,
			});
			saveSuccess = true;
			accessToken = '';
			appSecret = '';
			verifyToken = '';
			await loadGroups();
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			saving = false;
		}
	}

	async function deleteConfig() {
		trackAction('delete', 'whatsapp_config', orgId);
		try {
			await client.deleteWhatsAppConfig(orgId);
			config = null;
			phoneNumberId = '';
			groups = [];
			showDeleteModal = false;
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		}
	}

	function copyWebhookUrl() {
		trackButtonClick('copy_webhook_url', { org_id: orgId });
		if (config) {
			navigator.clipboard.writeText(config.webhook_url);
		}
	}

	$effect(() => {
		loadConfig();
		loadGroups();
	});
</script>

<svelte:head>
	<title>WhatsApp Integration - Loom</title>
</svelte:head>

<div class="mb-6">
	<h1 class="text-2xl font-bold text-fg">WhatsApp Business API</h1>
	<p class="text-fg-muted">Configure WhatsApp integration for your organization</p>
</div>

{#if loading}
	<div class="text-fg-muted">{i18n._('general.loading')}</div>
{:else}
	{#if error}
		<div class="mb-4 p-3 rounded-md bg-error/10 text-error text-sm">{error}</div>
	{/if}

	<Card>
		<div class="flex items-center gap-2 mb-4">
			{#if config}
				<Badge variant="success">Connected</Badge>
				<span class="text-fg-muted">Phone ID: {config.phone_number_id}</span>
			{:else}
				<Badge variant="muted">Not Configured</Badge>
			{/if}
		</div>

		<form onsubmit={(e) => { e.preventDefault(); saveConfig(); }} class="space-y-4">
			<Input
				label="Phone Number ID"
				placeholder="From Meta WhatsApp dashboard"
				bind:value={phoneNumberId}
			/>

			<Input
				label="Access Token"
				type="password"
				placeholder="WhatsApp Cloud API access token"
				bind:value={accessToken}
			/>

			<Input
				label="App Secret"
				type="password"
				placeholder="For webhook signature verification"
				bind:value={appSecret}
			/>

			<Input
				label="Verify Token"
				placeholder="For webhook challenge verification"
				bind:value={verifyToken}
			/>

			{#if config}
				<div class="p-3 bg-bg-muted rounded-md">
					<p class="text-sm text-fg-muted mb-1">Webhook URL (copy to Meta dashboard):</p>
					<div class="flex items-center gap-2">
						<code class="text-sm font-mono text-fg flex-1 break-all">{config.webhook_url}</code>
						<Button variant="secondary" size="sm" onclick={copyWebhookUrl}>
							Copy
						</Button>
					</div>
				</div>
			{/if}

			{#if saveSuccess}
				<div class="p-3 rounded-md bg-success/10 text-success text-sm">
					Configuration saved successfully
				</div>
			{/if}

			<div class="flex justify-between">
				<div>
					{#if config}
						<Button
							variant="danger"
							type="button"
							onclick={() => {
								trackButtonClick('delete_whatsapp_config');
								showDeleteModal = true;
							}}
						>
							Disconnect
						</Button>
					{/if}
				</div>
				<Button type="submit" disabled={saving} loading={saving}>
					{config ? 'Update Configuration' : 'Save Configuration'}
				</Button>
			</div>
		</form>
	</Card>

	{#if config && groups.length > 0}
		<div class="mt-6">
		<Card>
			<div class="flex items-center justify-between mb-4">
				<h2 class="text-lg font-medium text-fg">Conversation Groups</h2>
				<a href="/settings/orgs/{orgId}/whatsapp/groups" class="text-accent hover:underline text-sm">
					Manage Groups
				</a>
			</div>
			<div class="space-y-2">
				{#each groups as group (group.id)}
					<div class="flex items-center gap-3 p-2 rounded-md hover:bg-bg-muted">
						{#if group.color}
							<div class="w-4 h-4 rounded" style="background: {group.color}"></div>
						{/if}
						<span class="text-fg">{group.name}</span>
						{#if group.is_default}
							<Badge variant="muted" size="sm">Default</Badge>
						{/if}
					</div>
				{/each}
			</div>
		</Card>
		</div>
	{:else if config}
		<div class="mt-6">
		<Card>
			<div class="text-center py-4">
				<p class="text-fg-muted mb-2">No groups configured yet</p>
				<a href="/settings/orgs/{orgId}/whatsapp/groups" class="text-accent hover:underline">
					Create your first group
				</a>
			</div>
		</Card>
		</div>
	{/if}
{/if}

{#if showDeleteModal}
	<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" role="dialog" aria-modal="true">
		<div class="bg-bg border border-border rounded-lg p-6 w-full max-w-md" role="document" onclick={(e) => e.stopPropagation()}>
			<h2 class="text-lg font-bold text-fg mb-4">Disconnect WhatsApp</h2>
			<p class="text-fg-muted mb-4">
				Are you sure you want to disconnect WhatsApp? This will remove all configuration and stop receiving messages.
			</p>
			<div class="flex justify-end gap-2">
				<Button variant="secondary" onclick={() => (showDeleteModal = false)}>
					Cancel
				</Button>
				<Button variant="danger" onclick={deleteConfig}>
					Disconnect
				</Button>
			</div>
		</div>
	</div>
{/if}
