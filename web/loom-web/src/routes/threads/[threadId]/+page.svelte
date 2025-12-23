<script lang="ts">
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';
  import { createActor } from 'xstate';
  import { conversationMachine, connectionMachine } from '$lib/state';
  import { getApiClient } from '$lib/api';
  import { createRealtimeClient, type LlmEvent } from '$lib/realtime';
  import {
    MessageList,
    MessageInput,
    AgentStateTimeline,
    ToolExecutionPanel,
    ConnectionStatusIndicator,
  } from '$lib/components';
  import { AgentStateBadge, Skeleton, Card } from '$lib/ui';
  import { logger } from '$lib/logging';

  const threadId = $derived($page.params.threadId);

  const conversationActor = createActor(conversationMachine);
  const connectionActor = createActor(connectionMachine);
  
  let conversationState = $state(conversationActor.getSnapshot());
  let connectionState = $state(connectionActor.getSnapshot());
  
  let realtimeClient = createRealtimeClient({ serverUrl: '' });

  conversationActor.subscribe((snapshot) => {
    conversationState = snapshot;
  });

  connectionActor.subscribe((snapshot) => {
    connectionState = snapshot;
  });

  async function loadThread(id: string) {
    logger.info('Loading thread', { threadId: id });
    conversationActor.send({ type: 'LOAD_THREAD', threadId: id });
    
    try {
      const api = getApiClient();
      const thread = await api.getThread(id);
      conversationActor.send({ type: 'THREAD_LOADED', thread });
      
      // Connect to realtime
      connectionActor.send({ type: 'CONNECT', sessionId: id });
      await realtimeClient.connect(id);
      connectionActor.send({ type: 'CONNECTED' });
    } catch (error) {
      logger.error('Failed to load thread', { threadId: id, error: String(error) });
      conversationActor.send({ type: 'LOAD_FAILED', error: String(error) });
    }
  }

  function handleLlmEvent(event: LlmEvent) {
    switch (event.type) {
      case 'text_delta':
        conversationActor.send({ type: 'LLM_TEXT_DELTA', content: event.content });
        break;
      case 'tool_call_delta':
        conversationActor.send({
          type: 'LLM_TOOL_CALL_DELTA',
          callId: event.callId,
          toolName: event.toolName,
          argsFragment: event.argsFragment,
        });
        break;
      case 'completed':
        conversationActor.send({ type: 'LLM_COMPLETED', response: event.response });
        break;
      case 'error':
        conversationActor.send({ type: 'LLM_ERROR', error: event.error });
        break;
    }
  }

  function handleSendMessage(content: string) {
    logger.info('Sending message', { threadId, content: content.slice(0, 50) });
    conversationActor.send({ type: 'USER_INPUT', content });
    // In a full implementation, this would send to the server
  }

  onMount(() => {
    conversationActor.start();
    connectionActor.start();
    
    if (threadId) {
      loadThread(threadId);
    }

    // Subscribe to realtime events
    const unsubscribe = realtimeClient.onLlmEvent(handleLlmEvent);
    
    return () => {
      unsubscribe();
    };
  });

  onDestroy(() => {
    conversationActor.stop();
    connectionActor.stop();
    realtimeClient.disconnect();
  });

  // Reload when threadId changes
  $effect(() => {
    if (threadId && conversationState.context.thread?.id !== threadId) {
      loadThread(threadId);
    }
  });

  const ctx = $derived(conversationState.context);
  const isLoading = $derived(conversationState.value === 'loading');
  const isLoaded = $derived(typeof conversationState.value === 'object' && 'loaded' in conversationState.value);
  const canSend = $derived(ctx.currentAgentState === 'waiting_for_user_input');
</script>

<div class="flex flex-col h-full">
  <!-- Header -->
  <header class="border-b border-border p-4 flex items-center justify-between">
    <div class="flex items-center gap-3">
      <h2 class="font-semibold text-fg">
        {#if isLoading}
          <Skeleton width="200px" height="1.5rem" />
        {:else}
          {ctx.thread?.metadata.title || `Thread ${threadId?.slice(0, 12)}...`}
        {/if}
      </h2>
      {#if isLoaded}
        <AgentStateBadge state={ctx.currentAgentState} />
      {/if}
    </div>
    <ConnectionStatusIndicator status={connectionState.value} />
  </header>

  {#if isLoading}
    <div class="flex-1 flex items-center justify-center">
      <div class="animate-spin h-8 w-8 border-4 border-accent border-t-transparent rounded-full"></div>
    </div>
  {:else if conversationState.value === 'error'}
    <div class="flex-1 flex items-center justify-center">
      <Card padding="lg">
        <p class="text-error mb-4">{ctx.error}</p>
        <button
          class="text-accent hover:underline"
          onclick={() => threadId && loadThread(threadId)}
        >
          Try again
        </button>
      </Card>
    </div>
  {:else if isLoaded}
    <!-- Agent state timeline -->
    <div class="p-4 border-b border-border">
      <AgentStateTimeline
        currentState={ctx.currentAgentState}
        retries={ctx.retries}
        pendingToolCalls={ctx.toolExecutions.filter(t => t.type !== 'completed').map(t => t.call_id)}
      />
    </div>

    <!-- Messages -->
    <MessageList
      messages={ctx.messages}
      streamingContent={ctx.streamingContent}
      isStreaming={ctx.currentAgentState === 'calling_llm' && ctx.streamingContent.length > 0}
    />

    <!-- Tool execution panel -->
    {#if ctx.toolExecutions.length > 0}
      <div class="p-4 border-t border-border">
        <ToolExecutionPanel executions={ctx.toolExecutions} />
      </div>
    {/if}

    <!-- Message input -->
    <MessageInput
      disabled={!canSend}
      onSubmit={handleSendMessage}
    />
  {/if}
</div>
