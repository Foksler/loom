<script lang="ts">
  import type { ToolExecutionStatus } from '../api/types';
  import { ToolStatusBadge } from '../ui';

  interface Props {
    execution: ToolExecutionStatus;
  }

  let { execution }: Props = $props();
</script>

<div class="p-3">
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-2">
      <span class="font-mono text-sm text-fg">{execution.tool_name}</span>
      <ToolStatusBadge status={execution} />
    </div>
    <span class="text-xs text-fg-subtle font-mono">
      {execution.call_id.slice(0, 8)}...
    </span>
  </div>
  
  {#if execution.type === 'running' && execution.progress}
    <div class="mt-2">
      {#if execution.progress.fraction !== null}
        <div class="h-1.5 bg-bg-subtle rounded-full overflow-hidden">
          <div
            class="h-full bg-accent transition-all"
            style="width: {execution.progress.fraction * 100}%"
          ></div>
        </div>
      {/if}
      {#if execution.progress.message}
        <p class="text-xs text-fg-muted mt-1">{execution.progress.message}</p>
      {/if}
    </div>
  {/if}
  
  {#if execution.type === 'completed'}
    <details class="mt-2">
      <summary class="text-xs text-fg-muted cursor-pointer hover:text-fg">
        {execution.outcome.type === 'success' ? 'Show output' : 'Show error'}
      </summary>
      <pre class="mt-1 p-2 bg-bg-subtle rounded text-xs overflow-x-auto max-h-32">
        {execution.outcome.type === 'success' 
          ? JSON.stringify(execution.outcome.output, null, 2)
          : execution.outcome.error}
      </pre>
    </details>
  {/if}
</div>
