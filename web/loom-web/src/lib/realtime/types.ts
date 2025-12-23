import type { LlmResponse } from '../api/types';

// Connection status
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error';

// LLM event types from server
export type LlmEventType = 'text_delta' | 'tool_call_delta' | 'completed' | 'error';

// LLM event wire format
export interface LlmEventWire {
  event_type: LlmEventType;
  content?: string;
  call_id?: string;
  tool_name?: string;
  arguments_fragment?: string;
  response?: LlmResponse;
  error?: string;
}

// Control command types
export type ControlCommand = 'ping' | 'pong' | 'close' | 'resume';

// Control message wire format
export interface ControlWire {
  command: ControlCommand;
  reason?: string;
  payload?: unknown;
}

// Ack wire format
export interface AckWire {
  acked_at: string;
  sequence?: number;
}

// Server query wire format (for future use)
export interface ServerQueryWire {
  kind: unknown;
  sent_at: string;
  timeout_secs: number;
  metadata?: Record<string, unknown>;
}

// Query response wire format
export interface QueryResponseWire {
  result?: unknown;
  error?: string;
  responded_at: string;
}

// All realtime message types
export type RealtimeMessage =
  | { type: 'llm_event'; id: string; data: LlmEventWire; timestamp: string }
  | { type: 'server_query'; id: string; data: ServerQueryWire; timestamp: string }
  | { type: 'query_response'; id: string; data: QueryResponseWire; timestamp: string }
  | { type: 'control'; id: string; data: ControlWire; timestamp: string }
  | { type: 'ack'; id: string; data: AckWire; timestamp: string };

// Parsed LLM event for UI consumption
export type LlmEvent =
  | { type: 'text_delta'; content: string }
  | { type: 'tool_call_delta'; callId: string; toolName: string; argsFragment: string }
  | { type: 'completed'; response: LlmResponse }
  | { type: 'error'; error: string };

// Event handler types
export type MessageHandler = (msg: RealtimeMessage) => void;
export type LlmEventHandler = (event: LlmEvent) => void;
export type StatusHandler = (status: ConnectionStatus) => void;
