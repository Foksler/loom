import type {
  ConnectionStatus,
  RealtimeMessage,
  LlmEvent,
  LlmEventWire,
  MessageHandler,
  LlmEventHandler,
  StatusHandler,
} from './types';
import { logger } from '../logging';

export class LoomWebSocketClient {
  private ws: WebSocket | null = null;
  private sessionId: string | null = null;
  private messageHandlers = new Set<MessageHandler>();
  private llmEventHandlers = new Set<LlmEventHandler>();
  private statusHandlers = new Set<StatusHandler>();
  private status: ConnectionStatus = 'disconnected';
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;

  constructor(private serverUrl: string) {}

  async connect(sessionId: string): Promise<void> {
    this.sessionId = sessionId;
    this.setStatus('connecting');

    const wsUrl = this.serverUrl.replace(/^http/, 'ws');
    const url = `${wsUrl}/v1/ws/sessions/${encodeURIComponent(sessionId)}`;

    try {
      this.ws = new WebSocket(url);

      this.ws.onopen = () => {
        logger.info('WebSocket connected', { sessionId });
        this.reconnectAttempts = 0;
        this.setStatus('connected');
      };

      this.ws.onclose = (event) => {
        logger.info('WebSocket closed', { sessionId, code: event.code, reason: event.reason });
        this.handleDisconnect();
      };

      this.ws.onerror = (error) => {
        logger.error('WebSocket error', { sessionId, error: String(error) });
        this.setStatus('error');
      };

      this.ws.onmessage = (event) => {
        this.handleMessage(event);
      };
    } catch (error) {
      logger.error('WebSocket connection failed', { sessionId, error: String(error) });
      this.setStatus('error');
      throw error;
    }
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }
    this.sessionId = null;
    this.setStatus('disconnected');
  }

  send(msg: RealtimeMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    } else {
      logger.warn('Cannot send message, WebSocket not connected', { sessionId: this.sessionId });
    }
  }

  onMessage(handler: MessageHandler): () => void {
    this.messageHandlers.add(handler);
    return () => this.messageHandlers.delete(handler);
  }

  onLlmEvent(handler: LlmEventHandler): () => void {
    this.llmEventHandlers.add(handler);
    return () => this.llmEventHandlers.delete(handler);
  }

  onStatus(handler: StatusHandler): () => void {
    this.statusHandlers.add(handler);
    return () => this.statusHandlers.delete(handler);
  }

  getStatus(): ConnectionStatus {
    return this.status;
  }

  private setStatus(status: ConnectionStatus): void {
    this.status = status;
    this.statusHandlers.forEach((h) => h(status));
  }

  private handleMessage(event: MessageEvent): void {
    try {
      const msg = JSON.parse(event.data) as RealtimeMessage;

      // Notify all message handlers
      this.messageHandlers.forEach((h) => h(msg));

      // Convert LLM events for convenience handlers
      if (msg.type === 'llm_event') {
        const llmEvent = this.convertLlmEvent(msg.data);
        if (llmEvent) {
          this.llmEventHandlers.forEach((h) => h(llmEvent));
        }
      }
    } catch (error) {
      logger.error('Failed to parse WebSocket message', { error: String(error) });
    }
  }

  private convertLlmEvent(wire: LlmEventWire): LlmEvent | null {
    switch (wire.event_type) {
      case 'text_delta':
        return { type: 'text_delta', content: wire.content || '' };
      case 'tool_call_delta':
        return {
          type: 'tool_call_delta',
          callId: wire.call_id || '',
          toolName: wire.tool_name || '',
          argsFragment: wire.arguments_fragment || '',
        };
      case 'completed':
        if (wire.response) {
          return { type: 'completed', response: wire.response };
        }
        return null;
      case 'error':
        return { type: 'error', error: wire.error || 'Unknown error' };
      default:
        return null;
    }
  }

  private handleDisconnect(): void {
    this.ws = null;

    if (this.reconnectAttempts < this.maxReconnectAttempts && this.sessionId) {
      this.setStatus('reconnecting');
      this.reconnectAttempts++;
      const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

      logger.info('Attempting reconnection', {
        sessionId: this.sessionId,
        attempt: this.reconnectAttempts,
        delay,
      });

      setTimeout(() => {
        if (this.sessionId) {
          this.connect(this.sessionId);
        }
      }, delay);
    } else {
      this.setStatus('error');
    }
  }
}
