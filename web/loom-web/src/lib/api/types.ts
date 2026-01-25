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

export interface WsTokenResponse {
	token: string;
	expires_in: number;
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
	username?: string;
	locale?: string;
}

// Impersonation types
export interface ImpersonationState {
	is_impersonating: boolean;
	original_user?: {
		id: string;
		display_name: string;
	};
	impersonated_user?: {
		id: string;
		display_name: string;
	};
}

export interface ImpersonateResponse {
	message: string;
	impersonated_user: {
		id: string;
		display_name: string;
	};
}

export interface StopImpersonationResponse {
	message: string;
}

// Admin user list types
export interface AdminUser {
	id: string;
	display_name: string;
	primary_email: string | null;
	avatar_url: string | null;
	is_system_admin: boolean;
	is_support: boolean;
	is_auditor: boolean;
	created_at: string;
	updated_at: string;
	deleted_at: string | null;
}

export interface AdminUserListResponse {
	users: AdminUser[];
	total: number;
	limit: number;
	offset: number;
}

// Admin role update types
export interface UpdateUserRolesRequest {
	is_system_admin?: boolean;
	is_support?: boolean;
	is_auditor?: boolean;
}

export interface UpdateUserRolesResponse {
	id: string;
	display_name: string;
	primary_email: string | null;
	avatar_url: string | null;
	is_system_admin: boolean;
	is_support: boolean;
	is_auditor: boolean;
	created_at: string;
	updated_at: string;
	deleted_at: string | null;
}

export interface DeleteUserResponse {
	message: string;
	user_id: string;
}

// Weaver types
export type WeaverStatus = 'pending' | 'running' | 'succeeded' | 'failed' | 'terminating';

export interface Weaver {
	id: string;
	pod_name: string;
	status: WeaverStatus;
	created_at: string;
	image?: string;
	tags?: Record<string, string>;
	lifetime_hours?: number;
	age_hours?: number;
	owner_user_id?: string;
}

export interface ListWeaversResponse {
	weavers: Weaver[];
	count: number;
}

export interface CreateWeaverRequest {
	image: string;
	org_id: string;
	env?: Record<string, string>;
	resources?: {
		memory_limit?: string;
		cpu_limit?: string;
	};
	tags?: Record<string, string>;
	lifetime_hours?: number;
	command?: string[];
	args?: string[];
	workdir?: string;
}

// Support Access types
export type SupportAccessStatus = 'pending' | 'approved' | 'revoked' | 'expired';

export interface SupportAccessRequest {
	request_id: string;
	thread_id: string;
	requested_at: string;
	status: SupportAccessStatus;
}

export interface SupportAccessApproval {
	thread_id: string;
	granted_to: string;
	approved_at: string;
	expires_at: string;
}

export interface SupportAccessResponse {
	message: string;
}

export interface SupportAccessErrorResponse {
	message: string;
	code: string;
}

// Health check types
export type HealthStatus = 'healthy' | 'degraded' | 'unhealthy' | 'unknown';

export interface DatabaseHealth {
	status: HealthStatus;
	latency_ms: number;
	error?: string;
}

export interface BinDirHealth {
	status: HealthStatus;
	latency_ms: number;
	path: string;
	exists: boolean;
	is_dir: boolean;
	file_count?: number;
	error?: string;
}

export interface AnthropicAccountHealth {
	id: string;
	status: 'available' | 'cooling_down' | 'disabled';
	cooldown_remaining_secs?: number;
	last_error?: string;
}

export interface AnthropicPoolHealth {
	accounts_total: number;
	accounts_available: number;
	accounts_cooling: number;
	accounts_disabled: number;
	accounts: AnthropicAccountHealth[];
}

export interface LlmProviderHealth {
	name: string;
	status: HealthStatus;
	mode?: string;
	pool?: AnthropicPoolHealth;
	latency_ms?: number;
	error?: string;
}

export interface LlmProvidersHealth {
	status: HealthStatus;
	providers: LlmProviderHealth[];
}

export interface GoogleCseHealth {
	status: HealthStatus;
	latency_ms: number;
	configured: boolean;
	error?: string;
}

