<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { i18n } from '$lib/i18n';
	import { getApiClient } from '$lib/api/client';
	import type { AdminUser } from '$lib/api/types';
	import { Card, Badge, Button, Input } from '$lib/ui';

	const client = getApiClient();

	let users = $state<AdminUser[]>([]);
	let total = $state(0);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let search = $state('');
	let offset = $state(0);
	const limit = 20;

	let impersonatingId = $state<string | null>(null);

	async function loadUsers() {
		loading = true;
		error = null;
		try {
			const response = await client.listAdminUsers({ limit, offset, search: search || undefined });
			users = response.users;
			total = response.total;
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			loading = false;
		}
	}

	async function handleSearch() {
		offset = 0;
		await loadUsers();
	}

	async function impersonate(userId: string) {
		impersonatingId = userId;
		try {
			await client.startImpersonation(userId);
			window.location.href = '/threads';
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
			impersonatingId = null;
		}
	}

	function getRoleBadges(roles: string[]): Array<{ role: string; variant: 'accent' | 'warning' | 'success' | 'muted' }> {
		return roles.map((role) => {
			let variant: 'accent' | 'warning' | 'success' | 'muted' = 'muted';
			if (role === 'system_admin') variant = 'accent';
			else if (role === 'support') variant = 'warning';
			else if (role === 'auditor') variant = 'success';
			return { role, variant };
		});
	}

	function formatDate(dateStr: string | null): string {
		if (!dateStr) return '-';
		return new Date(dateStr).toLocaleDateString();
	}

	$effect(() => {
		loadUsers();
	});
</script>

<svelte:head>
	<title>{i18n._('admin.users.title')} - Loom</title>
</svelte:head>

<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
	<div class="mb-6">
		<h1 class="text-2xl font-bold text-fg">{i18n._('admin.users.title')}</h1>
		<p class="text-fg-muted">{i18n._('admin.users.description')}</p>
	</div>

	{#if error}
		<div class="mb-4 p-3 rounded-md bg-error/10 text-error text-sm">{error}</div>
	{/if}

	<div class="mb-6">
		<form onsubmit={(e) => { e.preventDefault(); handleSearch(); }} class="flex gap-2">
			<Input
				bind:value={search}
				placeholder={i18n._('admin.users.searchPlaceholder')}
				class="flex-1"
			/>
			<Button type="submit" disabled={loading}>
				{i18n._('general.search')}
			</Button>
		</form>
	</div>

	{#if loading && users.length === 0}
		<Card>
			<div class="text-fg-muted text-center py-8">{i18n._('general.loading')}</div>
		</Card>
	{:else}
		<div class="space-y-3">
			{#each users as user (user.id)}
				<Card>
					<div class="flex items-center justify-between gap-4">
						<div class="flex items-center gap-4 min-w-0">
							{#if user.avatar_url}
								<img src={user.avatar_url} alt="" class="w-10 h-10 rounded-full" />
							{:else}
								<div class="w-10 h-10 rounded-full bg-bg-muted flex items-center justify-center">
									<span class="text-sm font-medium text-fg-muted">
										{user.display_name?.charAt(0).toUpperCase() ?? '?'}
									</span>
								</div>
							{/if}
							<div class="min-w-0">
								<div class="font-medium text-fg truncate">{user.display_name}</div>
								<div class="text-sm text-fg-muted truncate">{user.email ?? '-'}</div>
								<div class="flex flex-wrap gap-1 mt-1">
									{#each getRoleBadges(user.global_roles) as { role, variant }}
										<Badge {variant} size="sm">{role}</Badge>
									{/each}
								</div>
							</div>
						</div>
						<div class="flex items-center gap-4 flex-shrink-0">
							<div class="text-sm text-fg-muted text-right">
								<div>{i18n._('admin.users.created')}: {formatDate(user.created_at)}</div>
								<div>{i18n._('admin.users.lastLogin')}: {formatDate(user.last_login_at)}</div>
							</div>
							<Button
								variant="secondary"
								size="sm"
								disabled={impersonatingId === user.id}
								loading={impersonatingId === user.id}
								onclick={() => impersonate(user.id)}
							>
								{i18n._('admin.users.impersonate')}
							</Button>
						</div>
					</div>
				</Card>
			{/each}
		</div>

		{#if users.length === 0}
			<Card>
				<p class="text-fg-muted text-center py-4">{i18n._('admin.users.empty')}</p>
			</Card>
		{/if}

		{#if total > limit}
			<div class="flex justify-between items-center mt-6">
				<Button
					variant="secondary"
					disabled={offset === 0 || loading}
					onclick={() => { offset = Math.max(0, offset - limit); loadUsers(); }}
				>
					{i18n._('general.previous')}
				</Button>
				<span class="text-sm text-fg-muted">
					{offset + 1} - {Math.min(offset + limit, total)} / {total}
				</span>
				<Button
					variant="secondary"
					disabled={offset + limit >= total || loading}
					onclick={() => { offset = offset + limit; loadUsers(); }}
				>
					{i18n._('general.next')}
				</Button>
			</div>
		{/if}
	{/if}
</div>
