/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import type { Meta, StoryObj } from '@storybook/svelte';
import AgentStateBadge from './AgentStateBadge.svelte';
import type { AgentStateKind } from '../api/types';

const meta = {
	title: 'UI/AgentStateBadge',
	component: AgentStateBadge,
	tags: ['autodocs'],
	argTypes: {
		state: {
			control: 'select',
			options: [
				'waiting_for_user_input',
				'calling_llm',
				'processing_llm_response',
				'executing_tools',
				'post_tools_hook',
				'error',
				'shutting_down',
			] as AgentStateKind[],
		},
		showIcon: { control: 'boolean' },
	},
} satisfies Meta<AgentStateBadge>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WaitingForInput: Story = {
	args: {
		state: 'waiting_for_user_input',
		showIcon: true,
	},
};

export const CallingLlm: Story = {
	args: {
		state: 'calling_llm',
		showIcon: true,
	},
};

export const ProcessingResponse: Story = {
	args: {
		state: 'processing_llm_response',
		showIcon: true,
	},
};

export const ExecutingTools: Story = {
	args: {
		state: 'executing_tools',
		showIcon: true,
	},
};

export const PostToolsHook: Story = {
	args: {
		state: 'post_tools_hook',
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
		state: 'shutting_down',
		showIcon: true,
	},
};

export const NoIcon: Story = {
	args: {
		state: 'calling_llm',
		showIcon: false,
	},
};