export interface SerperHealth {
	status: HealthStatus;
	latency_ms: number;
	configured: boolean;
	error?: string;
}

export interface GithubAppHealth {
	status: HealthStatus;
	latency_ms: number;
	configured: boolean;
	error?: string;
}

export interface KubernetesHealth {
	status: HealthStatus;
	latency_ms: number;
	namespace: string;
	reachable: boolean;
	error?: string;
}

export interface SmtpHealth {
	status: HealthStatus;
	latency_ms: number;
	configured: boolean;
	healthy: boolean;
	error?: string;
}

export interface GeoIpHealth {
	status: HealthStatus;
	latency_ms: number;
	configured: boolean;
	healthy: boolean;
	database_path?: string;
	database_type?: string;
	error?: string;
}

export interface JobsHealth {
	status: HealthStatus;
	jobs_total: number;
	jobs_healthy: number;
	jobs_failing: number;
	failing_jobs?: string[];
}

export interface AuthProviderHealth {
	name: string;
	status: HealthStatus;
	configured: boolean;
	error?: string;
}

export interface AuthProvidersHealth {
	status: HealthStatus;
	providers: AuthProviderHealth[];
}

export interface ScimHealth {
	status: HealthStatus;
	enabled: boolean;
	configured: boolean;
	org_id?: string;
	org_exists: boolean;
	error?: string;
}

export interface SecretsHealth {
	status: HealthStatus;
	latency_ms: number;
	configured: boolean;
	master_key_present: boolean;
	svid_signing_key_present: boolean;
	error?: string;
}

export interface WhatsAppHealth {
	status: HealthStatus;
	latency_ms: number;
	configured: boolean;
	configs_count: number;
	error?: string;
}

export interface HealthComponents {
	database: DatabaseHealth;
	bin_dir: BinDirHealth;
	llm_providers: LlmProvidersHealth;
	google_cse: GoogleCseHealth;
	serper: SerperHealth;
	github_app: GithubAppHealth;
	kubernetes?: KubernetesHealth;
	smtp: SmtpHealth;
	geoip: GeoIpHealth;
	jobs?: JobsHealth;
	auth_providers: AuthProvidersHealth;
	scim: ScimHealth;
	secrets?: SecretsHealth;
	whatsapp?: WhatsAppHealth;
}

export interface HealthVersionInfo {
	git_sha: string;
}

export interface HealthResponse {
	status: HealthStatus;
	timestamp: string;
	duration_ms: number;
	version: HealthVersionInfo;
	components: HealthComponents;
}

// Server log types
export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';

export interface LogEntry {
	id: number;
	timestamp: string;
	level: LogLevel;
	target: string;
	message: string;
	fields?: [string, string][];
}

export interface ListLogsResponse {
	entries: LogEntry[];
	buffer_size: number;
	buffer_capacity: number;
	current_id: number;
}

// =============================================================================
// Crash Analytics Types
// =============================================================================

export interface CrashProject {
	id: string;
	org_id: string;
	name: string;
	slug: string;
	platform: CrashPlatform;
	created_at: string;
	updated_at: string;
}

export type CrashPlatform = 'javascript' | 'node' | 'rust' | 'other';

export interface CrashProjectListResponse {
	projects: CrashProject[];
}

export type IssueStatus = 'unresolved' | 'resolved' | 'ignored' | 'regressed';
export type IssueLevel = 'error' | 'warning' | 'info';
export type IssuePriority = 'high' | 'medium' | 'low';

export interface IssueMetadata {
	exception_type: string;
	exception_value: string;
	filename?: string;
	function?: string;
}

export interface Issue {
	id: string;
	org_id: string;
	project_id: string;
	short_id: string;
	fingerprint: string;
	title: string;
	culprit?: string;
	metadata: IssueMetadata;
	status: IssueStatus;
	level: IssueLevel;
	priority: IssuePriority;
	event_count: number;
	user_count: number;
	first_seen: string;
	last_seen: string;
	resolved_at?: string;
	resolved_by?: string;
	resolved_in_release?: string;
	times_regressed: number;
	last_regressed_at?: string;
	regressed_in_release?: string;
	assigned_to?: string;
	created_at: string;
	updated_at: string;
}

