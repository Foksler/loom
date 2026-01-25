<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { getClipsClient, type CreateClipRequest } from '$lib/api/clips';
	import { getApiClient } from '$lib/api/client';
	import type { Org } from '$lib/api/types';
	import { ClipForm } from '$lib/components/clips';
	import { showNotification } from '$lib/components/notifications';
	import { trackButtonClick } from '$lib/analytics';

	const clipsClient = getClipsClient();
	const apiClient = getApiClient();

	let orgs = $state<Org[]>([]);
	let loading = $state(false);

	$effect(() => {
		if (browser) {
			loadOrgs();
		}
	});

	async function loadOrgs() {
		try {
			const response = await apiClient.listOrgs();
			orgs = response.orgs;
		} catch (e) {
			console.error('Failed to load organizations:', e);
		}
	}

	async function handleSubmit(data: CreateClipRequest) {
		loading = true;
		try {
			const clip = await clipsClient.createClip(data);
			showNotification({
				type: 'success',
				title: 'Clip created',
				message: `Clip "${clip.name}" has been created successfully.`,
			});
			await goto(`/clips/${clip.owner}/${clip.name}`);
		} catch (e) {
			showNotification({
				type: 'error',
				title: 'Failed to create clip',
				message: e instanceof Error ? e.message : 'An error occurred',
			});
		} finally {
			loading = false;
		}
	}

	function handleCancel() {
		trackButtonClick('cancel_new_clip');
		goto('/clips');
	}
</script>

<div class="new-clip-page">
	<header class="page-header">
		<h1 class="page-title">New Clip</h1>
		<p class="page-description">Create a new code snippet</p>
	</header>

	<div class="form-container">
		<ClipForm {orgs} {loading} onsubmit={handleSubmit} oncancel={handleCancel} />
	</div>

	<div class="help-text">
		<h3>After creating your clip</h3>
		<p>
			You can push files to your clip using git. Clone the repository and add your code:
		</p>
		<pre><code>git clone &lt;clone-url&gt;
cd &lt;clip-name&gt;
# Add your files
git add .
git commit -m "Add initial files"
git push origin main</code></pre>
	</div>
</div>

<style>
	.new-clip-page {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		max-width: 600px;
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

	.form-container {
		padding: var(--space-6);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.help-text {
		padding: var(--space-4);
		background: var(--color-bg-subtle);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.help-text h3 {
		margin: 0 0 var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.help-text p {
		margin: 0 0 var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.help-text pre {
		margin: 0;
		padding: var(--space-3);
		background: var(--color-bg);
		border-radius: var(--radius-sm);
		overflow-x: auto;
	}

	.help-text code {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
	}
</style>
