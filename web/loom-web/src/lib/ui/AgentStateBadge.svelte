<script lang="ts">
  import type { AgentStateKind } from '../api/types';
  import Badge from './Badge.svelte';

  interface Props {
    state: AgentStateKind;
    showIcon?: boolean;
  }

  let { state, showIcon = true }: Props = $props();

  const stateConfig: Record<AgentStateKind, { label: string; variant: 'default' | 'accent' | 'success' | 'warning' | 'error' | 'muted'; icon: string }> = {
    waiting_for_user_input: { label: 'Waiting', variant: 'muted', icon: '⏳' },
    calling_llm: { label: 'Calling LLM', variant: 'accent', icon: '🔄' },
    processing_llm_response: { label: 'Processing', variant: 'accent', icon: '📝' },
    executing_tools: { label: 'Executing Tools', variant: 'warning', icon: '⚙️' },
    post_tools_hook: { label: 'Post-Hook', variant: 'warning', icon: '🔧' },
    error: { label: 'Error', variant: 'error', icon: '❌' },
    shutting_down: { label: 'Shutting Down', variant: 'muted', icon: '🛑' },
  };

  const config = $derived(stateConfig[state] || stateConfig.waiting_for_user_input);
</script>

<Badge variant={config.variant}>
  {#if showIcon}
    <span class="mr-1">{config.icon}</span>
  {/if}
  {config.label}
</Badge>