export interface IssueListResponse {
	issues: Issue[];
	total: number;
}

export interface CrashFrame {
	filename?: string;
	function?: string;
	lineno?: number;
	colno?: number;
	abs_path?: string;
	context_line?: string;
	pre_context?: string[];
	post_context?: string[];
	in_app: boolean;
}

export interface CrashStacktrace {
	frames: CrashFrame[];
}

export interface CrashBreadcrumb {
	timestamp: string;
	category: string;
	message?: string;
	level: string;
	data?: Record<string, unknown>;
}

export interface CrashUserContext {
	id?: string;
	email?: string;
	username?: string;
	ip_address?: string;
}

export interface CrashEvent {
	id: string;
	project_id: string;
	issue_id: string;
	platform: CrashPlatform;
	timestamp: string;
	received_at: string;
	release?: string;
	environment: string;
	exception_type: string;
	exception_value: string;
	stacktrace?: CrashStacktrace;
	raw_stacktrace?: CrashStacktrace;
	breadcrumbs?: CrashBreadcrumb[];
	user?: CrashUserContext;
	tags?: Record<string, string>;
	extra?: Record<string, unknown>;
	active_flags?: string[];
}

export interface CrashEventListResponse {
	events: CrashEvent[];
	total: number;
}

// =============================================================================
// Crons Monitoring Types
// =============================================================================

export type MonitorStatus = 'active' | 'paused' | 'disabled';
export type MonitorHealth = 'healthy' | 'failing' | 'missed' | 'timeout' | 'unknown';
export type CheckInStatus = 'ok' | 'error' | 'in_progress';

export interface MonitorSchedule {
	type: 'cron' | 'interval';
	expression?: string;
	minutes?: number;
}

export interface Monitor {
	id: string;
	org_id: string;
	slug: string;
	name: string;
	description?: string;
	status: MonitorStatus;
	health: MonitorHealth;
	schedule: MonitorSchedule;
	timezone: string;
	checkin_margin_minutes: number;
	max_runtime_minutes?: number;
	ping_key: string;
	environments: string[];
	last_checkin_at?: string;
	last_checkin_status?: CheckInStatus;
	next_expected_at?: string;
	consecutive_failures: number;
	total_checkins: number;
	total_failures: number;
	created_at: string;
	updated_at: string;
}

export interface MonitorListResponse {
	monitors: Monitor[];
}

export interface CheckIn {
	id: string;
	monitor_id: string;
	status: CheckInStatus;
	duration_ms?: number;
	environment?: string;
	output?: string;
	created_at: string;
}

export interface CheckInListResponse {
	checkins: CheckIn[];
	total: number;
}

export interface CreateMonitorRequest {
	slug: string;
	name: string;
	description?: string;
	schedule: MonitorSchedule;
	timezone?: string;
	checkin_margin_minutes?: number;
	max_runtime_minutes?: number;
	environments?: string[];
}

export interface UpdateMonitorRequest {
	name?: string;
	description?: string;
	schedule?: MonitorSchedule;
	timezone?: string;
	checkin_margin_minutes?: number;
	max_runtime_minutes?: number;
	environments?: string[];
}

// =============================================================================
// Sessions & Release Health Types
// =============================================================================

export type SessionStatus = 'active' | 'exited' | 'crashed' | 'abnormal';
export type AdoptionStage = 'new' | 'growing' | 'adopted' | 'replaced';

export interface AppSession {
	id: string;
	project_id: string;
	distinct_id: string;
	status: SessionStatus;
	release?: string;
	environment: string;
	platform: string;
	crashed: boolean;
	error_count: number;
	duration_ms?: number;
	started_at: string;
	ended_at?: string;
}

export interface AppSessionListResponse {
	sessions: AppSession[];
	total: number;
}

