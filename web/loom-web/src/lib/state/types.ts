import type { Thread, AgentStateKind, ToolExecutionStatus, MessageSnapshot } from '../api/types';

export interface ConversationContext {
  thread: Thread | null;
  messages: MessageSnapshot[];
  currentAgentState: AgentStateKind;
  toolExecutions: ToolExecutionStatus[];
  streamingContent: string;
  error: string | null;
  retries: number;
  maxRetries: number;
}

export interface ConnectionContext {
  sessionId: string | null;
  retries: number;
  maxRetries: number;
  lastError: string | null;
}

export interface ThreadListContext {
  threads: import('../api/types').ThreadSummary[];
  searchQuery: string;
  limit: number;
  offset: number;
  total: number;
  error: string | null;
  isSearching: boolean;
}
