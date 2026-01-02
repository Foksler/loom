<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { i18n } from '$lib/i18n';
	import type { AdminUser } from '$lib/api/types';
	import Badge from './Badge.svelte';
	import Button from './Button.svelte';
	import Card from './Card.svelte';

	interface Props {
		user: AdminUser;
		currentUserId: string;
		onToggleSystemAdmin?: (userId: string, currentValue: boolean) => void;
		onImpersonate?: (userId: string) => void;
		isUpdating?: boolean;
		isImpersonating?: boolean;
	}

	let {
		user,
		currentUserId,
		onToggleSystemAdmin,
		onImpersonate,
		isUpdating = false,
		isImpersonating = false,
	}: Props = $props();

	const isCurrentUser = $derived(user.id === currentUserId);
	const isSystemAdmin = $derived(user.global_roles.includes('system_admin'));
	const isSupport = $derived(user.global_roles.includes('support'));
	const isAuditor = $derived(user.global_roles.includes('auditor'));

	function formatDate(dateStr: string | null): string {
		if (!dateStr) return '-';
		return new Date(dateStr).toLocaleDateString();
	}

	function getRoleBadges(): Array<{ role: string; variant: 'accent' | 'warning' | 'success' | 'muted' }> {
		const badges: Array<{ role: string; variant: 'accent' | 'warning' | 'success' | 'muted' }> = [];
		if (isSystemAdmin) badges.push({ role: 'system_admin', variant: 'accent' });
		if (isSupport) badges.push({ role: 'support', variant: 'warning' });
		if (isAuditor) badges.push({ role: 'auditor', variant: 'success' });
		return badges;
	}
</script>

<Card>
	<div class="flex items-center justify-between gap-4">
		<div class="flex items-center gap-4 min-w-0">
			{#if user.avatar_url}
				<img src={user.avatar_url} alt="" class="w-10 h-10 rounded-full" />
			{:else}
				<div class="w-10 h-10 rounded-full bg-bg-muted flex items-center justify-center">
					<span class="text-sm font-medium text-fg-muted">
						{user.display_name?.charAt(0).toUpperCase() ?? '?'}
					</span>
				</div>
			{/if}
			<div class="min-w-0">
				<div class="font-medium text-fg truncate">
					{user.display_name}
					{#if isCurrentUser}
						<span class="text-fg-muted text-sm">({i18n._('admin.users.you')})</span>
					{/if}
				</div>
				<div class="text-sm text-fg-muted truncate">{user.email ?? '-'}</div>
				<div class="flex flex-wrap gap-1 mt-1">
					{#each getRoleBadges() as { role, variant }}
						<Badge {variant} size="sm">{role}</Badge>
					{/each}
				</div>
			</div>
		</div>
		<div class="flex items-center gap-4 flex-shrink-0">
			<div class="text-sm text-fg-muted text-right">
				<div>{i18n._('admin.users.created')}: {formatDate(user.created_at)}</div>
				<div>{i18n._('admin.users.lastLogin')}: {formatDate(user.last_login_at)}</div>
			</div>
			<div class="flex gap-2">
				{#if onToggleSystemAdmin}
					<Button
						variant={isSystemAdmin ? 'danger' : 'secondary'}
						size="sm"
						disabled={isUpdating || isCurrentUser}
						loading={isUpdating}
						onclick={() => onToggleSystemAdmin?.(user.id, isSystemAdmin)}
					>
						{#if isSystemAdmin}
							{i18n._('admin.users.removeAdmin')}
						{:else}
							{i18n._('admin.users.makeAdmin')}
						{/if}
					</Button>
				{/if}
				{#if onImpersonate}
					<Button
						variant="secondary"
						size="sm"
						disabled={isImpersonating || isCurrentUser}
						loading={isImpersonating}
						onclick={() => onImpersonate?.(user.id)}
					>
						{i18n._('admin.users.impersonate')}
					</Button>
				{/if}
			</div>
		</div>
	</div>
</Card>
