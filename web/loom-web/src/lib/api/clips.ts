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
	star_count: number;
	clone_url: string;
	created_at: string;
	updated_at: string;
}

export interface ClipFile {
	path: string;
	content: string;
	size: number;
	is_redacted: boolean;
	language: string | null;
}

export interface ClipRevision {
	sha: string;
	author_name: string;
	author_email: string;
	timestamp: string;
	message: string;
}

export interface ListClipsResponse {
	clips: Clip[];
}

export interface ListClipFilesResponse {
	files: ClipFile[];
	revision: string;
}

export interface ClipFilesResponse {
	files: ClipFile[];
	revision: string;
}

export interface ClipRevisionsResponse {
	revisions: ClipRevision[];
}

export interface StarClipResponse {
	starred: boolean;
	star_count: number;
}

export interface CreateClipFile {
	path: string;
	content: string;
}

export interface CreateClipRequest {
	name: string;
	description?: string;
	visibility?: ClipVisibility;
	org_id?: string | null;
}

export interface UpdateClipRequest {
	name?: string;
	description?: string;
	visibility?: ClipVisibility;
}

export interface UpdateFilesRequest {
	files: CreateClipFile[];
	message?: string;
}

export interface ForkClipRequest {
	target_org_id: string;
	name?: string;
}

export interface ListClipsParams {
	limit?: number;
	offset?: number;
	language?: string;
	visibility?: ClipVisibility;
}

export interface SearchClipsParams {
	q: string;
	page?: number;
	per_page?: number;
}

export interface ClipSearchHit {
	clip: Clip;
	score: number;
}

export interface ClipSearchResponse {
	hits: ClipSearchHit[];
	total: number;
}

export class ClipsApiClient {
	constructor(private baseUrl: string = '') {}

