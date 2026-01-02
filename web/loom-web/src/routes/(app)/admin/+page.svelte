<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { onDestroy } from 'svelte';
	import { i18n } from '$lib/i18n';
	import { Card, Badge } from '$lib/ui';
	import type { HealthResponse, HealthStatus, LogEntry, LogLevel, ListLogsResponse } from '$lib/api/types';

	let health = $state<HealthResponse | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Log state
	let logs = $state<LogEntry[]>([]);
	let logsLoading = $state(true);
	let streaming = $state(false);
	let eventSource: EventSource | null = null;

	interface AdminSection {
		href: string;
		titleKey: string;
		descriptionKey: string;
		icon: string;
	}

	const adminSections: AdminSection[] = [
		{
			href: '/admin/users',
			titleKey: 'admin.users.title',
			descriptionKey: 'admin.users.description',
			icon: '👥',
		},
		{
			href: '/admin/anthropic-accounts',
			titleKey: 'admin.anthropic.title',
			descriptionKey: 'admin.dashboard.anthropic_description',
			icon: '🤖',
		},
		{
			href: '/admin/jobs',
			titleKey: 'jobs.title',
			descriptionKey: 'jobs.description',
			icon: '⚙️',
		},
		{
			href: '/admin/logs',
			titleKey: 'admin.logs.title',
			descriptionKey: 'admin.logs.description',
			icon: '📋',
		},
	];

	async function loadHealth() {
		loading = true;
		error = null;
		try {
			const res = await fetch('/health', { credentials: 'include' });
			if (!res.ok) throw new Error(`Failed to load health: ${res.status}`);
			health = await res.json();
		} catch (e) {
			error = e instanceof Error ? e.message : i18n._('general.error');
		} finally {
			loading = false;
		}
	}

	async function loadLogs() {
		logsLoading = true;
		try {
			const res = await fetch('/api/admin/logs?limit=50', { credentials: 'include' });
			if (!res.ok) throw new Error(`Failed to load logs: ${res.status}`);
			const data: ListLogsResponse = await res.json();
			logs = data.entries;
			// Auto-start streaming after initial load
			startStreaming();
		} catch {
			// Ignore log load errors on dashboard
		} finally {
			logsLoading = false;
		}
	}

	function startStreaming() {
		if (eventSource) return;

		eventSource = new EventSource('/api/admin/logs/stream', { withCredentials: true });

		eventSource.onmessage = (event) => {
			try {
				const entry: LogEntry = JSON.parse(event.data);
				logs = [...logs, entry].slice(-100);
				requestAnimationFrame(() => {
					const container = document.getElementById('dashboard-log-container');
					if (container) {
						container.scrollTop = container.scrollHeight;
					}
				});
			} catch {
				// Ignore parse errors
			}
		};

		eventSource.onerror = () => {
			stopStreaming();
		};

		streaming = true;
	}

	function stopStreaming() {
		if (eventSource) {
			eventSource.close();
			eventSource = null;
		}
		streaming = false;
	}

	function getStatusVariant(status: HealthStatus): 'success' | 'warning' | 'error' | 'muted' {
		switch (status) {
			case 'healthy':
				return 'success';
			case 'degraded':
				return 'warning';
			case 'unhealthy':
				return 'error';
			default:
				return 'muted';
		}
	}

	function getStatusIcon(status: HealthStatus): string {
		switch (status) {
			case 'healthy':
				return '✓';
			case 'degraded':
				return '!';
			case 'unhealthy':
				return '✕';
			default:
				return '?';
		}
	}

	function formatLatency(ms: number): string {
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	function getLevelVariant(level: LogLevel): 'muted' | 'accent' | 'success' | 'warning' | 'error' {
		switch (level) {
			case 'trace':
			case 'debug':
				return 'muted';
			case 'info':
				return 'accent';
			case 'warn':
				return 'warning';
			case 'error':
				return 'error';
			default:
				return 'muted';
		}
	}

	function formatTimestamp(ts: string): string {
		const date = new Date(ts);
		return date.toLocaleTimeString('en-US', {
			hour12: false,
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit',
		});
	}

	interface ComponentInfo {
		name: string;
		status: HealthStatus;
		latency?: number;
		configured?: boolean;
		error?: string;
		extra?: string;
	}

	function getComponents(): ComponentInfo[] {
		if (!health) return [];

		const components: ComponentInfo[] = [];
		const c = health.components;

		components.push({
			name: i18n._('admin.health.database'),
			status: c.database.status,
			latency: c.database.latency_ms,
			error: c.database.error,
		});

		components.push({
			name: i18n._('admin.health.bin_dir'),
			status: c.bin_dir.status,
			latency: c.bin_dir.latency_ms,
			error: c.bin_dir.error,
			extra: c.bin_dir.file_count !== undefined ? `${c.bin_dir.file_count} files` : undefined,
		});

		components.push({
			name: i18n._('admin.health.llm_providers'),
			status: c.llm_providers.status,
			extra:
				c.llm_providers.providers.length > 0
					? c.llm_providers.providers.map((p) => p.name).join(', ')
					: undefined,
		});

		components.push({
			name: i18n._('admin.health.google_cse'),
			status: c.google_cse.status,
			latency: c.google_cse.latency_ms,
			configured: c.google_cse.configured,
			error: c.google_cse.error,
		});

		components.push({
			name: i18n._('admin.health.github_app'),
			status: c.github_app.status,
			latency: c.github_app.latency_ms,
			configured: c.github_app.configured,
			error: c.github_app.error,
		});

		if (c.kubernetes) {
			components.push({
				name: i18n._('admin.health.kubernetes'),
				status: c.kubernetes.status,
				latency: c.kubernetes.latency_ms,
				error: c.kubernetes.error,
				extra: c.kubernetes.namespace,
			});
		}

		components.push({
			name: i18n._('admin.health.smtp'),
			status: c.smtp.status,
			latency: c.smtp.latency_ms,
			configured: c.smtp.configured,
			error: c.smtp.error,
		});

		components.push({
			name: i18n._('admin.health.geoip'),
			status: c.geoip.status,
			latency: c.geoip.latency_ms,
			configured: c.geoip.configured,
			error: c.geoip.error,
			extra: c.geoip.database_type,
		});

		if (c.jobs) {
			components.push({
				name: i18n._('admin.health.jobs'),
				status: c.jobs.status,
				extra: `${c.jobs.jobs_healthy}/${c.jobs.jobs_total} healthy`,
				error: c.jobs.failing_jobs?.join(', '),
			});
		}

		return components;
	}

	$effect(() => {
		loadHealth();
		loadLogs();
		const interval = setInterval(loadHealth, 30000);
		return () => {
			clearInterval(interval);
			stopStreaming();
		};
	});

	onDestroy(() => {
		stopStreaming();
	});
</script>

<svelte:head>
	<title>{i18n._('admin.dashboard.title')} - Loom</title>
</svelte:head>

<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
	<div class="mb-8">
		<h1 class="text-2xl font-bold text-fg">{i18n._('admin.dashboard.title')}</h1>
		<p class="text-fg-muted">{i18n._('admin.dashboard.description')}</p>
	</div>

	<!-- Admin Sections Navigation -->
	<div class="mb-8">
		<h2 class="text-lg font-semibold text-fg mb-4">{i18n._('admin.dashboard.sections')}</h2>
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
			{#each adminSections as section}
				<a href={section.href} class="block group">
					<Card hover={true}>
						<div class="flex items-start gap-3">
							<span class="text-2xl">{section.icon}</span>
							<div>
								<h3 class="font-medium text-fg group-hover:text-accent">
									{i18n._(section.titleKey)}
								</h3>
								<p class="text-sm text-fg-muted">
									{i18n._(section.descriptionKey)}
								</p>
							</div>
						</div>
					</Card>
				</a>
			{/each}
		</div>
	</div>

	<!-- Two column layout: Health + Logs -->
	<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
		<!-- Server Health -->
		<div>
			<div class="flex items-center justify-between mb-4">
				<h2 class="text-lg font-semibold text-fg">{i18n._('admin.health.title')}</h2>
				<button
					onclick={() => loadHealth()}
					disabled={loading}
					class="text-sm text-accent hover:text-accent-hover disabled:opacity-50"
				>
					{i18n._('general.refresh')}
				</button>
			</div>

			{#if error}
				<div class="mb-4 p-3 rounded-md bg-error/10 text-error text-sm">{error}</div>
			{/if}

			{#if loading && !health}
				<Card>
					<div class="text-fg-muted text-center py-8">{i18n._('general.loading')}</div>
				</Card>
			{:else if health}
				<!-- Overall Status -->
				<div class="mb-4">
				<Card>
					<div class="flex items-center gap-4">
						<div
							class="w-10 h-10 rounded-full flex items-center justify-center text-lg font-bold"
							class:bg-success={health.status === 'healthy'}
							class:bg-warning={health.status === 'degraded'}
							class:bg-error={health.status === 'unhealthy'}
							class:bg-fg-muted={health.status === 'unknown'}
							class:text-white={true}
						>
							{getStatusIcon(health.status)}
						</div>
						<div class="flex-1">
							<div class="flex items-center gap-2">
								<Badge variant={getStatusVariant(health.status)}>
									{i18n._(`admin.health.status.${health.status}`)}
								</Badge>
								<span class="text-xs text-fg-muted">
									{health.version.git_sha}
								</span>
							</div>
							<div class="text-xs text-fg-muted mt-1">
								{formatLatency(health.duration_ms)} &middot; {new Date(health.timestamp).toLocaleTimeString()}
							</div>
						</div>
					</div>
				</Card>
				</div>

				<!-- Component Grid -->
				<div class="grid grid-cols-2 gap-2">
					{#each getComponents() as component}
						<Card padding="sm">
							<div class="flex items-center gap-2">
								<Badge variant={getStatusVariant(component.status)} size="sm">
									{component.status.charAt(0).toUpperCase()}
								</Badge>
								<span class="text-sm text-fg truncate">{component.name}</span>
							</div>
							{#if component.error}
								<div class="text-xs text-error mt-1 truncate" title={component.error}>
									{component.error}
								</div>
							{/if}
						</Card>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Live Logs -->
		<div>
			<div class="flex items-center justify-between mb-4">
				<h2 class="text-lg font-semibold text-fg">
					{i18n._('admin.logs.title')}
					{#if streaming}
						<span class="inline-block w-2 h-2 rounded-full bg-success animate-pulse ml-2"></span>
					{/if}
				</h2>
				<a href="/admin/logs" class="text-sm text-accent hover:text-accent-hover">
					{i18n._('admin.dashboard.view_all_logs')}
				</a>
			</div>

			<div
				id="dashboard-log-container"
				class="bg-gray-900 rounded-lg border border-border overflow-auto font-mono text-xs"
				style="height: 400px;"
			>
				{#if logsLoading && logs.length === 0}
					<div class="text-gray-400 text-center py-8">{i18n._('general.loading')}</div>
				{:else if logs.length === 0}
					<div class="text-gray-400 text-center py-8">{i18n._('admin.logs.no_logs')}</div>
				{:else}
					<div class="p-2 space-y-0.5">
						{#each logs as log (log.id)}
							<div class="flex gap-2 hover:bg-gray-800/50 px-1 rounded">
								<span class="text-gray-500 shrink-0">{formatTimestamp(log.timestamp)}</span>
								<span
									class="shrink-0 w-12"
									class:text-gray-500={log.level === 'trace' || log.level === 'debug'}
									class:text-blue-400={log.level === 'info'}
									class:text-yellow-400={log.level === 'warn'}
									class:text-red-400={log.level === 'error'}
								>
									{log.level.toUpperCase().padEnd(5)}
								</span>
								<span class="text-gray-200 break-all">{log.message}</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>
