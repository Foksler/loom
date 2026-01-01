<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { i18n } from '$lib/i18n';
	import { Card, Badge, Button } from '$lib/ui';
	import {
		listAnthropicAccounts,
		initiateAnthropicOAuth,
		removeAnthropicAccount,
		type AnthropicAccount,
		type AccountsSummary,
	} from '$lib/api/anthropic';

	let accounts = $state<AnthropicAccount[]>([]);
	let summary = $state<AccountsSummary | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let addingAccount = $state(false);
	let removingId = $state<string | null>(null);
	let successMessage = $state<string | null>(null);
	let notConfigured = $state(false);

	$effect(() => {
		const added = $page.url.searchParams.get('added');
		if (added) {
			successMessage = `Account ${added} added successfully`;
			const url = new URL($page.url);
			url.searchParams.delete('added');
			history.replaceState({}, '', url.toString());
		}
	});

	async function loadAccounts() {
		loading = true;
		error = null;
		notConfigured = false;
		try {
			const response = await listAnthropicAccounts();
			accounts = response.accounts;
			summary = response.summary;
		} catch (e) {
			if (e instanceof Error && e.message.includes('404')) {
				notConfigured = true;
			} else {
				error = e instanceof Error ? e.message : i18n._('general.error');
			}
		} finally {
			loading = false;
		}
	}

	async function handleAddAccount() {
		addingAccount = true;
		error = null;
		try {
			const response = await initiateAnthropicOAuth('/admin/anthropic-accounts');
			window.location.href = response.redirect_url;
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
			addingAccount = false;
		}
	}

	async function handleRemove(accountId: string) {
		if (!confirm(i18n._('admin.anthropic.remove_confirm'))) {
			return;
		}
		removingId = accountId;
		error = null;
		try {
			await removeAnthropicAccount(accountId);
			await loadAccounts();
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			removingId = null;
		}
	}

	function getStatusBadge(status: string): { variant: 'success' | 'warning' | 'error'; label: string } {
		switch (status) {
			case 'available':
				return { variant: 'success', label: i18n._('admin.anthropic.status.available') };
			case 'cooling_down':
				return { variant: 'warning', label: i18n._('admin.anthropic.status.cooling_down') };
			case 'disabled':
				return { variant: 'error', label: i18n._('admin.anthropic.status.disabled') };
			default:
				return { variant: 'error', label: status };
		}
	}

	function formatCooldown(secs: number): string {
		const hours = Math.floor(secs / 3600);
		const minutes = Math.floor((secs % 3600) / 60);
		if (hours > 0) {
			return `${hours}h ${minutes}m`;
		}
		return `${minutes}m`;
	}

	function formatExpiry(dateStr: string): string {
		const date = new Date(dateStr);
		const now = new Date();
		const diffMs = date.getTime() - now.getTime();
		if (diffMs <= 0) {
			return 'Expired';
		}
		const diffMins = Math.floor(diffMs / 60000);
		if (diffMins < 60) {
			return `in ${diffMins} minutes`;
		}
		const diffHours = Math.floor(diffMins / 60);
		if (diffHours < 24) {
			return `in ${diffHours} hours`;
		}
		const diffDays = Math.floor(diffHours / 24);
		return `in ${diffDays} days`;
	}

	$effect(() => {
		loadAccounts();
	});
</script>

<svelte:head>
	<title>{i18n._('admin.anthropic.title')} - Loom</title>
</svelte:head>

<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
	<div class="mb-6 flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold text-fg">{i18n._('admin.anthropic.title')}</h1>
		</div>
		{#if !notConfigured}
			<Button onclick={handleAddAccount} disabled={addingAccount} loading={addingAccount}>
				{i18n._('admin.anthropic.add_account')}
			</Button>
		{/if}
	</div>

	{#if successMessage}
		<div class="mb-4 p-3 rounded-md bg-success/10 text-success text-sm">
			{successMessage}
		</div>
	{/if}

	{#if error}
		<div class="mb-4 p-3 rounded-md bg-error/10 text-error text-sm">{error}</div>
	{/if}

	{#if notConfigured}
		<Card>
			<p class="text-fg-muted text-center py-8">{i18n._('admin.anthropic.not_configured')}</p>
		</Card>
	{:else if loading && accounts.length === 0}
		<Card>
			<div class="text-fg-muted text-center py-8">{i18n._('general.loading')}</div>
		</Card>
	{:else if accounts.length === 0}
		<Card>
			<p class="text-fg-muted text-center py-8">{i18n._('admin.anthropic.no_accounts')}</p>
		</Card>
	{:else}
		<div class="space-y-3">
			{#each accounts as account (account.id)}
				{@const badge = getStatusBadge(account.status)}
				<Card>
					<div class="flex items-start justify-between gap-4">
						<div class="flex items-start gap-3 min-w-0">
							<div class="mt-1">
								{#if account.status === 'available'}
									<span class="inline-block w-3 h-3 rounded-full bg-success"></span>
								{:else if account.status === 'cooling_down'}
									<span class="inline-block w-3 h-3 rounded-full bg-warning"></span>
								{:else}
									<span class="inline-block w-3 h-3 rounded-full bg-error"></span>
								{/if}
							</div>
							<div class="min-w-0">
								<div class="font-medium text-fg">{account.id}</div>
								<div class="flex flex-wrap gap-2 mt-1">
									<Badge variant={badge.variant} size="sm">{badge.label}</Badge>
								</div>
								{#if account.status === 'cooling_down' && account.cooldown_remaining_secs}
									<div class="text-sm text-fg-muted mt-1">
										{i18n._('admin.anthropic.cooldown_remaining', { time: formatCooldown(account.cooldown_remaining_secs) })}
									</div>
								{/if}
								{#if account.last_error}
									<div class="text-sm text-error mt-1">
										{account.last_error}
									</div>
								{/if}
								{#if account.expires_at}
									<div class="text-sm text-fg-muted mt-1">
										{i18n._('admin.anthropic.expires_at', { time: formatExpiry(account.expires_at) })}
									</div>
								{/if}
							</div>
						</div>
						<Button
							variant="secondary"
							size="sm"
							onclick={() => handleRemove(account.id)}
							disabled={removingId === account.id}
							loading={removingId === account.id}
						>
							{i18n._('admin.anthropic.remove')}
						</Button>
					</div>
				</Card>
			{/each}
		</div>

		{#if summary}
			<div class="mt-6 text-sm text-fg-muted text-center">
				{summary.available} available, {summary.cooling_down} cooling, {summary.disabled} disabled ({summary.total} total)
			</div>
		{/if}
	{/if}
</div>
