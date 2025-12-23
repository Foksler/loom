<script lang="ts">
  import type { ToolExecutionStatus } from '../api/types';
  import Badge from './Badge.svelte';

  interface Props {
    status: ToolExecutionStatus;
    showIcon?: boolean;
  }

  let { status, showIcon = true }: Props = $props();

  const statusConfig = {
    pending: { label: 'Pending', variant: 'muted' as const, icon: '⏳' },
    running: { label: 'Running', variant: 'accent' as const, icon: '🔄' },
    completed: { label: 'Completed', variant: 'success' as const, icon: '✓' },
  };

  const config = $derived(() => {
    if (status.type === 'completed' && status.outcome.type === 'error') {
      return { label: 'Failed', variant: 'error' as const, icon: '✗' };
    }
    return statusConfig[status.type];
  });
</script>

<Badge variant={config().variant}>
  {#if showIcon}
    <span class="mr-1">{config().icon}</span>
  {/if}
  {config().label}
</Badge>
