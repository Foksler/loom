import type { Meta, StoryObj } from '@storybook/svelte';
import Card from './Card.svelte';

const meta = {
  title: 'UI/Card',
  component: Card,
  tags: ['autodocs'],
  argTypes: {
    padding: {
      control: 'select',
      options: ['none', 'sm', 'md', 'lg'],
    },
    hover: { control: 'boolean' },
  },
} satisfies Meta<Card>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    padding: 'md',
    children: 'Card content goes here. This is a basic card with default padding.',
  },
};

export const WithHover: Story = {
  args: {
    padding: 'md',
    hover: true,
    children: 'Hover over this card to see the effect.',
  },
};

export const NoPadding: Story = {
  args: {
    padding: 'none',
    children: 'Card with no padding.',
  },
};

export const LargePadding: Story = {
  args: {
    padding: 'lg',
    children: 'Card with large padding for more spacious content.',
  },
};
