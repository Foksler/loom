/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

export * from './types';
export { LoomWebSocketClient } from './wsClient';
export { LoomSseClient } from './sseClient';
export { createAccumulator, accumulateEvent, accumulateTextDeltas } from './accumulator';
export type { AccumulatedContent } from './accumulator';

import { LoomWebSocketClient } from './wsClient';
import { LoomSseClient } from './sseClient';

export type RealtimeClient = LoomWebSocketClient | LoomSseClient;

export interface RealtimeClientOptions {
	serverUrl: string;
	preferWebSocket?: boolean;
}

export function createRealtimeClient(options: RealtimeClientOptions): RealtimeClient {
	const { serverUrl, preferWebSocket = true } = options;

	if (preferWebSocket && typeof WebSocket !== 'undefined') {
		return new LoomWebSocketClient(serverUrl);
	}

	return new LoomSseClient(serverUrl);
}
