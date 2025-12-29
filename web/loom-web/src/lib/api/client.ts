/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import type {
	Thread,
	ListResponse,
	SearchResponse,
	ThreadVisibility,
	ListParams,
	SearchParams,
} from './types';
import { ApiError } from './types';

export class LoomApiClient {
	constructor(private baseUrl: string = '') {}

	private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
		const url = `${this.baseUrl}${path}`;
		const response = await fetch(url, {
			...options,
			headers: {
				'Content-Type': 'application/json',
				...options.headers,
			},
		});

		if (!response.ok) {
			const body = await response.text();
			throw new ApiError(response.status, body);
		}

		if (response.status === 204) {
			return undefined as T;
		}

		return response.json();
	}

	// Thread operations
	async listThreads(params: ListParams = {}): Promise<ListResponse> {
		const query = new URLSearchParams();
		if (params.workspace) query.set('workspace', params.workspace);
		if (params.limit) query.set('limit', String(params.limit));
		if (params.offset) query.set('offset', String(params.offset));

		const queryStr = query.toString();
		const path = queryStr ? `/api/threads?${queryStr}` : '/api/threads';
		return this.request<ListResponse>(path);
	}

	async getThread(id: string): Promise<Thread> {
		return this.request<Thread>(`/api/threads/${encodeURIComponent(id)}`);
	}

	async createOrUpdateThread(thread: Thread, expectedVersion?: number): Promise<Thread> {
		const headers: Record<string, string> = {};
		if (expectedVersion !== undefined) {
			headers['If-Match'] = String(expectedVersion);
		}

		return this.request<Thread>(`/api/threads/${encodeURIComponent(thread.id)}`, {
			method: 'PUT',
			headers,
			body: JSON.stringify(thread),
		});
	}

	async deleteThread(id: string): Promise<void> {
		await this.request<void>(`/api/threads/${encodeURIComponent(id)}`, {
			method: 'DELETE',
		});
	}

	async searchThreads(query: string, params: SearchParams = {}): Promise<SearchResponse> {
		const searchParams = new URLSearchParams({ q: query });
		if (params.limit) searchParams.set('limit', String(params.limit));
		if (params.offset) searchParams.set('offset', String(params.offset));
		if (params.workspace) searchParams.set('workspace', params.workspace);

		return this.request<SearchResponse>(`/api/threads/search?${searchParams}`);
	}

	async updateVisibility(
		id: string,
		visibility: ThreadVisibility,
		expectedVersion?: number
	): Promise<Thread> {
		const headers: Record<string, string> = {};
		if (expectedVersion !== undefined) {
			headers['If-Match'] = String(expectedVersion);
		}

		return this.request<Thread>(`/api/threads/${encodeURIComponent(id)}/visibility`, {
			method: 'POST',
			headers,
			body: JSON.stringify({ visibility }),
		});
	}

	// Health check
	async healthCheck(): Promise<{ status: string }> {
		return this.request<{ status: string }>('/health');
	}
}

// Singleton instance for convenience
let defaultClient: LoomApiClient | null = null;

export function getApiClient(baseUrl?: string): LoomApiClient {
	if (!defaultClient || baseUrl) {
		defaultClient = new LoomApiClient(baseUrl);
	}
	return defaultClient;
}
