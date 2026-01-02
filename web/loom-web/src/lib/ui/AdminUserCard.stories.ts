/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import type { Meta, StoryObj } from '@storybook/svelte';
import AdminUserCard from './AdminUserCard.svelte';
import type { AdminUser } from '$lib/api/types';

interface AdminUserCardProps {
	user: AdminUser;
	currentUserId: string;
	onToggleSystemAdmin?: (userId: string, currentValue: boolean) => void;
	onImpersonate?: (userId: string) => void;
	isUpdating?: boolean;
	isImpersonating?: boolean;
}

const meta: Meta<AdminUserCardProps> = {
	title: 'UI/AdminUserCard',
	component: AdminUserCard as any,
	tags: ['autodocs'],
	argTypes: {
		onToggleSystemAdmin: { action: 'toggleSystemAdmin' },
		onImpersonate: { action: 'impersonate' },
	},
};

export default meta;
type Story = StoryObj<AdminUserCardProps>;

const baseUser: AdminUser = {
	id: 'user-123',
	display_name: 'John Doe',
	email: 'john@example.com',
	avatar_url: null,
	global_roles: [],
	created_at: '2024-01-15T10:30:00Z',
	last_login_at: '2025-01-02T14:22:00Z',
};

export const RegularUser: Story = {
	args: {
		user: { ...baseUser },
		currentUserId: 'other-user',
	},
};

export const SystemAdmin: Story = {
	args: {
		user: { ...baseUser, global_roles: ['system_admin'] },
		currentUserId: 'other-user',
	},
};

export const AllRoles: Story = {
	args: {
		user: { ...baseUser, global_roles: ['system_admin', 'support', 'auditor'] },
		currentUserId: 'other-user',
	},
};

export const CurrentUser: Story = {
	args: {
		user: { ...baseUser, global_roles: ['system_admin'] },
		currentUserId: 'user-123',
	},
};

export const WithAvatar: Story = {
	args: {
		user: {
			...baseUser,
			avatar_url: 'https://avatars.githubusercontent.com/u/1?v=4',
			global_roles: ['system_admin'],
		},
		currentUserId: 'other-user',
	},
};

export const Updating: Story = {
	args: {
		user: { ...baseUser, global_roles: ['system_admin'] },
		currentUserId: 'other-user',
		isUpdating: true,
	},
};
