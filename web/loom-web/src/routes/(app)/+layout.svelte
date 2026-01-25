<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/auth';
	import { getApiClient } from '$lib/api/client';
	import { i18n, setLocale, getCurrentLocale, isRtl, type Locale, locales } from '$lib/i18n';
	import { ImpersonationBanner, ThreadDivider } from '$lib/ui';
	import { NotificationProvider } from '$lib/components/notifications';
	import { AnalyticsProvider, reset as analyticsReset, capture } from '$lib/analytics';
	import type { Snippet } from 'svelte';
	import type { CurrentUser, ImpersonationState } from '$lib/api/types';

	function trackNavClick(item: string, path: string) {
		capture('nav_clicked', { item, path });
	}

	interface Props {
		children: Snippet;
		data: { user: CurrentUser };
	}

	let { children, data }: Props = $props();

	let impersonationState = $state<ImpersonationState | null>(null);

	const isSystemAdmin = $derived(data.user?.global_roles?.includes('system_admin') ?? false);

	$effect(() => {
		authStore.start();
		authStore.loginSuccess(data.user);

		// Sync locale from user profile if set and different from current
		if (data.user?.locale && locales.includes(data.user.locale as Locale)) {
			const userLocale = data.user.locale as Locale;
			if (getCurrentLocale() !== userLocale) {
				setLocale(userLocale);
				// Update document direction for RTL languages
				if (typeof document !== 'undefined') {
					document.documentElement.dir = isRtl(userLocale) ? 'rtl' : 'ltr';
					document.documentElement.lang = userLocale;
				}
			}
		}
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
			// Reset analytics identity on logout
			analyticsReset();
			const client = getApiClient();
			await client.logout();
			await goto('/login');
		} catch {
			await goto('/login');
		}
	}
</script>

<AnalyticsProvider user={data.user ? { id: data.user.id, email: data.user.email, display_name: data.user.display_name } : null}>
<div class="app-layout">
	{#if impersonationState?.is_impersonating}
		<ImpersonationBanner impersonation={impersonationState} onStop={loadImpersonationState} />
	{/if}

	<header class="app-header">
		<div class="header-content">
			<div class="header-left">
				<a href="/threads" class="logo" onclick={() => trackNavClick('logo', '/threads')}>
					Loom
				</a>
				<nav class="nav">
					<a href="/threads" class="nav-link" onclick={() => trackNavClick('threads', '/threads')}>
						{i18n._('nav.threads')}
					</a>
					<a href="/repos" class="nav-link" onclick={() => trackNavClick('repos', '/repos')}>
						Repos
					</a>
					<a href="/clips" class="nav-link" onclick={() => trackNavClick('clips', '/clips')}>
						Clips
					</a>
					<a href="/weavers" class="nav-link" onclick={() => trackNavClick('weavers', '/weavers')}>
						{i18n._('nav.weavers')}
					</a>
					<div class="nav-group">
						<span class="nav-group-label">{i18n._('nav.observability')}</span>
						<div class="nav-group-links">
							<a href="/analytics" class="nav-link" onclick={() => trackNavClick('analytics', '/analytics')}>
								Analytics
							</a>
							<a href="/crashes" class="nav-link" onclick={() => trackNavClick('crashes', '/crashes')}>
								{i18n._('nav.crashes')}
							</a>
							<a href="/crons" class="nav-link" onclick={() => trackNavClick('crons', '/crons')}>
								{i18n._('nav.crons')}
							</a>
							<a href="/sessions" class="nav-link" onclick={() => trackNavClick('sessions', '/sessions')}>
								{i18n._('nav.sessions')}
							</a>
						</div>
					</div>
					<a href="/settings/profile" class="nav-link" onclick={() => trackNavClick('settings', '/settings/profile')}>
						{i18n._('nav.settings')}
					</a>
					{#if isSystemAdmin}
						<a href="/admin" class="nav-link nav-link-admin" onclick={() => trackNavClick('admin', '/admin')}>
							{i18n._('nav.admin')}
						</a>
					{/if}
				</nav>
			</div>
			
			<div class="header-right">
				{#if data.user}
					<div class="user-info">
						{#if data.user.avatar_url}
							<img 
								src={data.user.avatar_url} 
								alt="" 
								class="avatar"
							/>
						{:else}
							<div class="avatar avatar-placeholder">
								<span>
									{data.user.display_name?.charAt(0).toUpperCase() ?? '?'}
								</span>
							</div>
						{/if}
						<span class="user-name">
							{data.user.display_name}
						</span>
					</div>
					<button onclick={() => { trackNavClick('logout', '/login'); handleLogout(); }} class="logout-btn">
						{i18n._('auth.signOut')}
					</button>
				{/if}
			</div>
		</div>
		<ThreadDivider variant="simple" class="header-divider" />
	</header>

	<main class="app-main">
		{@render children()}
	</main>
	<NotificationProvider />
</div>
</AnalyticsProvider>

<style>
	.app-layout {
		min-height: 100vh;
		background: var(--color-bg);
	}

	.app-header {
		background: var(--color-bg-muted);
		border-bottom: 1px solid var(--color-border);
	}

	.header-content {
		max-width: 80rem;
		margin: 0 auto;
		padding: 0 var(--space-4);
		display: flex;
		justify-content: space-between;
		align-items: center;
		height: 64px;
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: var(--space-8);
	}

	.logo {
		font-size: var(--text-xl);
		font-weight: 600;
		color: var(--color-fg);
		text-decoration: none;
		font-family: var(--font-mono);
	}

	.logo:hover {
		color: var(--color-accent);
	}

	.nav {
		display: flex;
		align-items: center;
		gap: var(--space-6);
	}

	.nav-link {
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
		text-decoration: none;
		font-family: var(--font-mono);
		transition: color 0.15s ease;
	}

	.nav-link:hover {
		color: var(--color-fg);
	}

	.nav-link-admin {
		color: var(--color-warning);
	}

	.nav-link-admin:hover {
		color: var(--color-warning);
		opacity: 0.8;
	}

	.nav-group {
		position: relative;
		display: flex;
		align-items: center;
	}

	.nav-group-label {
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
		font-family: var(--font-mono);
		cursor: pointer;
		transition: color 0.15s ease;
	}

	.nav-group-label:hover {
		color: var(--color-fg);
	}

	.nav-group-label::after {
		content: '▾';
		margin-left: var(--space-1);
		font-size: 0.7em;
	}

	.nav-group-links {
		display: none;
		position: absolute;
		top: 100%;
		left: 0;
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: var(--space-2);
		min-width: 120px;
		z-index: 100;
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
	}

	.nav-group:hover .nav-group-links {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.nav-group-links .nav-link {
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-sm);
		white-space: nowrap;
	}

	.nav-group-links .nav-link:hover {
		background: var(--color-bg-subtle);
	}

	.header-right {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}

	.user-info {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: var(--radius-full);
	}

	.avatar-placeholder {
		background: var(--color-bg-subtle);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.avatar-placeholder span {
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-fg-muted);
		font-family: var(--font-mono);
	}

	.user-name {
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
		font-family: var(--font-mono);
	}

	.logout-btn {
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
		background: transparent;
		border: none;
		cursor: pointer;
		font-family: var(--font-mono);
		transition: color 0.15s ease;
	}

	.logout-btn:hover {
		color: var(--color-fg);
	}

	.app-main {
		background: var(--color-bg);
	}

	:global(.header-divider) {
		margin: 0;
	}
</style>
