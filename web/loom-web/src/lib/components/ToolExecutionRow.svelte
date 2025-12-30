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
  
  {#if execution.status === 'completed' || execution.status === 'failed'}
    <details class="mt-2">
      <summary class="text-xs text-fg-muted cursor-pointer hover:text-fg">
        {execution.error ? 'Show error' : 'Show output'}
      </summary>
      <pre class="mt-1 p-2 bg-bg-subtle rounded text-xs overflow-x-auto max-h-32">
        {execution.error ?? JSON.stringify(execution.result, null, 2)}
      </pre>
    </details>
  {/if}
</div>
