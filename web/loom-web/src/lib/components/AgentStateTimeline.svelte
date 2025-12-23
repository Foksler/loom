<script lang="ts">
  import type { AgentStateKind } from '../api/types';

  interface Props {
    currentState: AgentStateKind;
    retries?: number;
    pendingToolCalls?: string[];
  }

  let { currentState, retries = 0, pendingToolCalls = [] }: Props = $props();

  const states: { key: AgentStateKind; label: string; icon: string }[] = [
    { key: 'waiting_for_user_input', label: 'Waiting', icon: '⏳' },
    { key: 'calling_llm', label: 'LLM', icon: '🤖' },
    { key: 'processing_llm_response', label: 'Processing', icon: '📝' },
    { key: 'executing_tools', label: 'Tools', icon: '⚙️' },
  ];

  function isActive(stateKey: AgentStateKind): boolean {
    return stateKey === currentState;
  }

  function isPast(stateKey: AgentStateKind): boolean {
    const stateOrder: AgentStateKind[] = [
      'waiting_for_user_input',
      'calling_llm',
      'processing_llm_response',
      'executing_tools',
    ];
    const currentIndex = stateOrder.indexOf(currentState);
    const stateIndex = stateOrder.indexOf(stateKey);
    return stateIndex < currentIndex && currentIndex !== -1;
  }
</script>

<div class="flex items-center justify-between p-3 bg-bg-muted rounded-lg">
  {#each states as state, i}
    <div class="flex items-center">
      <div
        class="flex items-center justify-center w-8 h-8 rounded-full text-sm
               {isActive(state.key) 
                 ? 'bg-accent text-white' 
                 : isPast(state.key)
                   ? 'bg-success-soft text-success'
                   : 'bg-bg-subtle text-fg-muted'}"
      >
        {state.icon}
      </div>
      <span
        class="ml-2 text-sm font-medium
               {isActive(state.key) ? 'text-accent' : 'text-fg-muted'}"
      >
        {state.label}
        {#if isActive(state.key) && state.key === 'executing_tools' && pendingToolCalls.length > 0}
          <span class="text-xs">({pendingToolCalls.length})</span>
        {/if}
      </span>
    </div>
    
    {#if i < states.length - 1}
      <div class="flex-1 h-0.5 mx-2 {isPast(states[i + 1].key) || isActive(states[i + 1].key) ? 'bg-accent' : 'bg-border'}"></div>
    {/if}
  {/each}
  
  {#if currentState === 'error'}
    <div class="ml-4 flex items-center text-error">
      <span class="mr-1">❌</span>
      <span class="text-sm">Error (retry {retries})</span>
    </div>
  {/if}
</div>
