<script lang="ts">
  import type { MessageSnapshot } from '../api/types';
  import { Card } from '../ui';

  interface Props {
    message: MessageSnapshot;
    isStreaming?: boolean;
    streamingContent?: string;
  }

  let { message, isStreaming = false, streamingContent = '' }: Props = $props();

  const content = $derived(isStreaming ? streamingContent : message.content);

  const roleStyles = {
    user: 'ml-auto bg-accent text-white max-w-[80%]',
    assistant: 'mr-auto bg-bg-muted max-w-[80%]',
    tool: 'mr-auto bg-warning-soft border border-warning/20 max-w-[90%]',
    system: 'mx-auto bg-bg-subtle text-fg-muted text-center max-w-[90%]',
  };
</script>

<div class="flex {message.role === 'user' ? 'justify-end' : 'justify-start'}">
  <div class="rounded-lg p-3 {roleStyles[message.role]}">
    {#if message.role === 'tool'}
      <div class="text-xs font-medium text-warning mb-1">
        🔧 {message.tool_name}
      </div>
    {/if}
    
    <div class="whitespace-pre-wrap break-words">
      {content}
      {#if isStreaming}
        <span class="inline-block w-2 h-4 bg-accent animate-pulse ml-0.5"></span>
      {/if}
    </div>
    
    {#if message.role === 'tool' && message.tool_output}
      <details class="mt-2">
        <summary class="text-xs text-fg-muted cursor-pointer hover:text-fg">
          Show output
        </summary>
        <pre class="mt-1 p-2 bg-bg rounded text-xs overflow-x-auto">{JSON.stringify(message.tool_output, null, 2)}</pre>
      </details>
    {/if}
    
    {#if message.created_at}
      <div class="text-xs text-fg-subtle mt-1 {message.role === 'user' ? 'text-right' : ''}">
        {new Date(message.created_at).toLocaleTimeString()}
      </div>
    {/if}
  </div>
</div>