export interface ReleaseHealth {
	project_id: string;
	release: string;
	environment: string;
	total_sessions: number;
	crashed_sessions: number;
	errored_sessions: number;
	total_users: number;
	crashed_users: number;
	crash_free_session_rate: number;
	crash_free_user_rate: number;
	adoption_rate: number;
	adoption_stage: AdoptionStage;
	first_seen: string;
	last_seen: string;
	crash_free_rate_trend?: number;
}

export interface ReleaseHealthListResponse {
	releases: ReleaseHealth[];
}

export interface CrashFreeDataPoint {
	timestamp: string;
	crash_free_rate: number;
	total_sessions: number;
	crashed_sessions: number;
}

// Error class for API errors
// =============================================================================
// Product Analytics Types
// =============================================================================

export interface AnalyticsEvent {
	id: string;
	org_id: string;
	person_id?: string;
	distinct_id: string;
	event_name: string;
	properties: Record<string, unknown>;
	timestamp: string;
	ip_address?: string;
	user_agent?: string;
	lib?: string;
	lib_version?: string;
	created_at: string;
}

export interface AnalyticsEventListResponse {
	events: AnalyticsEvent[];
	has_more: boolean;
}

export interface AnalyticsEventCountResponse {
	count: number;
}

export interface AnalyticsPerson {
	id: string;
	org_id: string;
	properties: Record<string, unknown>;
	created_at: string;
	updated_at: string;
	merged_into_id?: string;
	merged_at?: string;
	identities: AnalyticsPersonIdentity[];
}

export interface AnalyticsPersonIdentity {
	id: string;
	person_id: string;
	distinct_id: string;
	identity_type: 'anonymous' | 'identified';
	created_at: string;
}

export interface AnalyticsPersonListResponse {
	persons: AnalyticsPerson[];
	has_more: boolean;
}

export interface AnalyticsListParams {
	limit?: number;
	offset?: number;
	event_name?: string;
	distinct_id?: string;
	person_id?: string;
	start_date?: string;
	end_date?: string;
}

// =========================================================================
// WhatsApp Types
// =========================================================================

export interface WhatsAppConfig {
	id: string;
	phone_number_id: string;
	enabled: boolean;
	webhook_url: string;
	created_at: string;
	updated_at: string;
}

export interface CreateWhatsAppConfigRequest {
	phone_number_id: string;
	access_token: string;
	app_secret: string;
	verify_token: string;
}

export interface WhatsAppGroup {
	id: string;
	name: string;
	description: string | null;
	color: string | null;
	is_default: boolean;
	created_at: string;
	updated_at: string;
}

export interface CreateWhatsAppGroupRequest {
	name: string;
	description?: string;
	color?: string;
}

export interface WhatsAppGroupListResponse {
	groups: WhatsAppGroup[];
}

export interface WhatsAppConversation {
	id: string;
	wa_phone_number: string;
	group_id: string | null;
	user_id: string | null;
	thread_id: string | null;
	last_customer_message_at: string;
	session_expires_at: string;
	session_active: boolean;
	status: string;
	created_at: string;
}

export interface WhatsAppConversationListResponse {
	conversations: WhatsAppConversation[];
}

export interface MoveConversationRequest {
	group_id: string | null;
}

export interface LinkPhoneRequest {
	phone_number: string;
}

export interface LinkPhoneResponse {
	message: string;
	expires_in_seconds: number;
}

export interface VerifyPhoneRequest {
	phone_number: string;
	otp: string;
}

export interface VerifyPhoneResponse {
	message: string;
	phone_number: string;
}

export interface WhatsAppSuccessResponse {
	message: string;
}

export interface WhatsAppErrorResponse {
	error: string;
	message: string;
}

export class ApiError extends Error {
	constructor(
		public readonly status: number,
		public readonly body: string
	) {
		super(`API Error ${status}: ${body}`);
		this.name = 'ApiError';
	}

	get statusCode(): number {
		return this.status;
	}

	get isForbidden(): boolean {
		return this.status === 403;
	}

	get isNotFound(): boolean {
		return this.status === 404;
	}

	getErrorCode(): string | null {
		try {
			const parsed = JSON.parse(this.body);
			return parsed.code || null;
		} catch {
			return null;
		}
	}
}
