/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

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

export interface CurrentUser {
	id: string;
	display_name: string;
	email: string | null;
	avatar_url: string | null;
	locale: string | null;
	global_roles: string[];
	created_at: string;
}

// Organization types
export type OrgVisibility = 'public' | 'unlisted' | 'private';
export type OrgRole = 'owner' | 'admin' | 'member';
export type OrgJoinPolicy = 'open' | 'request' | 'invite_only';

export interface Org {
	id: string;
	name: string;
	slug: string;
	visibility: OrgVisibility;
	join_policy: OrgJoinPolicy;
	is_personal: boolean;
	created_at: string;
	updated_at: string;
	member_count: number | null;
}

export interface ListOrgsResponse {
	orgs: Org[];
}

export interface CreateOrgRequest {
	name: string;
	slug: string;
	visibility?: OrgVisibility;
}

export interface UpdateOrgRequest {
	name?: string;
	visibility?: OrgVisibility;
	join_policy?: OrgJoinPolicy;
}

export interface OrgMember {
	user_id: string;
	display_name: string;
	email: string | null;
	avatar_url: string | null;
	role: OrgRole;
	joined_at: string;
}

export interface OrgMemberListResponse {
	members: OrgMember[];
}

// Team types
export type TeamRole = 'maintainer' | 'member';

export interface Team {
	id: string;
	org_id: string;
	name: string;
	slug: string;
	created_at: string;
	updated_at: string;
	member_count: number;
}

export interface TeamMember {
	user_id: string;
	display_name: string;
	email: string | null;
	avatar_url: string | null;
	role: TeamRole;
	joined_at: string;
}

export interface TeamListResponse {
	teams: Team[];
}

export interface TeamMemberListResponse {
	members: TeamMember[];
}

export interface CreateTeamRequest {
	name: string;
	slug: string;
}

export interface UpdateTeamRequest {
	name?: string;
}

// API Key types
export interface ApiKey {
	id: string;
	name: string;
	prefix: string;
	scopes: string[];
	created_at: string;
	last_used_at: string | null;
	created_by: string;
}

export interface ApiKeyListResponse {
	api_keys: ApiKey[];
}

export interface CreateApiKeyRequest {
	name: string;
	scopes: string[];
}

export interface CreateApiKeyResponse {
	id: string;
	name: string;
	key: string;
	prefix: string;
	scopes: string[];
	created_at: string;
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

// Auth types
export interface AuthProvidersResponse {
	providers: string[];
}

export interface AuthSuccessResponse {
	message: string;
}

export interface MagicLinkRequest {
	email: string;
}

export interface DeviceCodeStartResponse {
	device_code: string;
	user_code: string;
	verification_url: string;
	expires_in: number;
	interval: number;
}

export type DeviceCodePollStatus = 'pending' | 'completed' | 'expired' | 'denied';

export interface DeviceCodePollResponse {
	status: DeviceCodePollStatus;
	access_token?: string;
}

export interface DeviceCodeCompleteRequest {
	user_code: string;
}

export interface Session {
	id: string;
	session_type: 'web' | 'cli' | 'vscode';
	created_at: string;
	last_used_at: string;
	ip_address: string | null;
	user_agent: string | null;
	geo_location: string | null;
	is_current: boolean;
}

export interface SessionListResponse {
	sessions: Session[];
}

export interface UpdateProfileRequest {
	display_name?: string;
	locale?: string;
}

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
