<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { goto } from '$app/navigation';
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
	let createdWeaverId = $state<string | null>(null);
	let createLogLines = $state<string[]>([]);
	let createEventSource: EventSource | null = null;
	let showLogsModal = $state(false);
	let logsWeaver = $state<Weaver | null>(null);
	let logLines = $state<string[]>([]);
	let logsError = $state<string | null>(null);
	let logsConnecting = $state(false);
	let logsEventSource: EventSource | null = null;
	const DEFAULT_WEAVER_IMAGE = 'ghcr.io/ghuntley/loom/weaver:latest';
	const PRESET_IMAGES = [
		{ value: 'ghcr.io/ghuntley/loom/weaver:latest', label: 'Loom Weaver (latest)' },
		{ value: 'nixos/nix:latest', label: 'NixOS (latest)' },
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
		createLogLines = [];
		createdWeaverId = null;

		try {
			const weaver = await client.createWeaver(newWeaver);
			createdWeaverId = weaver.id;

			const url = `/api/weaver/${encodeURIComponent(weaver.id)}/logs?tail=100&timestamps=true`;
			createEventSource = new EventSource(url);

			createEventSource.onmessage = (event) => {
				if (event.data && event.data !== 'keep-alive') {
					createLogLines = [...createLogLines, event.data];
				}
			};

			createEventSource.onerror = () => {
				createEventSource?.close();
				createEventSource = null;
			};

			const pollForRunning = async () => {
				const maxAttempts = 60;
				for (let i = 0; i < maxAttempts; i++) {
					try {
						const updated = await client.getWeaver(weaver.id);
						if (updated.status === 'running') {
							cleanupCreateState();
							goto(`/weavers/${weaver.id}`);
							return;
						}
						if (updated.status === 'failed') {
							error = i18n._('weavers.createFailed');
							creating = false;
							createEventSource?.close();
							createEventSource = null;
							return;
						}
					} catch {
						// Ignore polling errors, keep trying
					}
					await new Promise((r) => setTimeout(r, 1000));
				}
				error = i18n._('weavers.createTimeout');
				creating = false;
				createEventSource?.close();
				createEventSource = null;
			};

			pollForRunning();
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
			creating = false;
		}
	}

	function cleanupCreateState() {
		showCreateModal = false;
		creating = false;
		createdWeaverId = null;
		createLogLines = [];
		newWeaver = { image: DEFAULT_WEAVER_IMAGE, lifetime_hours: 24, workdir: '' };
		if (createEventSource) {
			createEventSource.close();
			createEventSource = null;
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
		if (creating && createdWeaverId) {
			return;
		}
		showCreateModal = false;
		newWeaver = { image: DEFAULT_WEAVER_IMAGE, lifetime_hours: 24, workdir: '' };
		showImageDropdown = false;
		createdWeaverId = null;
		createLogLines = [];
		creating = false;
		if (createEventSource) {
			createEventSource.close();
			createEventSource = null;
		}
	}

	function selectImage(value: string) {
		newWeaver.image = value;
		showImageDropdown = false;
	}

	function openLogsModal(weaver: Weaver) {
		logsWeaver = weaver;
		logLines = [];
		logsError = null;
		logsConnecting = true;
		showLogsModal = true;

		const url = `/api/weaver/${encodeURIComponent(weaver.id)}/logs?tail=500&timestamps=true`;
		logsEventSource = new EventSource(url);

		logsEventSource.onopen = () => {
			logsConnecting = false;
		};

		logsEventSource.onmessage = (event) => {
			logsConnecting = false;
			if (event.data && event.data !== 'keep-alive') {
				logLines = [...logLines, event.data];
			}
		};

		logsEventSource.onerror = () => {
			logsConnecting = false;
			if (logsEventSource?.readyState === EventSource.CLOSED) {
				logsError = i18n._('weavers.logsClosed');
			} else {
				logsError = i18n._('weavers.logsError');
			}
			logsEventSource?.close();
			logsEventSource = null;
		};
	}

	function closeLogsModal() {
		showLogsModal = false;
		logsWeaver = null;
		logLines = [];
		logsError = null;
		logsConnecting = false;
		if (logsEventSource) {
			logsEventSource.close();
			logsEventSource = null;
		}
	}

	$effect(() => {
		loadWeavers();
		return () => {
			if (logsEventSource) {
				logsEventSource.close();
				logsEventSource = null;
			}
			if (createEventSource) {
				createEventSource.close();
				createEventSource = null;
			}
		};
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
							<Button
								variant="secondary"
								size="sm"
								onclick={() => openLogsModal(weaver)}
							>
								{i18n._('weavers.logs')}
							</Button>
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
			class="bg-bg border border-border rounded-lg w-full max-w-lg flex flex-col {createdWeaverId ? 'max-h-[80vh]' : ''}"
			role="document"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="p-6 {createdWeaverId ? 'pb-0' : ''}">
				<h2 id="create-weaver-title" class="text-lg font-bold text-fg mb-4">
					{createdWeaverId ? i18n._('weavers.creatingTitle') : i18n._('weavers.createTitle')}
				</h2>

				{#if createdWeaverId}
					<div class="mb-4">
						<p class="text-sm text-fg-muted mb-2">{i18n._('weavers.creatingProgress')}</p>
						<p class="text-xs font-mono text-fg-muted">{createdWeaverId}</p>
					</div>
				{:else}
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
				{/if}
			</div>

			{#if createdWeaverId}
				<div class="flex-1 overflow-auto mx-6 my-4 p-3 bg-black rounded-md min-h-[200px] max-h-[300px]">
					{#if createLogLines.length === 0}
						<p class="text-fg-muted text-sm">{i18n._('weavers.logsConnecting')}</p>
					{:else}
						<pre class="font-mono text-xs text-green-400 whitespace-pre-wrap break-all">{createLogLines.join('\n')}</pre>
					{/if}
				</div>
				<div class="p-6 pt-0 border-t border-border mt-auto">
					<p class="text-xs text-fg-muted text-center">{i18n._('weavers.creatingWait')}</p>
				</div>
			{/if}
		</div>
	</div>
{/if}

{#if showLogsModal && logsWeaver}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
		aria-labelledby="logs-modal-title"
		tabindex="-1"
		onclick={closeLogsModal}
		onkeydown={(e) => e.key === 'Escape' && closeLogsModal()}
	>
		<div
			class="bg-bg border border-border rounded-lg w-full max-w-4xl max-h-[80vh] flex flex-col"
			role="document"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="flex items-center justify-between p-4 border-b border-border">
				<div>
					<h2 id="logs-modal-title" class="text-lg font-bold text-fg">
						{i18n._('weavers.logsTitle')}
					</h2>
					<p class="text-sm text-fg-muted font-mono">{logsWeaver.id}</p>
				</div>
				<button
					type="button"
					onclick={closeLogsModal}
					class="text-fg-muted hover:text-fg p-1"
					aria-label={i18n._('general.close')}
				>
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</button>
			</div>

			<div class="flex-1 overflow-auto p-4 bg-black">
				{#if logsConnecting}
					<p class="text-fg-muted text-sm">{i18n._('weavers.logsConnecting')}</p>
				{:else if logsError}
					<p class="text-error text-sm">{logsError}</p>
				{:else if logLines.length === 0}
					<p class="text-fg-muted text-sm">{i18n._('weavers.logsNoData')}</p>
				{:else}
					<pre class="font-mono text-xs text-green-400 whitespace-pre-wrap break-all">{logLines.join('\n')}</pre>
				{/if}
			</div>

			<div class="flex justify-end p-4 border-t border-border">
				<Button variant="secondary" onclick={closeLogsModal}>
					{i18n._('general.close')}
				</Button>
			</div>
		</div>
	</div>
{/if}
