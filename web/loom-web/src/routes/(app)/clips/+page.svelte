<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { browser } from '$app/environment';
	import { getClipsClient, type Clip, type ClipVisibility } from '$lib/api/clips';
	import { getApiClient } from '$lib/api/client';
	import type { Org, CurrentUser } from '$lib/api/types';
	import { ClipList } from '$lib/components/clips';
	import { Button } from '$lib/ui';
	import { trackLinkClick, trackFilterChange, trackButtonClick } from '$lib/analytics';

	const clipsClient = getClipsClient();
	const apiClient = getApiClient();

	let currentUser = $state<CurrentUser | null>(null);
	let orgs = $state<Org[]>([]);
	let clips = $state<Clip[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let activeTab = $state<'my' | 'starred' | 'explore'>('my');
	let languageFilter = $state<string>('');
	let searchQuery = $state<string>('');
	let isSearching = $state(false);

	$effect(() => {
		if (browser) {
			loadCurrentUser();
			loadOrgs();
		}
	});

	$effect(() => {
		if (browser && (currentUser || activeTab !== 'my')) {
			loadClips();
		}
	});

	async function loadCurrentUser() {
		try {
			currentUser = await apiClient.getCurrentUser();
		} catch (e) {
			console.error('Failed to load current user:', e);
		}
	}

	async function loadOrgs() {
		try {
			const response = await apiClient.listOrgs();
			orgs = response.orgs;
		} catch (e) {
			console.error('Failed to load organizations:', e);
		}
	}

	async function loadClips() {
		loading = true;
		error = null;
		isSearching = false;
		try {
			const params = languageFilter ? { language: languageFilter } : {};
			let response;
			if (activeTab === 'my' && currentUser) {
				response = await clipsClient.listUserClips(currentUser.id, params);
			} else if (activeTab === 'starred') {
				response = await clipsClient.listStarredClips(params);
			} else {
				response = await clipsClient.listPublicClips(params);
			}
			clips = response.clips;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load clips';
		} finally {
			loading = false;
		}
	}

	async function handleSearch() {
		if (!searchQuery.trim()) {
			isSearching = false;
			loadClips();
			return;
		}

		loading = true;
		error = null;
		isSearching = true;
		trackButtonClick('search_clips', { query: searchQuery });

		try {
			const response = await clipsClient.searchClips({ q: searchQuery });
			clips = response.hits.map((h) => h.clip);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Search failed';
		} finally {
			loading = false;
		}
	}

	function handleSearchInput(e: Event) {
		const target = e.target as HTMLInputElement;
		searchQuery = target.value;
		if (!searchQuery.trim()) {
			isSearching = false;
			loadClips();
		}
	}

	function handleSearchKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			handleSearch();
		}
	}

	function handleClipClick(clip: Clip) {
		trackLinkClick('clip', `/clips/${clip.owner}/${clip.name}`, {
			clip_id: clip.id,
			owner: clip.owner,
			name: clip.name,
		});
		window.location.href = `/clips/${clip.owner}/${clip.name}`;
	}

	function handleTabChange(tab: 'my' | 'starred' | 'explore') {
		trackFilterChange('clips_tab', tab, { page: 'clips' });
		activeTab = tab;
		searchQuery = '';
		isSearching = false;
	}

	function handleLanguageChange(e: Event) {
		const target = e.target as HTMLInputElement;
		languageFilter = target.value;
		trackFilterChange('clips_language', languageFilter || 'all', { page: 'clips' });
		loadClips();
	}
</script>

<div class="clips-page">
	<header class="page-header">
		<div class="header-content">
			<div>
				<h1 class="page-title">Clips</h1>
				<p class="page-description">Code snippets and examples</p>
			</div>
			<Button href="/clips/new" onclick={() => trackButtonClick('new_clip')}>New Clip</Button>
		</div>
	</header>

	<div class="tabs">
		<button
			class="tab"
			class:active={activeTab === 'my'}
			onclick={() => handleTabChange('my')}
		>
			My Clips
		</button>
		<button
			class="tab"
			class:active={activeTab === 'starred'}
			onclick={() => handleTabChange('starred')}
		>
			Starred
		</button>
		<button
			class="tab"
			class:active={activeTab === 'explore'}
			onclick={() => handleTabChange('explore')}
		>
			Explore
		</button>
	</div>

	<div class="filters">
		{#if activeTab === 'explore'}
			<div class="filter-group search-group">
				<input
					id="search-input"
					type="text"
					class="filter-input search-input"
					placeholder="Search public clips..."
					value={searchQuery}
					oninput={handleSearchInput}
					onkeydown={handleSearchKeydown}
				/>
				<Button size="sm" onclick={handleSearch}>Search</Button>
			</div>
		{/if}
		<div class="filter-group">
			<label class="filter-label" for="language-filter">Language</label>
			<input
				id="language-filter"
				type="text"
				class="filter-input"
				placeholder="e.g., rust, python"
				value={languageFilter}
				oninput={handleLanguageChange}
			/>
		</div>
	</div>

	{#if loading}
		<div class="loading">Loading clips...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if clips.length === 0}
		<div class="empty-state">
			{#if isSearching}
				<h2>No results</h2>
				<p>No clips found matching "{searchQuery}".</p>
			{:else if activeTab === 'my'}
				<h2>No clips yet</h2>
				<p>Create your first clip to get started.</p>
				<Button href="/clips/new" onclick={() => trackButtonClick('create_clip_empty_state')}>Create Clip</Button>
			{:else if activeTab === 'starred'}
				<h2>No starred clips</h2>
				<p>Star clips to save them here.</p>
			{:else}
				<h2>No public clips</h2>
				<p>Be the first to share a public clip!</p>
			{/if}
		</div>
	{:else}
		<ClipList {clips} onclipclick={handleClipClick} />
	{/if}
</div>

<style>
	.clips-page {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		max-width: 1200px;
		margin: 0 auto;
		padding: var(--space-6);
	}

	.page-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
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

	.tabs {
		display: flex;
		gap: var(--space-1);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.tab {
		padding: var(--space-3) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-fg-muted);
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		cursor: pointer;
		transition: all 0.15s ease;
	}

	.tab:hover {
		color: var(--color-fg);
	}

	.tab.active {
		color: var(--color-accent);
		border-bottom-color: var(--color-accent);
	}

	.filters {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		flex-wrap: wrap;
	}

	.filter-group {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.filter-label {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.filter-input {
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
	}

	.search-group {
		flex: 1;
		max-width: 400px;
	}

	.search-input {
		flex: 1;
	}

	.loading,
	.error {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		text-align: center;
		padding: var(--space-8);
	}

	.error {
		color: var(--color-error);
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		text-align: center;
		padding: var(--space-8);
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
	}

	.empty-state h2 {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-fg);
	}

	.empty-state p {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}
</style>
