<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { i18n } from '$lib/i18n';
	import { getApiClient } from '$lib/api/client';
	import type { Weaver, WeaverStatus, CreateWeaverRequest } from '$lib/api/types';
	import { Card, Badge, Button, Input } from '$lib/ui';

	const client = getApiClient();

	let weavers = $state<Weaver[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let deletingId = $state<string | null>(null);

	let showCreateModal = $state(false);
	let creating = $state(false);
	const DEFAULT_WEAVER_IMAGE = 'ghcr.io/ghuntley/loom/weaver:latest';
	const PRESET_IMAGES = [
		{ value: 'ghcr.io/ghuntley/loom/weaver:latest', label: 'Loom Weaver (latest)' },
		{ value: 'ghcr.io/ghuntley/loom/weaver:nightly', label: 'Loom Weaver (nightly)' },
		{ value: 'ubuntu:24.04', label: 'Ubuntu 24.04' },
		{ value: 'debian:bookworm', label: 'Debian Bookworm' },
		{ value: 'alpine:latest', label: 'Alpine (latest)' },
	];

	let newWeaver = $state({
		image: DEFAULT_WEAVER_IMAGE,
		lifetime_hours: 24,
		workdir: '',
	});
	let showImageDropdown = $state(false);

	async function loadWeavers() {
		loading = true;
		error = null;
		try {
			const response = await client.listWeavers();
			weavers = response.weavers;
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			loading = false;
		}
	}

	async function createWeaver() {
		if (!newWeaver.image) return;
		creating = true;
		error = null;
		try {
			const weaver = await client.createWeaver(newWeaver);
			weavers = [weaver, ...weavers];
			showCreateModal = false;
			newWeaver = { image: DEFAULT_WEAVER_IMAGE, lifetime_hours: 24, workdir: '' };
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			creating = false;
		}
	}

	async function deleteWeaver(id: string) {
		if (!confirm(i18n._('weavers.deleteConfirm'))) return;
		deletingId = id;
		try {
			await client.deleteWeaver(id);
			weavers = weavers.filter((w) => w.id !== id);
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			deletingId = null;
		}
	}

	function getStatusVariant(status: WeaverStatus): 'success' | 'warning' | 'error' | 'muted' {
		switch (status) {
			case 'running':
				return 'success';
			case 'pending':
				return 'warning';
			case 'failed':
				return 'error';
			case 'succeeded':
			case 'terminating':
			default:
				return 'muted';
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleString();
	}

	function formatAge(hours: number | undefined): string {
		if (hours === undefined) return '-';
		if (hours < 1) return `${Math.round(hours * 60)}m`;
		return `${hours.toFixed(1)}h`;
	}

	function closeModal() {
		showCreateModal = false;
		newWeaver = { image: DEFAULT_WEAVER_IMAGE, lifetime_hours: 24, workdir: '' };
		showImageDropdown = false;
	}

	function selectImage(value: string) {
		newWeaver.image = value;
		showImageDropdown = false;
	}

	$effect(() => {
		loadWeavers();
	});
</script>

<svelte:head>
	<title>{i18n._('weavers.title')} - Loom</title>
</svelte:head>

<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
	<div class="flex justify-between items-center mb-6">
		<div>
			<h1 class="text-2xl font-bold text-fg">{i18n._('weavers.title')}</h1>
			<p class="text-fg-muted">{i18n._('weavers.description')}</p>
		</div>
		<div class="flex gap-2">
			<Button variant="secondary" onclick={loadWeavers} disabled={loading}>
				{i18n._('general.refresh')}
			</Button>
			<Button onclick={() => (showCreateModal = true)}>
				{i18n._('weavers.create')}
			</Button>
		</div>
	</div>

	{#if error}
		<div class="mb-4 p-3 rounded-md bg-error/10 text-error text-sm">{error}</div>
	{/if}

	{#if loading && weavers.length === 0}
		<Card>
			<div class="text-fg-muted text-center py-8">{i18n._('general.loading')}</div>
		</Card>
	{:else if weavers.length === 0}
		<Card>
			<div class="text-center py-8">
				<p class="text-fg-muted mb-4">{i18n._('weavers.empty')}</p>
				<Button onclick={() => (showCreateModal = true)}>
					{i18n._('weavers.createFirst')}
				</Button>
			</div>
		</Card>
	{:else}
		<div class="space-y-3">
			{#each weavers as weaver (weaver.id)}
				<Card>
					<div class="flex items-start justify-between gap-4">
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2 mb-1">
								<span class="font-mono text-sm text-fg truncate">{weaver.id}</span>
								<Badge variant={getStatusVariant(weaver.status)} size="sm">
									{weaver.status}
								</Badge>
							</div>
							{#if weaver.image}
								<div class="text-sm text-fg-muted truncate mb-2">
									<span class="font-medium">{i18n._('weavers.image')}:</span> {weaver.image}
								</div>
							{/if}
							<div class="flex flex-wrap gap-4 text-xs text-fg-muted">
								<div>
									<span class="font-medium">{i18n._('weavers.created')}:</span>
									{formatDate(weaver.created_at)}
								</div>
								<div>
									<span class="font-medium">{i18n._('weavers.age')}:</span>
									{formatAge(weaver.age_hours)}
								</div>
								{#if weaver.lifetime_hours}
									<div>
										<span class="font-medium">{i18n._('weavers.lifetime')}:</span>
										{weaver.lifetime_hours}h
									</div>
								{/if}
							</div>
							{#if weaver.tags && Object.keys(weaver.tags).length > 0}
								<div class="flex flex-wrap gap-1 mt-2">
									{#each Object.entries(weaver.tags) as [key, value]}
										<Badge variant="muted" size="sm">{key}: {value}</Badge>
									{/each}
								</div>
							{/if}
						</div>
						<div class="flex gap-2 flex-shrink-0">
							{#if weaver.status === 'running'}
								<a href="/weavers/{weaver.id}">
									<Button variant="primary" size="sm">
										{i18n._('weavers.attach')}
									</Button>
								</a>
							{/if}
							<Button
								variant="danger"
								size="sm"
								disabled={deletingId === weaver.id}
								loading={deletingId === weaver.id}
								onclick={() => deleteWeaver(weaver.id)}
							>
								{i18n._('weavers.delete')}
							</Button>
						</div>
					</div>
				</Card>
			{/each}
		</div>
	{/if}
</div>

{#if showCreateModal}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
		aria-labelledby="create-weaver-title"
		tabindex="-1"
		onclick={closeModal}
		onkeydown={(e) => e.key === 'Escape' && closeModal()}
	>
		<div
			class="bg-bg border border-border rounded-lg p-6 w-full max-w-lg"
			role="document"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<h2 id="create-weaver-title" class="text-lg font-bold text-fg mb-4">
				{i18n._('weavers.createTitle')}
			</h2>

			<form onsubmit={(e) => { e.preventDefault(); createWeaver(); }} class="space-y-4">
				<div class="w-full">
					<label for="image" class="block text-sm font-medium text-fg mb-1.5">
						{i18n._('weavers.imageName')}
					</label>
					<div class="relative">
						<input
							id="image"
							type="text"
							bind:value={newWeaver.image}
							placeholder="ghcr.io/org/image:tag"
							required
							onfocus={() => (showImageDropdown = true)}
							class="w-full h-10 px-3 pr-10 rounded-md border border-border bg-bg text-fg placeholder:text-fg-subtle focus:outline-none focus:ring-2 focus:ring-accent focus:ring-offset-2 focus:ring-offset-bg"
						/>
						<button
							type="button"
							onclick={() => (showImageDropdown = !showImageDropdown)}
							class="absolute right-0 top-0 h-10 w-10 flex items-center justify-center text-fg-muted hover:text-fg"
						>
							<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
							</svg>
						</button>
						{#if showImageDropdown}
							<div class="absolute z-10 w-full mt-1 bg-bg border border-border rounded-md shadow-lg max-h-60 overflow-auto">
								{#each PRESET_IMAGES as preset}
									<button
										type="button"
										onclick={() => selectImage(preset.value)}
										class="w-full px-3 py-2 text-left text-sm hover:bg-bg-muted flex flex-col {newWeaver.image === preset.value ? 'bg-accent/10' : ''}"
									>
										<span class="text-fg font-medium">{preset.label}</span>
										<span class="text-fg-muted text-xs font-mono">{preset.value}</span>
									</button>
								{/each}
							</div>
						{/if}
					</div>
				</div>

				<div class="w-full">
					<label for="lifetime" class="block text-sm font-medium text-fg mb-1.5">
						{i18n._('weavers.lifetimeLabel')}
					</label>
					<select
						id="lifetime"
						bind:value={newWeaver.lifetime_hours}
						class="w-full h-10 px-3 rounded-md border border-border bg-bg text-fg"
					>
						<option value={1}>1 {i18n._('weavers.hour')}</option>
						<option value={4}>4 {i18n._('weavers.hours')}</option>
						<option value={8}>8 {i18n._('weavers.hours')}</option>
						<option value={24}>24 {i18n._('weavers.hours')}</option>
						<option value={48}>48 {i18n._('weavers.hours')}</option>
					</select>
				</div>

				<Input
					label={i18n._('weavers.workdir')}
					bind:value={newWeaver.workdir}
					placeholder="/app"
				/>

				<div class="flex justify-end gap-2 pt-2">
					<Button variant="secondary" type="button" onclick={closeModal}>
						{i18n._('general.cancel')}
					</Button>
					<Button type="submit" disabled={creating || !newWeaver.image} loading={creating}>
						{i18n._('weavers.create')}
					</Button>
				</div>
			</form>
		</div>
	</div>
{/if}
