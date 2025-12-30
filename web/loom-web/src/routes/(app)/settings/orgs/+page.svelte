<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { i18n } from '$lib/i18n';
	import { getApiClient } from '$lib/api/client';
	import type { Org } from '$lib/api/types';
	import { Card, Badge, Button } from '$lib/ui';

	let orgs: Org[] = $state([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	const client = getApiClient();

	async function loadOrgs() {
		loading = true;
		error = null;
		try {
			const response = await client.listOrgs();
			orgs = response.orgs;
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('settings.orgs.loadError');
		} finally {
			loading = false;
		}
	}

	function getVisibilityVariant(visibility: Org['visibility']): 'accent' | 'success' | 'muted' {
		switch (visibility) {
			case 'public':
				return 'success';
			case 'unlisted':
				return 'accent';
			case 'private':
				return 'muted';
		}
	}

	function getVisibilityLabel(visibility: Org['visibility']): string {
		switch (visibility) {
			case 'public':
				return i18n._('settings.orgs.visibility.public');
			case 'unlisted':
				return i18n._('settings.orgs.visibility.unlisted');
			case 'private':
				return i18n._('settings.orgs.visibility.private');
		}
	}

	$effect(() => {
		loadOrgs();
	});
</script>

<svelte:head>
	<title>{i18n._('settings.orgs.title')} - Loom</title>
</svelte:head>

<div>
	<div class="flex items-center justify-between mb-6">
		<div>
			<h1 class="text-2xl font-bold text-fg">
				{i18n._('settings.orgs.title')}
			</h1>
			<p class="text-fg-muted mt-1">
				{i18n._('settings.orgs.description')}
			</p>
		</div>
		<a href="/settings/orgs/new">
			<Button>
				{i18n._('settings.orgs.create')}
			</Button>
		</a>
	</div>

	{#if loading}
		<div class="text-fg-muted">{i18n._('general.loading')}</div>
	{:else if error}
		<Card>
			<div class="text-error">{error}</div>
			<Button variant="secondary" onclick={loadOrgs} class="mt-2">
				{i18n._('general.retry')}
			</Button>
		</Card>
	{:else if orgs.length === 0}
		<Card>
			<p class="text-fg-muted">{i18n._('settings.orgs.noOrgs')}</p>
		</Card>
	{:else}
		<div class="space-y-4">
			{#each orgs as org (org.id)}
				<a href="/settings/orgs/{org.id}" class="block">
					<Card hover>
						<div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-2 mb-2">
									<span class="font-semibold text-fg">{org.name}</span>
									{#if org.is_personal}
										<Badge variant="muted" size="sm">
											{i18n._('settings.orgs.personal')}
										</Badge>
									{/if}
									<Badge variant={getVisibilityVariant(org.visibility)} size="sm">
										{getVisibilityLabel(org.visibility)}
									</Badge>
								</div>

								<div class="space-y-1 text-sm">
									<div class="text-fg-muted">
										{i18n._('settings.orgs.slug')}: <span class="font-mono">{org.slug}</span>
									</div>
									{#if org.member_count !== null}
										<div class="text-fg-muted">
											{org.member_count} {org.member_count === 1 ? i18n._('settings.orgs.member') : i18n._('settings.orgs.members')}
										</div>
									{/if}
								</div>
							</div>
						</div>
					</Card>
				</a>
			{/each}
		</div>
	{/if}
</div>
