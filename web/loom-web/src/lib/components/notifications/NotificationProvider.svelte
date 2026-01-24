<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts" module>
	import { writable } from 'svelte/store';

	export interface Notification {
		id: string;
		type: 'info' | 'success' | 'warning' | 'error';
		title: string;
		message: string;
		link?: string;
		duration?: number;
	}

	export const notifications = writable<Notification[]>([]);

	export function showNotification(notification: Omit<Notification, 'id'>) {
		const id = crypto.randomUUID();
		const duration = notification.duration ?? 5000;

		notifications.update((n) => [...n, { ...notification, id }]);

		if (duration > 0) {
			setTimeout(() => {
				dismissNotification(id);
			}, duration);
		}
	}

	export function dismissNotification(id: string) {
		notifications.update((n) => n.filter((notification) => notification.id !== id));
	}
</script>

<script lang="ts">
	import { fly, fade } from 'svelte/transition';

	const typeIcons: Record<Notification['type'], string> = {
		info: 'i',
		success: '✓',
		warning: '!',
		error: '✕',
	};
</script>

<div class="notification-container" role="region" aria-label="Notifications">
	{#each $notifications as notification (notification.id)}
		<div
			class="notification notification-{notification.type}"
			in:fly={{ x: 300, duration: 200 }}
			out:fade={{ duration: 150 }}
			role="alert"
		>
			<span class="notification-icon">{typeIcons[notification.type]}</span>
			<div class="notification-content">
				<strong class="notification-title">{notification.title}</strong>
				<p class="notification-message">{notification.message}</p>
				{#if notification.link}
					<a href={notification.link} class="notification-link">View details</a>
				{/if}
			</div>
			<button
				type="button"
				class="notification-dismiss"
				onclick={() => dismissNotification(notification.id)}
				aria-label="Dismiss notification"
			>
				&times;
			</button>
		</div>
	{/each}
</div>

<style>
	.notification-container {
		position: fixed;
		top: var(--space-4);
		right: var(--space-4);
		z-index: 1000;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		max-width: 400px;
		pointer-events: none;
	}

	.notification {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
		pointer-events: auto;
		font-family: var(--font-mono);
	}

	.notification-info {
		border-left: 3px solid var(--color-accent);
	}

	.notification-success {
		border-left: 3px solid var(--color-success);
	}

	.notification-warning {
		border-left: 3px solid var(--color-warning);
	}

	.notification-error {
		border-left: 3px solid var(--color-error);
	}

	.notification-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		font-size: var(--text-sm);
		font-weight: 700;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.notification-info .notification-icon {
		background: var(--color-accent);
		color: var(--color-bg);
	}

	.notification-success .notification-icon {
		background: var(--color-success);
		color: var(--color-bg);
	}

	.notification-warning .notification-icon {
		background: var(--color-warning);
		color: var(--color-bg);
	}

	.notification-error .notification-icon {
		background: var(--color-error);
		color: var(--color-bg);
	}

	.notification-content {
		flex: 1;
		min-width: 0;
	}

	.notification-title {
		display: block;
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-fg);
		margin-bottom: var(--space-1);
	}

	.notification-message {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--color-fg-muted);
		line-height: 1.4;
	}

	.notification-link {
		display: inline-block;
		margin-top: var(--space-2);
		font-size: var(--text-xs);
		color: var(--color-accent);
		text-decoration: none;
	}

	.notification-link:hover {
		text-decoration: underline;
	}

	.notification-dismiss {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 20px;
		height: 20px;
		padding: 0;
		font-size: var(--text-lg);
		color: var(--color-fg-muted);
		background: transparent;
		border: none;
		cursor: pointer;
		flex-shrink: 0;
	}

	.notification-dismiss:hover {
		color: var(--color-fg);
	}
</style>
