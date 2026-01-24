<script lang="ts">
	import Stacktrace from './Stacktrace.svelte';
	import Breadcrumbs from './Breadcrumbs.svelte';
	import ActiveFlags from './ActiveFlags.svelte';
	import UserContext from './UserContext.svelte';
	import { RelativeTime, CopyButton } from '$lib/components/common';

	type BreadcrumbLevel = 'debug' | 'info' | 'warning' | 'error';

	interface Frame {
		function?: string;
		module?: string;
		filename?: string;
		abs_path?: string;
		lineno?: number;
		colno?: number;
		context_line?: string;
		pre_context?: string[];
		post_context?: string[];
		in_app: boolean;
	}

	interface Breadcrumb {
		timestamp: string;
		category: string;
		message?: string;
		level: BreadcrumbLevel;
		data?: Record<string, unknown>;
	}

	interface UserContextData {
		id?: string;
		email?: string;
		username?: string;
		ip_address?: string;
	}

	interface DeviceContext {
		name?: string;
		family?: string;
		model?: string;
		arch?: string;
	}

	interface BrowserContext {
		name?: string;
		version?: string;
	}

	interface OsContext {
		name?: string;
		version?: string;
	}

	interface CrashEvent {
		id: string;
		exception_type: string;
		exception_value: string;
		stacktrace: { frames: Frame[] };
		raw_stacktrace?: { frames: Frame[] };
		release?: string;
		dist?: string;
		environment: string;
		platform: string;
		server_name?: string;
		tags: Record<string, string>;
		extra: Record<string, unknown>;
		user_context?: UserContextData;
		device_context?: DeviceContext;
		browser_context?: BrowserContext;
		os_context?: OsContext;
		active_flags: Record<string, string>;
		breadcrumbs: Breadcrumb[];
		timestamp: string;
		received_at: string;
	}

	interface Props {
		event: CrashEvent;
	}

	let { event }: Props = $props();

	let showRawStacktrace = $state(false);

	const tagEntries = $derived(Object.entries(event.tags));
	const extraEntries = $derived(Object.entries(event.extra));
</script>