	private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
		const url = `${this.baseUrl}${path}`;
		const response = await fetch(url, {
			...options,
			credentials: 'include',
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

	// =========================================================================
	// Clip CRUD
	// =========================================================================

	async createClip(request: CreateClipRequest): Promise<Clip> {
		return this.request<Clip>('/api/clips', {
			method: 'POST',
			body: JSON.stringify(request),
		});
	}

	async getClipByOwnerName(owner: string, name: string): Promise<Clip> {
		return this.request<Clip>(
			`/api/clips/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`
		);
	}

	async getClipById(id: string): Promise<Clip> {
		return this.request<Clip>(`/api/clips/${encodeURIComponent(id)}`);
	}

	async updateClip(id: string, request: UpdateClipRequest): Promise<Clip> {
		return this.request<Clip>(`/api/clips/${encodeURIComponent(id)}`, {
			method: 'PATCH',
			body: JSON.stringify(request),
		});
	}

	async deleteClip(id: string): Promise<void> {
		await this.request<void>(`/api/clips/${encodeURIComponent(id)}`, {
			method: 'DELETE',
		});
	}

	// =========================================================================
	// Clip Listings
	// =========================================================================

	async listUserClips(userId: string, params: ListClipsParams = {}): Promise<ListClipsResponse> {
		const query = new URLSearchParams();
		if (params.limit) query.set('limit', String(params.limit));
		if (params.offset) query.set('offset', String(params.offset));
		if (params.language) query.set('language', params.language);
		if (params.visibility) query.set('visibility', params.visibility);

		const queryStr = query.toString();
		const path = `/api/users/${encodeURIComponent(userId)}/clips${queryStr ? `?${queryStr}` : ''}`;
		return this.request<ListClipsResponse>(path);
	}

	async listOrgClips(orgId: string, params: ListClipsParams = {}): Promise<ListClipsResponse> {
		const query = new URLSearchParams();
		if (params.limit) query.set('limit', String(params.limit));
		if (params.offset) query.set('offset', String(params.offset));
		if (params.language) query.set('language', params.language);
		if (params.visibility) query.set('visibility', params.visibility);

		const queryStr = query.toString();
		const path = `/api/orgs/${encodeURIComponent(orgId)}/clips${queryStr ? `?${queryStr}` : ''}`;
		return this.request<ListClipsResponse>(path);
	}

	async listPublicClips(params: ListClipsParams = {}): Promise<ListClipsResponse> {
		const query = new URLSearchParams();
		if (params.limit) query.set('limit', String(params.limit));
		if (params.offset) query.set('offset', String(params.offset));
		if (params.language) query.set('language', params.language);

		const queryStr = query.toString();
		const path = `/api/clips/public${queryStr ? `?${queryStr}` : ''}`;
		return this.request<ListClipsResponse>(path);
	}

	async listStarredClips(params: ListClipsParams = {}): Promise<ListClipsResponse> {
		const query = new URLSearchParams();
		if (params.limit) query.set('limit', String(params.limit));
		if (params.offset) query.set('offset', String(params.offset));

		const queryStr = query.toString();
		const path = `/api/clips/starred${queryStr ? `?${queryStr}` : ''}`;
		return this.request<ListClipsResponse>(path);
	}

	async searchClips(params: SearchClipsParams): Promise<ClipSearchResponse> {
		const query = new URLSearchParams();
		query.set('q', params.q);
		if (params.page) query.set('page', String(params.page));
		if (params.per_page) query.set('per_page', String(params.per_page));

		const path = `/api/clips/search?${query.toString()}`;
		return this.request<ClipSearchResponse>(path);
	}

	// =========================================================================
	// Files
	// =========================================================================

	async listClipFiles(id: string, revision?: string): Promise<ListClipFilesResponse> {
		const query = revision ? `?ref=${encodeURIComponent(revision)}` : '';
		return this.request<ListClipFilesResponse>(
			`/api/clips/${encodeURIComponent(id)}/files${query}`
		);
	}

	async getClipFile(id: string, filePath: string, revision?: string): Promise<ClipFile> {
		const query = revision ? `?ref=${encodeURIComponent(revision)}` : '';
		return this.request<ClipFile>(
			`/api/clips/${encodeURIComponent(id)}/files/${filePath}${query}`
		);
	}

	async getClipFileRaw(id: string, filePath: string, revision?: string): Promise<string> {
		const query = revision ? `?ref=${encodeURIComponent(revision)}` : '';
		const url = `${this.baseUrl}/api/clips/${encodeURIComponent(id)}/raw/${filePath}${query}`;
		const response = await fetch(url, { credentials: 'include' });

		if (!response.ok) {
			throw new Error(`API Error: ${response.status}`);
		}

		return response.text();
	}

	async updateClipFiles(id: string, request: UpdateFilesRequest): Promise<ClipFilesResponse> {
		return this.request<ClipFilesResponse>(
			`/api/clips/${encodeURIComponent(id)}/files`,
			{
				method: 'POST',
				body: JSON.stringify(request),
			}
		);
	}

	// =========================================================================
	// Revisions
	// =========================================================================

	async listClipRevisions(id: string, limit?: number): Promise<ClipRevisionsResponse> {
		const query = limit ? `?limit=${limit}` : '';
		return this.request<ClipRevisionsResponse>(
			`/api/clips/${encodeURIComponent(id)}/revisions${query}`
		);
	}

	// =========================================================================
	// Stars
	// =========================================================================

	async starClip(id: string): Promise<StarClipResponse> {
		return this.request<StarClipResponse>(
			`/api/clips/${encodeURIComponent(id)}/star`,
			{ method: 'POST' }
		);
	}

	async unstarClip(id: string): Promise<StarClipResponse> {
		return this.request<StarClipResponse>(
			`/api/clips/${encodeURIComponent(id)}/star`,
			{ method: 'DELETE' }
		);
	}

	async getClipStarStatus(id: string): Promise<StarClipResponse> {
		return this.request<StarClipResponse>(
			`/api/clips/${encodeURIComponent(id)}/starred`
		);
	}

	// =========================================================================
	// Fork
	// =========================================================================

	async forkClip(id: string, request: ForkClipRequest): Promise<Clip> {
		return this.request<Clip>(
			`/api/clips/${encodeURIComponent(id)}/fork`,
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
