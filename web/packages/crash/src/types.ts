/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

/**
 * Platform types for crash events.
 */
export type Platform = 'javascript' | 'node' | 'browser' | 'react-native';

/**
 * Breadcrumb severity level.
 */
export type BreadcrumbLevel = 'debug' | 'info' | 'warning' | 'error' | 'fatal';

/**
 * Issue severity level.
 */
export type IssueLevel = 'error' | 'warning' | 'info';

/**
 * Breadcrumb representing an event leading up to a crash.
 */
export interface Breadcrumb {
	/** Timestamp of the breadcrumb */
	timestamp?: string;
	/** Category (e.g., 'http', 'navigation', 'ui', 'console') */
	category?: string;
	/** Human-readable message */
	message?: string;
	/** Severity level */
	level?: BreadcrumbLevel;
	/** Type of breadcrumb (e.g., 'http', 'navigation', 'default') */
	type?: string;
	/** Additional data */
	data?: Record<string, unknown>;
}

/**
 * User context to attach to crash events.
 */
export interface UserContext {
	/** User ID */
	id?: string;
	/** User email */
	email?: string;
	/** Username */
	username?: string;
	/** IP address (usually set by server) */
	ip_address?: string;
	/** Additional user data */
	[key: string]: unknown;
}

/**
 * Device context for crash events.
 */
export interface DeviceContext {
	/** Device name */
	name?: string;
	/** Device family */
	family?: string;
	/** Device model */
	model?: string;
	/** Device model ID */
	model_id?: string;
	/** Screen resolution */
	screen_resolution?: string;
	/** Screen density */
	screen_density?: number;
}

/**
 * Browser context for crash events.
 */
export interface BrowserContext {
	/** Browser name */
	name?: string;
	/** Browser version */
	version?: string;
}

/**
 * OS context for crash events.
 */
export interface OsContext {
	/** OS name */
	name?: string;
	/** OS version */
	version?: string;
	/** Kernel version */
	kernel_version?: string;
}

/**
 * Request context for crash events.
 */
export interface RequestContext {
	/** Request URL */
	url?: string;
	/** HTTP method */
	method?: string;
	/** Request headers */
	headers?: Record<string, string>;
	/** Query string */
	query_string?: string;
	/** Request body (for POST) */
	data?: unknown;
}

/**
 * Single stack frame.
 */
export interface StackFrame {
	/** Function name */
	function?: string;
	/** Module name */
	module?: string;
	/** File path */
	filename?: string;
	/** Absolute file path */
	abs_path?: string;
	/** Line number */
	lineno?: number;
	/** Column number */
	colno?: number;
	/** Whether this frame is in the user's code (not library) */
	in_app?: boolean;
	/** Lines before the error line */
	pre_context?: string[];
	/** The error line itself */
	context_line?: string;
	/** Lines after the error line */
	post_context?: string[];
	/** Local variables (if available) */
	vars?: Record<string, unknown>;
}

/**
 * Stacktrace containing multiple frames.
 */
export interface Stacktrace {
	/** Stack frames (most recent call last) */
	frames: StackFrame[];
}

/**
 * Exception mechanism describing how the error was captured.
 */
export interface Mechanism {
	/** Type of mechanism (e.g., 'onerror', 'onunhandledrejection') */
	type: string;
	/** Whether the exception was handled */
	handled: boolean;
	/** Additional mechanism data */
	data?: Record<string, unknown>;
}

/**
 * A crash event to be sent to the server.
 */
export interface CrashEvent {
	/** Event ID (UUID) */
	event_id?: string;
	/** Project ID (required by SDK endpoints) */
	project_id?: string;
	/** Exception type (e.g., 'TypeError', 'ReferenceError') */
	exception_type: string;
	/** Exception message */
	exception_value: string;
	/** Stacktrace */
	stacktrace?: Stacktrace;
	/** Platform */
	platform: Platform;
	/** Release version */
	release?: string;
	/** Distribution variant */
	dist?: string;
	/** Environment (e.g., 'production', 'staging') */
	environment?: string;
	/** Exception level */
	level?: IssueLevel;
	/** Exception mechanism */
	mechanism?: Mechanism;
	/** Timestamp */
	timestamp?: string;
	/** User context */
	user?: UserContext;
	/** Device context */
	device?: DeviceContext;
	/** Browser context */
	browser?: BrowserContext;
	/** OS context */
	os?: OsContext;
	/** Request context */
	request?: RequestContext;
	/** Custom tags */
	tags?: Record<string, string>;
	/** Extra data */
	extra?: Record<string, unknown>;
	/** Active feature flags at crash time */
	active_flags?: Record<string, unknown>;
	/** Breadcrumbs leading up to the crash */
	breadcrumbs?: Breadcrumb[];
	/** Person ID from analytics integration */
	person_id?: string;
	/** Distinct ID from analytics integration */
	distinct_id?: string;
	/** SDK info */
	sdk?: {
		name: string;
		version: string;
	};
}

