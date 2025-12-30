<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { i18n } from '$lib/i18n';
	import { getApiClient } from '$lib/api/client';
	import type { Session } from '$lib/api/types';
	import { Card, Badge, Button } from '$lib/ui';

	let sessions: Session[] = $state([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let revokingId = $state<string | null>(null);

	const client = getApiClient();

	async function loadSessions() {
		loading = true;
		error = null;
		try {
			const response = await client.listSessions();
			sessions = response.sessions;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load sessions';
		} finally {
			loading = false;
		}
	}

	async function revokeSession(sessionId: string) {
		if (!confirm(i18n._('settings.sessions.revokeConfirm'))) {
			return;
		}

		revokingId = sessionId;
		try {
			await client.revokeSession(sessionId);
			sessions = sessions.filter((s) => s.id !== sessionId);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to revoke session';
		} finally {
			revokingId = null;
		}
	}

	function getSessionTypeLabel(type: Session['session_type']): string {
		switch (type) {
			case 'web':
				return i18n._('settings.sessions.web');
			case 'cli':
				return i18n._('settings.sessions.cli');
			case 'vscode':
				return i18n._('settings.sessions.vscode');
		}
	}

	function getSessionTypeVariant(type: Session['session_type']): 'accent' | 'success' | 'warning' {
		switch (type) {
			case 'web':
				return 'accent';
			case 'cli':
				return 'success';
			case 'vscode':
				return 'warning';
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleString();
	}

	$effect(() => {
		loadSessions();
	});
</script>

<svelte:head>
	<title>{i18n._('settings.sessions.title')} - Loom</title>
</svelte:head>

<div>
	<h1 class="text-2xl font-bold text-fg mb-2">
		{i18n._('settings.sessions.title')}
	</h1>
	<p class="text-fg-muted mb-6">
		{i18n._('settings.sessions.description')}
	</p>

	{#if loading}
		<div class="text-fg-muted">{i18n._('general.loading')}</div>
	{:else if error}
		<Card>
			<div class="text-error">{error}</div>
			<Button variant="secondary" onclick={loadSessions} class="mt-2">
				{i18n._('general.retry')}
			</Button>
		</Card>
	{:else if sessions.length === 0}
		<Card>
			<p class="text-fg-muted">{i18n._('settings.sessions.noSessions')}</p>
		</Card>
	{:else}
		<div class="space-y-4">
			{#each sessions as session (session.id)}
				<Card>
					<div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
						<div class="flex-1 min-w-0">
							<div class="flex items-center gap-2 mb-2">
								<Badge variant={getSessionTypeVariant(session.session_type)} size="sm">
									{getSessionTypeLabel(session.session_type)}
								</Badge>
								{#if session.is_current}
									<Badge variant="success" size="sm">
										{i18n._('settings.sessions.current')}
									</Badge>
								{/if}
							</div>

							<div class="space-y-1 text-sm">
								{#if session.ip_address}
									<div class="text-fg">
										{session.ip_address}
										{#if session.geo_location}
											<span class="text-fg-muted"> · {session.geo_location}</span>
										{/if}
									</div>
								{/if}

								{#if session.user_agent}
									<div class="text-fg-muted truncate" title={session.user_agent}>
										{session.user_agent}
									</div>
								{/if}

								<div class="text-fg-muted">
									<span>{i18n._('settings.sessions.lastUsed')}: {formatDate(session.last_used_at)}</span>
									<span class="mx-2">·</span>
									<span>{i18n._('settings.sessions.createdAt')}: {formatDate(session.created_at)}</span>
								</div>
							</div>
						</div>

						{#if !session.is_current}
							<Button
								variant="danger"
								size="sm"
								disabled={revokingId === session.id}
								loading={revokingId === session.id}
								onclick={() => revokeSession(session.id)}
							>
								{i18n._('settings.sessions.revoke')}
							</Button>
						{/if}
					</div>
				</Card>
			{/each}
		</div>
	{/if}
</div>
