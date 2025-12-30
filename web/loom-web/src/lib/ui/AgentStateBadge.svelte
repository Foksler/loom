<script lang="ts">
  import type { AgentStateKind } from '../api/types';
  import Badge from './Badge.svelte';

  interface Props {
    state: AgentStateKind;
    showIcon?: boolean;
  }

  let { state, showIcon = true }: Props = $props();

  const stateConfig: Record<AgentStateKind, { label: string; variant: 'default' | 'accent' | 'success' | 'warning' | 'error' | 'muted'; icon: string }> = {
    idle: { label: 'Idle', variant: 'muted', icon: '💤' },
    thinking: { label: 'Thinking', variant: 'accent', icon: '🔄' },
    streaming: { label: 'Streaming', variant: 'accent', icon: '📝' },
    tool_pending: { label: 'Tool Pending', variant: 'warning', icon: '🔧' },
    tool_executing: { label: 'Executing Tools', variant: 'warning', icon: '⚙️' },
    waiting_input: { label: 'Waiting', variant: 'muted', icon: '⏳' },
    error: { label: 'Error', variant: 'error', icon: '❌' },
  };

  const config = $derived(stateConfig[state] || stateConfig.idle);
</script>

<Badge variant={config.variant}>
  {#if showIcon}
    <span class="mr-1">{config.icon}</span>
  {/if}
  {config.label}
</Badge>
