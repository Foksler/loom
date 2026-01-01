<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/auth';
	import { getApiClient } from '$lib/api/client';
	import { i18n } from '$lib/i18n';
	import { ImpersonationBanner } from '$lib/ui';
	import type { Snippet } from 'svelte';
	import type { CurrentUser, ImpersonationState } from '$lib/api/types';

	interface Props {
		children: Snippet;
		data: { user: CurrentUser };
	}

	let { children, data }: Props = $props();

	let impersonationState = $state<ImpersonationState | null>(null);

	const isSystemAdmin = $derived(data.user?.global_roles?.includes('system_admin') ?? false);

	// Initialize auth store with server-loaded user
	$effect(() => {
		authStore.start();
		authStore.loginSuccess(data.user);
	});

	$effect(() => {
		if (isSystemAdmin) {
			loadImpersonationState();
		}
	});

	async function loadImpersonationState() {
		try {
			const client = getApiClient();
			impersonationState = await client.getImpersonationState();
		} catch {
			// Ignore errors - impersonation state is optional
		}
	}

	async function handleLogout() {
		try {
			const client = getApiClient();
			await client.logout();
			await goto('/login');
		} catch {
			// Even if logout fails, redirect to login
			await goto('/login');
		}
	}
</script>

<div class="min-h-screen bg-gray-50 dark:bg-gray-900">
	<!-- Impersonation banner -->
	{#if impersonationState?.is_impersonating}
		<ImpersonationBanner impersonation={impersonationState} onStop={loadImpersonationState} />
	{/if}

	<!-- Header with user info and logout -->
	<header class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex justify-between items-center h-16">
				<div class="flex items-center gap-8">
					<a href="/threads" class="text-xl font-bold text-gray-900 dark:text-white">
						Loom
					</a>
					<nav class="flex items-center gap-6">
						<a href="/threads" class="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white">
							{i18n._('nav.threads')}
						</a>
						<a href="/weavers" class="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white">
							{i18n._('nav.weavers')}
						</a>
						<a href="/settings/profile" class="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white">
							{i18n._('nav.settings')}
						</a>
						{#if isSystemAdmin}
							<a href="/admin/users" class="text-sm text-warning hover:text-warning/80">
								{i18n._('nav.admin')}
							</a>
							<a href="/admin/anthropic-accounts" class="text-sm text-warning hover:text-warning/80">
								{i18n._('admin.anthropic.title')}
							</a>
							<a href="/admin/jobs" class="text-sm text-warning hover:text-warning/80">
								{i18n._('nav.jobs')}
							</a>
						{/if}
					</nav>
				</div>
				
				<div class="flex items-center gap-4">
					{#if data.user}
						<div class="flex items-center gap-3">
							{#if data.user.avatar_url}
								<img 
									src={data.user.avatar_url} 
									alt="" 
									class="w-8 h-8 rounded-full"
								/>
							{:else}
								<div class="w-8 h-8 rounded-full bg-gray-300 dark:bg-gray-600 flex items-center justify-center">
									<span class="text-sm font-medium text-gray-600 dark:text-gray-300">
										{data.user.display_name?.charAt(0).toUpperCase() ?? '?'}
									</span>
								</div>
							{/if}
							<span class="text-sm text-gray-700 dark:text-gray-300">
								{data.user.display_name}
							</span>
						</div>
						<button
							onclick={handleLogout}
							class="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
						>
							{i18n._('auth.signOut')}
						</button>
					{/if}
				</div>
			</div>
		</div>
	</header>

	<!-- Main content -->
	<main>
		{@render children()}
	</main>
</div>
