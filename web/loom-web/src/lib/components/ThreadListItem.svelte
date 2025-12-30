<script lang="ts">
  import type { ThreadSummary } from '../api/types';

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
      {#if thread.last_message_preview}
        <p class="text-sm text-fg-muted truncate mt-0.5">
          {thread.last_message_preview}
        </p>
      {/if}
    </div>
    <span class="text-xs text-fg-subtle whitespace-nowrap">
      {formatRelativeTime(thread.updated_at)}
    </span>
  </div>
  
  <div class="flex items-center gap-2 mt-2">
    <span class="text-xs text-fg-subtle">{thread.message_count} messages</span>
  </div>
</button>
