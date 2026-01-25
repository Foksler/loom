<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { i18n } from '$lib/i18n';
	import { getApiClient } from '$lib/api/client';
	import type { WhatsAppGroup } from '$lib/api/types';
	import { Card, Badge, Button, Input } from '$lib/ui';
	import { trackAction, trackModalOpen, trackModalClose } from '$lib/analytics';

	const orgId = $derived($page.params.orgId!);
	const client = getApiClient();

	let groups = $state<WhatsAppGroup[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Create modal
	let showCreateModal = $state(false);
	let newGroupName = $state('');
	let newGroupDescription = $state('');
	let newGroupColor = $state('#3B82F6');
	let creating = $state(false);

	// Delete state
	let deletingId = $state<string | null>(null);

	async function loadGroups() {
		loading = true;
		error = null;
		try {
			const response = await client.listWhatsAppGroups(orgId);
			groups = response.groups;
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			loading = false;
		}
	}

	async function createGroup() {
		if (!newGroupName.trim()) return;
		trackAction('create', 'whatsapp_group', newGroupName);
		creating = true;
		error = null;
		try {
			const group = await client.createWhatsAppGroup(orgId, {
				name: newGroupName,
				description: newGroupDescription || undefined,
				color: newGroupColor,
			});
			groups = [...groups, group];
			closeCreateModal();
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			creating = false;
		}
	}

	async function deleteGroup(group: WhatsAppGroup) {
		if (!confirm(`Delete group "${group.name}"? Conversations will be unassigned.`)) return;
		trackAction('delete', 'whatsapp_group', group.id);
		deletingId = group.id;
		error = null;
		try {
			await client.deleteWhatsAppGroup(orgId, group.id);
			groups = groups.filter((g) => g.id !== group.id);
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			deletingId = null;
		}
	}

	function openCreateModal() {
		trackModalOpen('create_whatsapp_group');
		showCreateModal = true;
	}

	function closeCreateModal() {
		trackModalClose('create_whatsapp_group');
		showCreateModal = false;
		newGroupName = '';
		newGroupDescription = '';
		newGroupColor = '#3B82F6';
	}

	$effect(() => {
		loadGroups();
	});
</script>

<svelte:head>
	<title>WhatsApp Groups - Loom</title>
</svelte:head>

<div class="mb-6">
	<nav class="text-sm text-fg-muted mb-2">
		<a href="/settings/orgs/{orgId}/whatsapp" class="hover:text-fg">WhatsApp</a>
		<span class="mx-2">/</span>
		<span>Groups</span>
	</nav>
	<h1 class="text-2xl font-bold text-fg">Conversation Groups</h1>
	<p class="text-fg-muted">Organize WhatsApp conversations into groups for easier management</p>
</div>

{#if loading}
	<div class="text-fg-muted">{i18n._('general.loading')}</div>
{:else}
	{#if error}
		<div class="mb-4 p-3 rounded-md bg-error/10 text-error text-sm">{error}</div>
	{/if}

	<Card>
		<div class="flex justify-between items-center mb-4">
			<h2 class="text-lg font-medium text-fg">Groups</h2>
			<Button onclick={openCreateModal}>Create Group</Button>
		</div>

		{#if groups.length > 0}
			<div class="space-y-2">
				{#each groups as group (group.id)}
					<div class="flex items-center justify-between p-3 border border-border rounded-md hover:bg-bg-muted">
						<div class="flex items-center gap-3">
							<div
								class="w-4 h-4 rounded"
								style="background: {group.color ?? '#6B7280'}"
							></div>
							<div>
								<div class="font-medium text-fg">{group.name}</div>
								{#if group.description}
									<div class="text-sm text-fg-muted">{group.description}</div>
								{/if}
							</div>
							{#if group.is_default}
								<Badge variant="secondary" size="sm">Default</Badge>
							{/if}
						</div>
						<div class="flex gap-2">
							<Button
								variant="ghost"
								size="sm"
								disabled={deletingId === group.id}
								loading={deletingId === group.id}
								onclick={() => deleteGroup(group)}
							>
								Delete
							</Button>
						</div>
					</div>
				{/each}
			</div>
		{:else}
			<div class="text-center py-8 text-fg-muted">
				<p class="mb-2">No groups yet</p>
				<p class="text-sm">Create groups to organize your WhatsApp conversations</p>
			</div>
		{/if}
	</Card>
{/if}

{#if showCreateModal}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
		onclick={closeCreateModal}
		onkeydown={(e) => e.key === 'Escape' && closeCreateModal()}
	>
		<div
			class="bg-bg border border-border rounded-lg p-6 w-full max-w-md"
			role="document"
			onclick={(e) => e.stopPropagation()}
		>
			<h2 class="text-lg font-bold text-fg mb-4">Create Group</h2>
			<form onsubmit={(e) => { e.preventDefault(); createGroup(); }} class="space-y-4">
				<Input
					label="Name"
					placeholder="e.g., Support, Sales"
					bind:value={newGroupName}
				/>

				<Input
					label="Description (optional)"
					placeholder="Brief description of this group"
					bind:value={newGroupDescription}
				/>

				<div class="w-full">
					<label for="color" class="block text-sm font-medium text-fg mb-1.5">
						Color
					</label>
					<input
						id="color"
						type="color"
						bind:value={newGroupColor}
						class="h-10 w-20 rounded-md border border-border cursor-pointer"
					/>
				</div>

				<div class="flex justify-end gap-2 mt-4">
					<Button variant="secondary" type="button" onclick={closeCreateModal}>
						Cancel
					</Button>
					<Button type="submit" disabled={creating || !newGroupName.trim()} loading={creating}>
						Create Group
					</Button>
				</div>
			</form>
		</div>
	</div>
{/if}
