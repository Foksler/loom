<script lang="ts">
  import type { ConnectionStatus } from '../realtime/types';
  import { Badge } from '../ui';

  interface Props {
    status: ConnectionStatus;
  }

  let { status }: Props = $props();

  const statusConfig: Record<ConnectionStatus, { label: string; variant: 'success' | 'warning' | 'error' | 'muted'; icon: string }> = {
    connected: { label: 'Connected', variant: 'success', icon: '●' },
    connecting: { label: 'Connecting...', variant: 'muted', icon: '○' },
    disconnected: { label: 'Disconnected', variant: 'muted', icon: '○' },
    reconnecting: { label: 'Reconnecting...', variant: 'warning', icon: '◐' },
    error: { label: 'Connection Error', variant: 'error', icon: '●' },
  };

  const config = $derived(statusConfig[status]);
</script>

<Badge variant={config.variant} size="sm">
  <span class="mr-1 {status === 'connecting' || status === 'reconnecting' ? 'animate-pulse' : ''}">
    {config.icon}
  </span>
  {config.label}
</Badge>
