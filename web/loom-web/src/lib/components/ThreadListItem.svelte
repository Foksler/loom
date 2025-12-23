<script lang="ts">
  import type { ThreadSummary } from '../api/types';
  import { AgentStateBadge, Badge } from '../ui';

  interface Props {
    thread: ThreadSummary;
    isActive?: boolean;
    onclick?: () => void;
  }

  let { thread, isActive = false, onclick }: Props = $props();

  function formatRelativeTime(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  }
</script>

<button
  type="button"
  class="w-full text-left p-3 rounded-lg border transition-colors
         {isActive 
           ? 'border-accent bg-accent-soft' 
           : 'border-transparent hover:bg-bg-muted'}"
  onclick={onclick}
>
  <div class="flex items-start justify-between gap-2">
    <div class="flex-1 min-w-0">
      <h3 class="font-medium text-fg truncate">
        {thread.title || `Thread ${thread.id.slice(0, 12)}...`}
      </h3>
      <p class="text-sm text-fg-muted truncate mt-0.5">
        {thread.workspace_root || 'No workspace'}
      </p>
    </div>
    <span class="text-xs text-fg-subtle whitespace-nowrap">
      {formatRelativeTime(thread.last_activity_at)}
    </span>
  </div>
  
  <div class="flex items-center gap-2 mt-2">
    {#if thread.provider}
      <Badge variant="muted" size="sm">{thread.provider}</Badge>
    {/if}
    <span class="text-xs text-fg-subtle">{thread.message_count} messages</span>
  </div>
  
  {#if thread.tags.length > 0}
    <div class="flex flex-wrap gap-1 mt-2">
      {#each thread.tags.slice(0, 3) as tag}
        <Badge variant="default" size="sm">{tag}</Badge>
      {/each}
      {#if thread.tags.length > 3}
        <Badge variant="muted" size="sm">+{thread.tags.length - 3}</Badge>
      {/if}
    </div>
  {/if}
</button>