<div class="crash-event-detail">
	<header class="event-header">
		<div class="event-id-row">
			<span class="event-id">{event.id}</span>
			<CopyButton value={event.id} size="sm" />
		</div>
		<div class="event-timing">
			<span class="timing-label">Occurred</span>
			<RelativeTime timestamp={event.timestamp} />
		</div>
	</header>

	<section class="event-section">
		<h2 class="section-title">Exception</h2>
		<div class="exception-info">
			<span class="exception-type">{event.exception_type}</span>
			<p class="exception-value">{event.exception_value}</p>
		</div>
	</section>

	<section class="event-section">
		<div class="stacktrace-toggle">
			<button
				class="toggle-btn"
				class:active={!showRawStacktrace}
				onclick={() => (showRawStacktrace = false)}
			>
				Symbolicated
			</button>
			{#if event.raw_stacktrace}
				<button
					class="toggle-btn"
					class:active={showRawStacktrace}
					onclick={() => (showRawStacktrace = true)}
				>
					Raw
				</button>
			{/if}
		</div>
		{#if showRawStacktrace && event.raw_stacktrace}
			<Stacktrace stacktrace={event.raw_stacktrace} title="Raw Stacktrace" raw />
		{:else}
			<Stacktrace stacktrace={event.stacktrace} />
		{/if}
	</section>

	{#if event.breadcrumbs.length > 0}
		<section class="event-section">
			<Breadcrumbs breadcrumbs={event.breadcrumbs} />
		</section>
	{/if}

	<div class="context-grid">
		{#if event.user_context}
			<UserContext user={event.user_context} />
		{/if}

		{#if Object.keys(event.active_flags).length > 0}
			<ActiveFlags flags={event.active_flags} />
		{/if}

		{#if event.device_context}
			<div class="context-card">
				<h3 class="context-title">Device</h3>
				<dl class="context-list">
					{#if event.device_context.name}
						<div class="context-item">
							<dt>Name</dt>
							<dd>{event.device_context.name}</dd>
						</div>
					{/if}
					{#if event.device_context.family}
						<div class="context-item">
							<dt>Family</dt>
							<dd>{event.device_context.family}</dd>
						</div>
					{/if}
					{#if event.device_context.model}
						<div class="context-item">
							<dt>Model</dt>
							<dd>{event.device_context.model}</dd>
						</div>
					{/if}
					{#if event.device_context.arch}
						<div class="context-item">
							<dt>Arch</dt>
							<dd>{event.device_context.arch}</dd>
						</div>
					{/if}
				</dl>
			</div>
		{/if}

		{#if event.browser_context}
			<div class="context-card">
				<h3 class="context-title">Browser</h3>
				<dl class="context-list">
					{#if event.browser_context.name}
						<div class="context-item">
							<dt>Name</dt>
							<dd>{event.browser_context.name}</dd>
						</div>
					{/if}
					{#if event.browser_context.version}
						<div class="context-item">
							<dt>Version</dt>
							<dd>{event.browser_context.version}</dd>
						</div>
					{/if}
				</dl>
			</div>
		{/if}

		{#if event.os_context}
			<div class="context-card">
				<h3 class="context-title">OS</h3>
				<dl class="context-list">
					{#if event.os_context.name}
						<div class="context-item">
							<dt>Name</dt>
							<dd>{event.os_context.name}</dd>
						</div>
					{/if}
					{#if event.os_context.version}
						<div class="context-item">
							<dt>Version</dt>
							<dd>{event.os_context.version}</dd>
						</div>
					{/if}
				</dl>
			</div>
		{/if}
	</div>

	<section class="event-section">
		<h2 class="section-title">Environment</h2>
		<dl class="env-list">
			<div class="env-item">
				<dt>Platform</dt>
				<dd>{event.platform}</dd>
			</div>
			<div class="env-item">
				<dt>Environment</dt>
				<dd>{event.environment}</dd>
			</div>
			{#if event.release}
				<div class="env-item">
					<dt>Release</dt>
					<dd>{event.release}</dd>
				</div>
			{/if}
			{#if event.dist}
				<div class="env-item">
					<dt>Dist</dt>
					<dd>{event.dist}</dd>
				</div>
			{/if}
			{#if event.server_name}
				<div class="env-item">
					<dt>Server</dt>
					<dd>{event.server_name}</dd>
				</div>
			{/if}
		</dl>
	</section>

	{#if tagEntries.length > 0}
		<section class="event-section">
			<h2 class="section-title">Tags</h2>
			<div class="tags-grid">
				{#each tagEntries as [key, value]}
					<div class="tag-item">
						<span class="tag-key">{key}</span>
						<span class="tag-value">{value}</span>
					</div>
				{/each}
			</div>
		</section>
	{/if}

	{#if extraEntries.length > 0}
		<section class="event-section">
			<h2 class="section-title">Extra Data</h2>
			<pre class="extra-data">{JSON.stringify(event.extra, null, 2)}</pre>
		</section>
	{/if}
</div>

<style>
	.crash-event-detail {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}

	.event-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-3);
	}

	.event-id-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.event-id {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.event-timing {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.timing-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.event-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.section-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
	}

	.exception-info {
		padding: var(--space-4);
		background: var(--color-error-soft);
		border-radius: var(--radius-md);
	}

	.exception-type {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-error);
	}

	.exception-value {
		margin: var(--space-2) 0 0 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
		white-space: pre-wrap;
		word-break: break-word;
	}

	.stacktrace-toggle {
		display: flex;
		gap: 1px;
		background: var(--color-border-muted);
		border-radius: var(--radius-sm);
		padding: 1px;
		width: fit-content;
	}

	.toggle-btn {
		padding: var(--space-1) var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: var(--color-bg-muted);
		color: var(--color-fg-muted);
		border: none;
		cursor: pointer;
	}

	.toggle-btn:first-child {
		border-radius: var(--radius-sm) 0 0 var(--radius-sm);
	}

	.toggle-btn:last-child {
		border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
	}

	.toggle-btn.active {
		background: var(--color-accent-soft);
		color: var(--color-accent);
	}

	.context-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: var(--space-4);
	}

	.context-card {
		background: var(--color-bg-muted);
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.context-title {
		margin: 0;
		padding: var(--space-3) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
		background: var(--color-bg-subtle);
		border-bottom: 1px solid var(--color-border-muted);
	}

	.context-list {
		margin: 0;
		padding: var(--space-3) var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.context-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.context-item dt {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.context-item dd {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg);
	}

	.env-list {
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
	}

	.env-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.env-item dt {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.env-item dd {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg);
	}

	.tags-grid {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}

	.tag-item {
		display: flex;
		align-items: center;
		padding: var(--space-1) var(--space-2);
		background: var(--color-bg-muted);
		border-radius: var(--radius-sm);
	}

	.tag-key {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.tag-key::after {
		content: ':';
		margin-right: var(--space-1);
	}

	.tag-value {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg);
	}

	.extra-data {
		margin: 0;
		padding: var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg);
		background: var(--color-bg-muted);
		border-radius: var(--radius-md);
		overflow-x: auto;
	}
</style>
