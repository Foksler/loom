import { createMachine, assign } from 'xstate';
import type { ConversationContext } from './types';
import type { Thread, LlmResponse, ToolProgress, ToolExecutionOutcome, AgentStateKind } from '../api/types';

type ConversationEvent =
  | { type: 'LOAD_THREAD'; threadId: string }
  | { type: 'THREAD_LOADED'; thread: Thread }
  | { type: 'LOAD_FAILED'; error: string }
  | { type: 'USER_INPUT'; content: string }
  | { type: 'LLM_TEXT_DELTA'; content: string }
  | { type: 'LLM_TOOL_CALL_DELTA'; callId: string; toolName: string; argsFragment: string }
  | { type: 'LLM_COMPLETED'; response: LlmResponse }
  | { type: 'LLM_ERROR'; error: string }
  | { type: 'TOOL_PROGRESS'; callId: string; progress: ToolProgress }
  | { type: 'TOOL_COMPLETED'; callId: string; outcome: ToolExecutionOutcome }
  | { type: 'ALL_TOOLS_COMPLETED' }
  | { type: 'HOOK_COMPLETED' }
  | { type: 'RETRY' }
  | { type: 'SHUTDOWN_REQUESTED' };

const initialContext: ConversationContext = {
  thread: null,
  messages: [],
  currentAgentState: 'waiting_for_user_input',
  toolExecutions: [],
  streamingContent: '',
  error: null,
  retries: 0,
  maxRetries: 3,
};

export const conversationMachine = createMachine({
  id: 'conversation',
  initial: 'idle',
  context: initialContext,
  types: {} as {
    context: ConversationContext;
    events: ConversationEvent;
  },
  states: {
    idle: {
      on: {
        LOAD_THREAD: {
          target: 'loading',
        },
      },
    },
    loading: {
      on: {
        THREAD_LOADED: {
          target: 'loaded',
          actions: assign({
            thread: ({ event }) => event.thread,
            messages: ({ event }) => event.thread.conversation.messages,
            currentAgentState: ({ event }) => event.thread.agent_state.kind,
            error: null,
          }),
        },
        LOAD_FAILED: {
          target: 'error',
          actions: assign({
            error: ({ event }) => event.error,
          }),
        },
      },
    },
    loaded: {
      initial: 'waitingForUserInput',
      states: {
        waitingForUserInput: {
          entry: assign({
            currentAgentState: 'waiting_for_user_input' as AgentStateKind,
            streamingContent: '',
          }),
          on: {
            USER_INPUT: {
              target: 'callingLlm',
              actions: assign({
                messages: ({ context, event }) => [
                  ...context.messages,
                  {
                    id: `m-${Date.now()}`,
                    role: 'user' as const,
                    content: event.content,
                    tool_name: null,
                    tool_call_id: null,
                    tool_input: null,
                    tool_output: null,
                    created_at: new Date().toISOString(),
                  },
                ],
              }),
            },
          },
        },
        callingLlm: {
          entry: assign({
            currentAgentState: 'calling_llm' as AgentStateKind,
            streamingContent: '',
          }),
          on: {
            LLM_TEXT_DELTA: {
              actions: assign({
                streamingContent: ({ context, event }) => context.streamingContent + event.content,
              }),
            },
            LLM_TOOL_CALL_DELTA: {
              actions: assign({
                toolExecutions: ({ context, event }) => {
                  const existing = context.toolExecutions.find(
                    (t) => t.type === 'pending' && t.call_id === event.callId
                  );
                  if (existing) return context.toolExecutions;
                  return [
                    ...context.toolExecutions,
                    {
                      type: 'pending' as const,
                      call_id: event.callId,
                      tool_name: event.toolName,
                      requested_at: new Date().toISOString(),
                    },
                  ];
                },
              }),
            },
            LLM_COMPLETED: {
              target: 'processingLlmResponse',
              actions: assign({
                messages: ({ context }) => {
                  if (!context.streamingContent) return context.messages;
                  return [
                    ...context.messages,
                    {
                      id: `m-${Date.now()}`,
                      role: 'assistant' as const,
                      content: context.streamingContent,
                      tool_name: null,
                      tool_call_id: null,
                      tool_input: null,
                      tool_output: null,
                      created_at: new Date().toISOString(),
                    },
                  ];
                },
                streamingContent: '',
              }),
            },
            LLM_ERROR: {
              target: 'error',
              actions: assign({
                error: ({ event }) => event.error,
                retries: ({ context }) => context.retries + 1,
              }),
            },
          },
        },
        processingLlmResponse: {
          entry: assign({
            currentAgentState: 'processing_llm_response' as AgentStateKind,
          }),
          always: [
            {
              guard: ({ context }) => context.toolExecutions.some((t) => t.type === 'pending'),
              target: 'executingTools',
            },
            {
              target: 'waitingForUserInput',
            },
          ],
        },
        executingTools: {
          entry: assign({
            currentAgentState: 'executing_tools' as AgentStateKind,
          }),
          on: {
            TOOL_PROGRESS: {
              actions: assign({
                toolExecutions: ({ context, event }) =>
                  context.toolExecutions.map((t) =>
                    t.call_id === event.callId && t.type === 'pending'
                      ? {
                          type: 'running' as const,
                          call_id: t.call_id,
                          tool_name: t.tool_name,
                          started_at: new Date().toISOString(),
                          progress: event.progress,
                        }
                      : t
                  ),
              }),
            },
            TOOL_COMPLETED: {
              actions: assign({
                toolExecutions: ({ context, event }) =>
                  context.toolExecutions.map((t) =>
                    t.call_id === event.callId
                      ? {
                          type: 'completed' as const,
                          call_id: t.call_id,
                          tool_name: t.tool_name,
                          outcome: event.outcome,
                        }
                      : t
                  ),
              }),
            },
            ALL_TOOLS_COMPLETED: {
              target: 'postToolsHook',
            },
          },
        },
        postToolsHook: {
          entry: assign({
            currentAgentState: 'post_tools_hook' as AgentStateKind,
          }),
          on: {
            HOOK_COMPLETED: {
              target: 'callingLlm',
              actions: assign({
                toolExecutions: [],
              }),
            },
          },
        },
        error: {
          entry: assign({
            currentAgentState: 'error' as AgentStateKind,
          }),
          on: {
            RETRY: [
              {
                guard: ({ context }) => context.retries < context.maxRetries,
                target: 'callingLlm',
              },
              {
                target: 'waitingForUserInput',
                actions: assign({
                  error: null,
                  retries: 0,
                }),
              },
            ],
          },
        },
      },
      on: {
        SHUTDOWN_REQUESTED: {
          target: 'shuttingDown',
        },
      },
    },
    shuttingDown: {
      entry: assign({
        currentAgentState: 'shutting_down' as AgentStateKind,
      }),
      type: 'final',
    },
    error: {
      on: {
        LOAD_THREAD: {
          target: 'loading',
          actions: assign({
            error: null,
          }),
        },
      },
    },
  },
});

export type ConversationMachine = typeof conversationMachine;
