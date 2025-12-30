<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { page } from '$app/stores';
	import { i18n, locales, localeNames, setLocale, type Locale } from '$lib/i18n';
	import { getApiClient } from '$lib/api/client';
	import { authStore } from '$lib/auth';
	import { Card, Button, Input } from '$lib/ui';

	const parentData = $derived($page.data as { user: import('$lib/api/types').CurrentUser });
	const user = $derived(parentData.user);

	let displayName = $state(user?.display_name ?? '');
	let selectedLocale = $state<Locale>((user?.locale as Locale) ?? 'en');
	let saving = $state(false);
	let successMessage = $state<string | null>(null);
	let errorMessage = $state<string | null>(null);

	$effect(() => {
		if (user) {
			displayName = user.display_name ?? '';
			selectedLocale = (user.locale as Locale) ?? 'en';
		}
	});

	const client = getApiClient();

	async function handleSave() {
		saving = true;
		successMessage = null;
		errorMessage = null;

		try {
			const updatedUser = await client.updateProfile({
				display_name: displayName,
				locale: selectedLocale,
			});
			authStore.loginSuccess(updatedUser);
			setLocale(selectedLocale);
			successMessage = i18n._('settings.profile.saved');
		} catch (e) {
			errorMessage = e instanceof Error ? e.message : i18n._('settings.profile.error');
		} finally {
			saving = false;
		}
	}

	function handleLocaleChange(event: Event) {
		const target = event.target as HTMLSelectElement;
		selectedLocale = target.value as Locale;
	}
</script>

<svelte:head>
	<title>{i18n._('settings.profile.title')} - Loom</title>
</svelte:head>

<div>
	<h1 class="text-2xl font-bold text-fg mb-6">
		{i18n._('settings.profile.title')}
	</h1>

	<Card>
		<form onsubmit={(e) => { e.preventDefault(); handleSave(); }} class="space-y-6">
			<Input
				label={i18n._('settings.profile.displayName')}
				bind:value={displayName}
			/>

			<div class="w-full">
				<label for="email" class="block text-sm font-medium text-fg mb-1.5">
					{i18n._('settings.profile.email')}
				</label>
				<input
					id="email"
					type="email"
					value={user?.email ?? ''}
					disabled
					class="w-full h-10 px-3 rounded-md border border-border bg-bg-muted text-fg-muted
								 cursor-not-allowed opacity-60"
				/>
				<p class="mt-1.5 text-sm text-fg-muted">
					{i18n._('settings.profile.emailHint')}
				</p>
			</div>

			<div class="w-full">
				<label for="locale" class="block text-sm font-medium text-fg mb-1.5">
					{i18n._('settings.profile.locale')}
				</label>
				<select
					id="locale"
					value={selectedLocale}
					onchange={handleLocaleChange}
					class="w-full h-10 px-3 rounded-md border border-border bg-bg text-fg
								 focus:outline-none focus:ring-2 focus:ring-accent focus:ring-offset-2 focus:ring-offset-bg"
				>
					{#each locales as locale}
						<option value={locale}>{localeNames[locale]}</option>
					{/each}
				</select>
			</div>

			{#if successMessage}
				<div class="p-3 rounded-md bg-success/10 text-success text-sm">
					{successMessage}
				</div>
			{/if}

			{#if errorMessage}
				<div class="p-3 rounded-md bg-error/10 text-error text-sm">
					{errorMessage}
				</div>
			{/if}

			<div class="flex justify-end">
				<Button type="submit" disabled={saving} loading={saving}>
					{saving ? i18n._('settings.profile.saving') : i18n._('settings.profile.save')}
				</Button>
			</div>
		</form>
	</Card>
</div>
