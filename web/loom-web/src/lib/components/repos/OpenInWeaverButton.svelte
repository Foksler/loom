<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { getApiClient } from '$lib/api/client';
	import type { Repository } from '$lib/api/repos';
	import { i18n } from '$lib/i18n';
	import { Button } from '$lib/ui';

	interface Props {
		repo: Repository;
	}

	let { repo }: Props = $props();

	let loading = $state(false);
	let error = $state<string | null>(null);

	async function openInWeaver() {
		loading = true;
		error = null;

		try {
			const client = getApiClient();
			const weaver = await client.createWeaver({
				image: 'ghcr.io/ghuntley/loom/weaver:latest',
				lifetime_hours: 24,
				env: {
					REPO_CLONE_URL: repo.clone_url,
					REPO_NAME: repo.name,
				},
				tags: {
					repo_id: repo.id,
					repo_name: `${repo.owner_id}/${repo.name}`,
				},
			});

			goto(`/weavers/${weaver.id}`);
		} catch (e) {
			error = e instanceof Error ? e.message : i18n.t('client.repos.weaver.create_failed');
			loading = false;
		}
	}
</script>

<div class="relative">
	<Button variant="primary" size="sm" onclick={openInWeaver} disabled={loading} {loading}>
		<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
		</svg>
		{i18n.t('client.repos.weaver.open')}
	</Button>

	{#if error}
		<div class="absolute right-0 mt-2 p-3 bg-error-soft border border-error rounded-lg text-sm text-error max-w-xs z-10">
			{error}
		</div>
	{/if}
</div>
