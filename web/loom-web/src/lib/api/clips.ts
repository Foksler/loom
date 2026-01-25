/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

/**
 * Clips API types and client methods.
 */

export type ClipVisibility = 'private' | 'internal' | 'public';

export interface Clip {
	id: string;
	owner: string;
	name: string;
	description: string | null;
	visibility: ClipVisibility;
	created_by: string;
	org_id: string | null;
	is_fork: boolean;
	forked_from: string | null;
	file_count: number;
	size_bytes: number;
	language: string | null;
	clone_url: string;
	created_at: string;
	updated_at: string;
}

export interface ClipFile {
	path: string;
	content: string;
	size_bytes: number;
	is_redacted: boolean;
	language: string | null;
}

export interface ListClipsResponse {
	clips: Clip[];
}

export interface ListClipFilesResponse {
	files: string[];
}

export interface CreateClipRequest {
	name: string;
	description?: string;
	visibility?: ClipVisibility;
	org_id?: string;
}

export interface UpdateClipRequest {
	name?: string;
	description?: string;
	visibility?: ClipVisibility;
}

export interface ForkClipRequest {
	name?: string;
	org_id?: string;
}

export interface ListClipsParams {
	limit?: number;
	offset?: number;
	language?: string;
	visibility?: ClipVisibility;
}

export class ClipsApiClient {
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
			throw new Error(`API Error: ${response.status} ${body}`);
		}

		if (response.status === 204) {
			return undefined as T;
		}

		return response.json();
	}

	async createClip(request: CreateClipRequest): Promise<Clip> {
		return this.request<Clip>('/api/clips', {
			method: 'POST',
			body: JSON.stringify(request),
		});
	}

	async getClip(owner: string, name: string): Promise<Clip> {
		return this.request<Clip>(
			`/api/clips/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`
		);
	}

	async updateClip(owner: string, name: string, request: UpdateClipRequest): Promise<Clip> {
		return this.request<Clip>(
			`/api/clips/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`,
			{
				method: 'PATCH',
				body: JSON.stringify(request),
			}
		);
	}

	async deleteClip(owner: string, name: string): Promise<void> {
		await this.request<void>(
			`/api/clips/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`,
			{
				method: 'DELETE',
			}
		);
	}

	async listUserClips(params: ListClipsParams = {}): Promise<ListClipsResponse> {
		const query = new URLSearchParams();
		if (params.limit) query.set('limit', String(params.limit));
		if (params.offset) query.set('offset', String(params.offset));
		if (params.language) query.set('language', params.language);
		if (params.visibility) query.set('visibility', params.visibility);

		const queryStr = query.toString();
		const path = queryStr ? `/api/clips/user?${queryStr}` : '/api/clips/user';
		return this.request<ListClipsResponse>(path);
	}

	async listOrgClips(orgId: string, params: ListClipsParams = {}): Promise<ListClipsResponse> {
		const query = new URLSearchParams();
		if (params.limit) query.set('limit', String(params.limit));
		if (params.offset) query.set('offset', String(params.offset));
		if (params.language) query.set('language', params.language);
		if (params.visibility) query.set('visibility', params.visibility);

		const queryStr = query.toString();
		const path = queryStr
			? `/api/clips/org/${encodeURIComponent(orgId)}?${queryStr}`
			: `/api/clips/org/${encodeURIComponent(orgId)}`;
		return this.request<ListClipsResponse>(path);
	}

	async listPublicClips(params: ListClipsParams = {}): Promise<ListClipsResponse> {
		const query = new URLSearchParams();
		if (params.limit) query.set('limit', String(params.limit));
		if (params.offset) query.set('offset', String(params.offset));
		if (params.language) query.set('language', params.language);

		const queryStr = query.toString();
		const path = queryStr ? `/api/clips/explore?${queryStr}` : '/api/clips/explore';
		return this.request<ListClipsResponse>(path);
	}

	async listClipFiles(owner: string, name: string): Promise<ListClipFilesResponse> {
		return this.request<ListClipFilesResponse>(
			`/api/clips/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/files`
		);
	}

	async getClipFile(owner: string, name: string, filePath: string): Promise<ClipFile> {
		return this.request<ClipFile>(
			`/api/clips/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/files/${encodeURIComponent(filePath)}`
		);
	}

	async forkClip(owner: string, name: string, request: ForkClipRequest = {}): Promise<Clip> {
		return this.request<Clip>(
			`/api/clips/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/fork`,
			{
				method: 'POST',
				body: JSON.stringify(request),
			}
		);
	}
}

let clipsClient: ClipsApiClient | null = null;

export function getClipsClient(): ClipsApiClient {
	if (!clipsClient) {
		clipsClient = new ClipsApiClient();
	}
	return clipsClient;
}
