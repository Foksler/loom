/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import type { Meta, StoryObj } from '@storybook/svelte';
import AgentStateBadge from './AgentStateBadge.svelte';
import type { AgentStateKind } from '../api/types';

interface AgentStateBadgeProps {
	state: AgentStateKind;
	showIcon?: boolean;
}

const meta: Meta<AgentStateBadgeProps> = {
	title: 'UI/AgentStateBadge',
	component: AgentStateBadge as any,
	tags: ['autodocs'],
	argTypes: {
		state: {
			control: 'select',
			options: [
				'waiting_input',
				'thinking',
				'streaming',
				'tool_executing',
				'tool_pending',
				'error',
				'idle',
			],
		},
		showIcon: { control: 'boolean' },
	},
};

export default meta;
type Story = StoryObj<AgentStateBadgeProps>;

export const WaitingForInput: Story = {
	args: {
		state: 'waiting_input',
		showIcon: true,
	},
};

export const CallingLlm: Story = {
	args: {
		state: 'thinking',
		showIcon: true,
	},
};

export const ProcessingResponse: Story = {
	args: {
		state: 'streaming',
		showIcon: true,
	},
};

export const ExecutingTools: Story = {
	args: {
		state: 'tool_executing',
		showIcon: true,
	},
};

export const PostToolsHook: Story = {
	args: {
		state: 'tool_pending',
		showIcon: true,
	},
};

export const Error: Story = {
	args: {
		state: 'error',
		showIcon: true,
	},
};

export const ShuttingDown: Story = {
	args: {
		state: 'idle',
		showIcon: true,
	},
};

export const NoIcon: Story = {
	args: {
		state: 'thinking',
		showIcon: false,
	},
};
