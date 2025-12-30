<script lang="ts">
  import { onMount } from 'svelte';
  import { createActor, type SnapshotFrom } from 'xstate';
  import { threadListMachine } from '../state';
  import { getApiClient } from '../api';
  import { Input, Skeleton, Button } from '../ui';
  import ThreadListItem from './ThreadListItem.svelte';
  import { logger } from '../logging';

  interface Props {
    selectedThreadId?: string | null;
    onSelectThread?: (threadId: string) => void;
  }

  let { selectedThreadId = null, onSelectThread }: Props = $props();

  const actor = createActor(threadListMachine);
  let snapshot: SnapshotFrom<typeof threadListMachine> = $state(actor.getSnapshot());
  let searchInput = $state('');
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  actor.subscribe((s) => {
    snapshot = s;
  });

  async function fetchThreads() {
    const api = getApiClient();
    const ctx = snapshot.context;

    try {
      let response;
      if (ctx.searchQuery) {
        response = await api.searchThreads(ctx.searchQuery, {
          limit: ctx.limit,
          offset: ctx.offset,
        });
        actor.send({
          type: 'FETCH_SUCCESS',
          threads: response.hits.map(hit => ({
            id: hit.id,
            title: hit.title,
            created_at: hit.created_at,
            updated_at: hit.updated_at,
            message_count: 0,
          })),
          total: response.hits.length,
        });
      } else {
        response = await api.listThreads({
          limit: ctx.limit,
          offset: ctx.offset,
        });
        actor.send({
          type: 'FETCH_SUCCESS',
          threads: response.threads,
          total: response.total,
        });
      }
    } catch (error) {
      logger.error('Failed to fetch threads', { error: String(error) });
      actor.send({ type: 'FETCH_ERROR', error: String(error) });
    }
  }

  function handleSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      if (searchInput.trim()) {
        actor.send({ type: 'SEARCH', query: searchInput.trim() });
      } else {
        actor.send({ type: 'CLEAR_SEARCH' });
      }
    }, 300);
  }

  function handleSelectThread(threadId: string) {
    onSelectThread?.(threadId);
  }

  onMount(() => {
    actor.start();
    actor.send({ type: 'FETCH' });

    actor.subscribe((s) => {
      if (s.value === 'loading') {
        fetchThreads();
      }
    });

    return () => actor.stop();
  });

  $effect(() => {
    handleSearch();
  });
</script>

<div class="flex flex-col h-full">
  <div class="p-3 border-b border-border">
    <Input
      type="search"
      placeholder="Search threads..."
      bind:value={searchInput}
    />
  </div>

  <div class="flex-1 overflow-y-auto p-2">
    {#if snapshot.value === 'loading' && snapshot.context.threads.length === 0}
      <div class="space-y-2">
        {#each Array(5) as _}
          <div class="p-3">
            <Skeleton width="70%" height="1rem" />
            <Skeleton width="50%" height="0.75rem" rounded="sm" />
          </div>
        {/each}
      </div>
    {:else if snapshot.value === 'error'}
      <div class="p-4 text-center">
        <p class="text-error mb-2">{snapshot.context.error}</p>
        <Button variant="secondary" onclick={() => actor.send({ type: 'FETCH' })}>
          Retry
        </Button>
      </div>
    {:else if snapshot.context.threads.length === 0}
      <div class="p-4 text-center text-fg-muted">
        {snapshot.context.searchQuery ? 'No threads found' : 'No threads yet'}
      </div>
    {:else}
      <div class="space-y-1">
        {#each snapshot.context.threads as thread (thread.id)}
          <ThreadListItem
            {thread}
            isActive={thread.id === selectedThreadId}
            onclick={() => handleSelectThread(thread.id)}
          />
        {/each}
      </div>
    {/if}
  </div>

  {#if snapshot.context.total > snapshot.context.limit}
    <div class="p-3 border-t border-border flex justify-between items-center">
      <Button
        variant="ghost"
        size="sm"
        disabled={snapshot.context.offset === 0}
        onclick={() => actor.send({ type: 'PREV_PAGE' })}
      >
        Previous
      </Button>
      <span class="text-sm text-fg-muted">
        {snapshot.context.offset + 1}-{Math.min(snapshot.context.offset + snapshot.context.limit, snapshot.context.total)} of {snapshot.context.total}
      </span>
      <Button
        variant="ghost"
        size="sm"
        disabled={snapshot.context.offset + snapshot.context.limit >= snapshot.context.total}
        onclick={() => actor.send({ type: 'NEXT_PAGE' })}
      >
        Next
      </Button>
    </div>
  {/if}
</div>
