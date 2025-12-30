<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { getApiClient } from '$lib/api/client';
	import { i18n } from '$lib/i18n';

	let email = $state('');
	let magicLinkSent = $state(false);
	let isSubmitting = $state(false);
	let error = $state<string | null>(null);

	const redirectTo = $derived.by(() => {
		const param = $page.url.searchParams.get('redirectTo') ?? '/';
		// Only allow relative paths starting with / to prevent open redirect attacks
		if (param.startsWith('/') && !param.startsWith('//')) {
			return param;
		}
		return '/';
	});

	async function requestMagicLink(e: SubmitEvent) {
		e.preventDefault();
		if (!email.trim()) return;

		isSubmitting = true;
		error = null;

		try {
			const client = getApiClient();
			await client.requestMagicLink(email);
			magicLinkSent = true;
		} catch (err) {
			error = i18n._('auth.login.error');
		} finally {
			isSubmitting = false;
		}
	}

	function getOAuthUrl(provider: string): string {
		const params = new URLSearchParams();
		params.set('redirect', redirectTo);
		return `/api/auth/login/${provider}?${params.toString()}`;
	}
</script>

<svelte:head>
	<title>{i18n._('auth.login.title')}</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900 px-4">
	<div class="max-w-md w-full space-y-8">
		<div class="text-center">
			<h1 class="text-3xl font-bold text-gray-900 dark:text-white">{i18n._('auth.login.title')}</h1>
			<p class="mt-2 text-sm text-gray-600 dark:text-gray-400">
				{i18n._('auth.login.subtitle')}
			</p>
		</div>

		<div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-8 space-y-6">
			<!-- OAuth Buttons -->
			<div class="space-y-3">
				<a
					href={getOAuthUrl('github')}
					class="w-full flex items-center justify-center gap-3 px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
				>
					<svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
						<path d="M12 0C5.374 0 0 5.373 0 12c0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23A11.509 11.509 0 0112 5.803c1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576C20.566 21.797 24 17.3 24 12c0-6.627-5.373-12-12-12z"/>
					</svg>
					{i18n._('auth.login.github')}
				</a>

				<a
					href={getOAuthUrl('google')}
					class="w-full flex items-center justify-center gap-3 px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
				>
					<svg class="w-5 h-5" viewBox="0 0 24 24">
						<path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
						<path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
						<path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
						<path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
					</svg>
					{i18n._('auth.login.google')}
				</a>
			</div>

			<div class="relative">
				<div class="absolute inset-0 flex items-center">
					<div class="w-full border-t border-gray-300 dark:border-gray-600"></div>
				</div>
				<div class="relative flex justify-center text-sm">
					<span class="px-2 bg-white dark:bg-gray-800 text-gray-500">{i18n._('auth.login.or')}</span>
				</div>
			</div>

			<!-- Magic Link Form -->
			{#if magicLinkSent}
				<div class="text-center py-4">
					<div class="inline-flex items-center justify-center w-12 h-12 rounded-full bg-green-100 dark:bg-green-900 mb-4">
						<svg class="w-6 h-6 text-green-600 dark:text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
						</svg>
					</div>
					<h3 class="text-lg font-medium text-gray-900 dark:text-white">{i18n._('auth.login.checkEmail')}</h3>
					<p class="mt-1 text-sm text-gray-600 dark:text-gray-400">
						{i18n._('auth.login.magicLinkSent')} <strong>{email}</strong>
					</p>
					<button
						type="button"
						onclick={() => { magicLinkSent = false; email = ''; }}
						class="mt-4 text-sm text-blue-600 dark:text-blue-400 hover:underline"
					>
						{i18n._('auth.login.useDifferentEmail')}
					</button>
				</div>
			{:else}
				<form onsubmit={requestMagicLink} class="space-y-4">
					<div>
						<label for="email" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
							{i18n._('auth.login.emailLabel')}
						</label>
						<input
							type="email"
							id="email"
							bind:value={email}
							required
							disabled={isSubmitting}
							placeholder={i18n._('auth.login.emailPlaceholder')}
							class="mt-1 block w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg shadow-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50"
						/>
					</div>

					{#if error}
						<p class="text-sm text-red-600 dark:text-red-400">{error}</p>
					{/if}

					<button
						type="submit"
						disabled={isSubmitting || !email.trim()}
						class="w-full flex justify-center py-3 px-4 border border-transparent rounded-lg shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
					>
						{#if isSubmitting}
							{i18n._('auth.login.sending')}
						{:else}
							{i18n._('auth.login.sendMagicLink')}
						{/if}
					</button>
				</form>
			{/if}
		</div>
	</div>
</div>
