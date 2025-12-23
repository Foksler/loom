/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import type { Meta, StoryObj } from '@storybook/svelte';
import Badge from './Badge.svelte';

const meta = {
	title: 'UI/Badge',
	component: Badge,
	tags: ['autodocs'],
	argTypes: {
		variant: {
			control: 'select',
			options: ['default', 'accent', 'success', 'warning', 'error', 'muted'],
		},
		size: {
			control: 'select',
			options: ['sm', 'md'],
		},
	},
} satisfies Meta<Badge>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
	args: {
		variant: 'default',
		children: 'Default',
	},
};

export const Accent: Story = {
	args: {
		variant: 'accent',
		children: 'Accent',
	},
};

export const Success: Story = {
	args: {
		variant: 'success',
		children: 'Success',
	},
};

export const Warning: Story = {
	args: {
		variant: 'warning',
		children: 'Warning',
	},
};

export const Error: Story = {
	args: {
		variant: 'error',
		children: 'Error',
	},
};

export const Muted: Story = {
	args: {
		variant: 'muted',
		children: 'Muted',
	},
};

export const Small: Story = {
	args: {
		size: 'sm',
		children: 'Small Badge',
	},
};
