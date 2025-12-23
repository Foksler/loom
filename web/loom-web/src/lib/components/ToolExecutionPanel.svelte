<script lang="ts">
  import type { ToolExecutionStatus } from '../api/types';
  import { ToolStatusBadge, Card } from '../ui';
  import ToolExecutionRow from './ToolExecutionRow.svelte';

  interface Props {
    executions: ToolExecutionStatus[];
    expanded?: boolean;
  }

  let { executions, expanded = false }: Props = $props();

  let isExpanded = $state(expanded);
</script>

{#if executions.length > 0}
  <Card padding="none">
    <button
      type="button"
      class="w-full p-3 flex items-center justify-between hover:bg-bg-muted transition-colors"
      onclick={() => isExpanded = !isExpanded}
    >
      <span class="font-medium text-fg">
        Tools ({executions.length})
      </span>
      <span class="text-fg-muted transform transition-transform {isExpanded ? 'rotate-180' : ''}">
        ▼
      </span>
    </button>
    
    {#if isExpanded}
      <div class="border-t border-border divide-y divide-border">
        {#each executions as execution (execution.call_id)}
          <ToolExecutionRow {execution} />
        {/each}
      </div>
    {/if}
  </Card>
{/if}
