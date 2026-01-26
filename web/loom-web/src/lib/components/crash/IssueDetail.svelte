<script lang="ts">
	import IssueStatusBadge from './IssueStatusBadge.svelte';
	import Stacktrace from './Stacktrace.svelte';
	import Breadcrumbs from './Breadcrumbs.svelte';
	import ActiveFlags from './ActiveFlags.svelte';
	import UserContext from './UserContext.svelte';
	import { RelativeTime, StatCard } from '$lib/components/common';

	type IssueStatus = 'unresolved' | 'resolved' | 'ignored' | 'regressed';
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

	interface Issue {
		id: string;
		short_id: string;
		title: string;
		culprit?: string;
		status: IssueStatus;
		level: string;
		priority: string;
		event_count: number;
		user_count: number;
		first_seen: string;
		last_seen: string;
		resolved_at?: string;
		resolved_by?: string;
		resolved_in_release?: string;
		times_regressed: number;
		assigned_to?: string;
		metadata: {
			exception_type: string;
			exception_value: string;
			filename?: string;
			function?: string;
		};
		latest_event?: {
			id: string;
			stacktrace: { frames: Frame[] };
			breadcrumbs: Breadcrumb[];
			active_flags: Record<string, string>;
			user_context?: UserContextData;
			release?: string;
			environment: string;
			platform: string;
			timestamp: string;
		};
	}

	interface Props {
		issue: Issue;
		onresolve?: () => void;
		onunresolve?: () => void;
		onignore?: () => void;
	}

	let { issue, onresolve, onunresolve, onignore }: Props = $props();

	const canResolve = $derived(issue.status === 'unresolved' || issue.status === 'regressed');
	const canUnresolve = $derived(issue.status === 'resolved' || issue.status === 'ignored');
</script>

<div class="issue-detail">
	<header class="issue-header">
		<div class="issue-title-row">
			<span class="issue-id">{issue.short_id}</span>
			<IssueStatusBadge status={issue.status} />
		</div>
		<h1 class="issue-title">{issue.title}</h1>
		{#if issue.culprit}
			<span class="issue-culprit">{issue.culprit}</span>
		{/if}
	</header>

	<div class="issue-actions">
		{#if canResolve}
			<button class="action-btn action-resolve" onclick={onresolve}>
				Resolve
			</button>
		{/if}
		{#if canUnresolve}
			<button class="action-btn action-unresolve" onclick={onunresolve}>
				Unresolve
			</button>
		{/if}
		{#if issue.status !== 'ignored'}
			<button class="action-btn action-ignore" onclick={onignore}>
				Ignore
			</button>
		{/if}
	</div>

	<div class="issue-stats">
		<StatCard label="Events" value={issue.event_count.toLocaleString()} size="sm" />
		<StatCard label="Users Affected" value={issue.user_count.toLocaleString()} size="sm" />
		<StatCard label="Times Regressed" value={issue.times_regressed.toString()} size="sm" />
	</div>

	<div class="issue-timeline">
		<div class="timeline-item">
			<span class="timeline-label">First seen</span>
			<RelativeTime timestamp={issue.first_seen} />
		</div>
		<div class="timeline-item">
			<span class="timeline-label">Last seen</span>
			<RelativeTime timestamp={issue.last_seen} />
		</div>
		{#if issue.resolved_at}
			<div class="timeline-item">
				<span class="timeline-label">Resolved</span>
				<RelativeTime timestamp={issue.resolved_at} />
				{#if issue.resolved_in_release}
					<span class="timeline-release">in {issue.resolved_in_release}</span>
				{/if}
			</div>
		{/if}
	</div>

	{#if issue.latest_event}
		<div class="issue-sections">
			<section class="issue-section">
				<h2 class="section-title">Exception</h2>
				<div class="exception-info">
					<span class="exception-type">{issue.metadata.exception_type}</span>
					<p class="exception-value">{issue.metadata.exception_value}</p>
				</div>
			</section>

			{#if issue.latest_event.stacktrace}
				<section class="issue-section">
					<Stacktrace stacktrace={issue.latest_event.stacktrace} />
				</section>
			{/if}

			{#if issue.latest_event.breadcrumbs && issue.latest_event.breadcrumbs.length > 0}
				<section class="issue-section">
					<Breadcrumbs breadcrumbs={issue.latest_event.breadcrumbs} />
				</section>
			{/if}

			<div class="issue-context-grid">
				{#if issue.latest_event.user_context}
					<UserContext user={issue.latest_event.user_context} />
				{/if}

				{#if issue.latest_event.active_flags && Object.keys(issue.latest_event.active_flags).length > 0}
					<ActiveFlags flags={issue.latest_event.active_flags} />
				{/if}
			</div>

			<section class="issue-section">
				<h2 class="section-title">Environment</h2>
				<dl class="env-list">
					<div class="env-item">
						<dt>Platform</dt>
						<dd>{issue.latest_event.platform}</dd>
					</div>
					<div class="env-item">
						<dt>Environment</dt>
						<dd>{issue.latest_event.environment}</dd>
					</div>
					{#if issue.latest_event.release}
						<div class="env-item">
							<dt>Release</dt>
							<dd>{issue.latest_event.release}</dd>
						</div>
					{/if}
				</dl>
			</section>
		</div>
	{/if}
</div>

<style>
	.issue-detail {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}

	.issue-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.issue-title-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.issue-id {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.issue-title {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-xl);
		font-weight: 600;
		color: var(--color-fg);
	}

	.issue-culprit {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-fg-muted);
	}

	.issue-actions {
		display: flex;
		gap: var(--space-2);
	}

	.action-btn {
		padding: var(--space-2) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		border: 1px solid var(--color-border-muted);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: all 0.15s ease;
	}

	.action-resolve {
		background: var(--color-success-soft);
		color: var(--color-success);
		border-color: var(--color-success);
	}

	.action-resolve:hover {
		background: var(--color-success);
		color: var(--color-bg);
	}

	.action-unresolve {
		background: var(--color-bg-muted);
		color: var(--color-fg);
	}

	.action-unresolve:hover {
		background: var(--color-bg-subtle);
	}

	.action-ignore {
		background: var(--color-bg-muted);
		color: var(--color-fg-muted);
	}

	.action-ignore:hover {
		background: var(--color-bg-subtle);
		color: var(--color-fg);
	}

	.issue-stats {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: var(--space-3);
	}

	.issue-timeline {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-muted);
		border-radius: var(--radius-md);
	}

	.timeline-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.timeline-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
	}

	.timeline-release {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-accent);
	}

	.issue-sections {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.issue-section {
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

	.issue-context-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: var(--space-4);
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
</style>
