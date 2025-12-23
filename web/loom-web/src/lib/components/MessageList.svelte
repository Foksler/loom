<script lang="ts">
  import type { MessageSnapshot } from '../api/types';
  import MessageBubble from './MessageBubble.svelte';

  interface Props {
    messages: MessageSnapshot[];
    streamingContent?: string;
    isStreaming?: boolean;
  }

  let { messages, streamingContent = '', isStreaming = false }: Props = $props();

  let containerRef: HTMLDivElement;

  $effect(() => {
    // Auto-scroll to bottom when new messages arrive
    if (containerRef) {
      containerRef.scrollTop = containerRef.scrollHeight;
    }
  });
</script>

<div bind:this={containerRef} class="flex-1 overflow-y-auto p-4 space-y-4">
  {#if messages.length === 0 && !isStreaming}
    <div class="flex items-center justify-center h-full text-fg-muted">
      Start a conversation...
    </div>
  {:else}
    {#each messages as message (message.id || message.created_at)}
      <MessageBubble {message} />
    {/each}
    
    {#if isStreaming && streamingContent}
      <MessageBubble
        message={{ id: null, role: 'assistant', content: '', tool_name: null, tool_call_id: null, tool_input: null, tool_output: null, created_at: null }}
        isStreaming={true}
        {streamingContent}
      />
    {/if}
  {/if}
</div>