/**
 * Response from crash capture endpoint.
 */
export interface CaptureResponse {
	/** Event ID */
	event_id: string;
	/** Issue ID */
	issue_id?: string;
	/** Short ID for display */
	short_id?: string;
}

/**
 * Options for capturing an exception.
 */
export interface CaptureOptions {
	/** Custom tags for this event */
	tags?: Record<string, string>;
	/** Extra data for this event */
	extra?: Record<string, unknown>;
	/** Override the exception level */
	level?: IssueLevel;
	/** Custom mechanism */
	mechanism?: Mechanism;
	/** Custom fingerprint for grouping */
	fingerprint?: string[];
}

/**
 * Batch configuration for crash events.
 */
export interface BatchConfig {
	/** Flush interval in milliseconds (default: 5000) */
	flushIntervalMs: number;
	/** Maximum events per batch (default: 10) */
	maxBatchSize: number;
	/** Maximum queue size before dropping oldest (default: 100) */
	maxQueueSize: number;
}

/**
 * BeforeSend hook to filter or modify events before sending.
 */
export type BeforeSendHook = (event: CrashEvent) => CrashEvent | null | Promise<CrashEvent | null>;

/**
 * Session status for session tracking.
 */
export type SessionStatus = 'active' | 'exited' | 'crashed' | 'abnormal' | 'errored';

/**
 * Response from session start endpoint.
 */
export interface SessionStartResponse {
	/** Session ID assigned by the server */
	session_id: string;
	/** Whether this session is being sampled */
	sampled: boolean;
}

/**
 * Response from session end endpoint.
 */
export interface SessionEndResponse {
	/** Whether the session was ended successfully */
	success: boolean;
}

/**
 * Configuration for session tracking.
 */
export interface SessionConfig {
	/** Project ID for the session */
	projectId: string;
	/** Distinct ID for the user (usually from analytics) */
	distinctId: string;
	/** Person ID if known (usually from analytics) */
	personId?: string;
	/** Environment (e.g., 'production', 'staging') */
	environment: string;
	/** Release version */
	release?: string;
	/** Sample rate (0.0 - 1.0, default: 1.0) */
	sampleRate: number;
	/** Base URL for the session API */
	baseUrl?: string;
	/** Whether to use SDK endpoints (API key auth) instead of user auth endpoints */
	useSdkEndpoints?: boolean;
}

/**
 * Options for creating a CrashClient.
 */
export interface CrashClientOptions {
	/** Base URL of the Loom server */
	baseUrl: string;
	/** Project ID or slug */
	project: string;
	/** API key for authentication */
	apiKey?: string;
	/** Authorization token (alternative to apiKey) */
	authToken?: string;
	/** Release version (recommended) */
	release?: string;
	/** Distribution variant */
	dist?: string;
	/** Environment (default: 'production') */
	environment?: string;
	/** Maximum number of breadcrumbs to keep (default: 100) */
	maxBreadcrumbs?: number;
	/** Batch configuration */
	batch?: Partial<BatchConfig>;
	/** Request timeout in milliseconds (default: 5000) */
	timeoutMs?: number;
	/** Whether to log debug information (default: false) */
	debug?: boolean;
	/** Hook to filter or modify events before sending */
	beforeSend?: BeforeSendHook;
	/** Analytics client for identity integration */
	analytics?: {
		getDistinctId(): string;
		getPersonId?(): string | undefined;
	};
	/** Flags client for active flags integration */
	flags?: {
		getAllFlags(): Record<string, unknown>;
	};
	/** Whether to enable session tracking (default: false) */
	sessionTracking?: boolean;
	/** Session sample rate (0.0 - 1.0, default: 1.0) */
	sessionSampleRate?: number;
	/** Distinct ID for session tracking (defaults to random UUID or analytics.getDistinctId()) */
	sessionDistinctId?: string;
}

/**
 * Default batch configuration values.
 */
export const DEFAULT_BATCH_CONFIG: BatchConfig = {
	flushIntervalMs: 5000,
	maxBatchSize: 10,
	maxQueueSize: 100
};

/**
 * SDK version.
 */
export const SDK_VERSION = '0.1.0';

/**
 * SDK name.
 */
export const SDK_NAME = '@loom/crash';
