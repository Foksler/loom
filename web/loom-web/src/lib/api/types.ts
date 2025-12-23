// Core API types for loom-web

export interface Thread {
  id: string;
  title: string | null;
  created_at: string;
  updated_at: string;
  message_count: number;
  metadata?: Record<string, unknown>;
}

export interface ThreadSummary {
  id: string;
  title: string | null;
  created_at: string;
  updated_at: string;
  message_count: number;
  last_message_preview?: string;
}

export interface MessageSnapshot {
  id: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  created_at: string;
  tool_calls?: ToolCall[];
  tool_call_id?: string;
}

export interface ToolCall {
  id: string;
  name: string;
  arguments: string;
}

export interface LlmResponse {
  id: string;
  model: string;
  content: string;
  tool_calls?: ToolCall[];
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
  finish_reason: 'stop' | 'tool_calls' | 'length' | 'content_filter' | null;
}

export type AgentStateKind =
  | 'idle'
  | 'thinking'
  | 'streaming'
  | 'tool_pending'
  | 'tool_executing'
  | 'waiting_input'
  | 'error';

export interface ToolExecutionStatus {
  call_id: string;
  tool_name: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  started_at?: string;
  completed_at?: string;
  result?: unknown;
  error?: string;
}

export interface ToolProgress {
  call_id: string;
  progress: number;
  message?: string;
}

export interface ToolExecutionOutcome {
  call_id: string;
  success: boolean;
  result?: unknown;
  error?: string;
}

// API request/response types
export interface ListParams {
  workspace?: string;
  limit?: number;
  offset?: number;
}

export interface SearchParams {
  workspace?: string;
  limit?: number;
  offset?: number;
}

export interface ListResponse {
  threads: ThreadSummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface SearchResponse {
  hits: SearchHit[];
  limit: number;
  offset: number;
}

export interface SearchHit {
  id: string;
  title: string | null;
  score: number;
  created_at: string;
  updated_at: string;
}

export type ThreadVisibility = 'public' | 'private' | 'unlisted';

// Error class for API errors
export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: string
  ) {
    super(`API Error ${status}: ${body}`);
    this.name = 'ApiError';
  }
}
