/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

/**
 * Check-in status values.
 */
export type CheckInStatus = 'in_progress' | 'ok' | 'error' | 'missed' | 'timeout';

/**
 * Check-in source values.
 */
export type CheckInSource = 'ping' | 'sdk' | 'manual' | 'system';

/**
 * Monitor health status values.
 */
export type MonitorHealth = 'healthy' | 'failing' | 'missed' | 'timeout' | 'unknown';

/**
 * Monitor status values.
 */
export type MonitorStatus = 'active' | 'paused' | 'disabled';

/**
 * Monitor schedule types.
 */
export type MonitorSchedule =
	| { type: 'cron'; expression: string }
	| { type: 'interval'; minutes: number };

/**
 * A cron monitor definition.
 */
export interface Monitor {
	/** Monitor ID */
	id: string;
	/** Organization ID */
	org_id: string;
	/** Optional project ID */
	project_id?: string;
	/** URL-safe slug identifier */
	slug: string;
	/** Human-readable name */
	name: string;
	/** Optional description */
	description?: string;
	/** Current status */
	status: MonitorStatus;
	/** Current health */
	health: MonitorHealth;
	/** Schedule configuration */
	schedule: MonitorSchedule;
	/** IANA timezone */
	timezone: string;
	/** Grace period before marking missed (minutes) */
	checkin_margin_minutes: number;
	/** Max runtime before timeout alert (minutes) */
	max_runtime_minutes?: number;
	/** Ping key for simple integration */
	ping_key: string;
	/** Environment filter */
	environments: string[];
	/** Last check-in timestamp */
	last_checkin_at?: string;
	/** Last check-in status */
	last_checkin_status?: CheckInStatus;
	/** Next expected check-in */
	next_expected_at?: string;
	/** Consecutive failure count */
	consecutive_failures: number;
	/** Total check-in count */
	total_checkins: number;
	/** Total failure count */
	total_failures: number;
	/** Created timestamp */
	created_at: string;
	/** Updated timestamp */
	updated_at: string;
}

/**
 * A check-in record.
 */
export interface CheckIn {
	/** Check-in ID */
	id: string;
	/** Monitor ID */
	monitor_id: string;
	/** Check-in status */
	status: CheckInStatus;
	/** Job start timestamp */
	started_at?: string;
	/** Job finish timestamp */
	finished_at: string;
	/** Job duration in milliseconds */
	duration_ms?: number;
	/** Environment */
	environment?: string;
	/** Release version */
	release?: string;
	/** Exit code for failed jobs */
	exit_code?: number;
	/** Truncated stdout/stderr (max 10KB) */
	output?: string;
	/** Link to crash event */
	crash_event_id?: string;
	/** Check-in source */
	source: CheckInSource;
	/** Created timestamp */
	created_at: string;
}

/**
 * Response from creating a check-in.
 */
export interface CheckInResponse {
	/** Check-in ID */
	id: string;
}

/**
 * Request to create a check-in.
 */
export interface CheckInRequest {
	/** Organization ID */
	org_id: string;
	/** Check-in status */
	status: CheckInStatus;
	/** Job start timestamp */
	started_at?: string;
	/** Job finish timestamp */
	finished_at?: string;
	/** Job duration in milliseconds */
	duration_ms?: number;
	/** Environment */
	environment?: string;
	/** Release version */
	release?: string;
	/** Exit code for failed jobs */
	exit_code?: number;
	/** Truncated stdout/stderr (max 10KB) */
	output?: string;
	/** Link to crash event */
	crash_event_id?: string;
}

/**
 * Request to update a check-in.
 */
export interface CheckInUpdateRequest {
	/** Check-in status */
	status: CheckInStatus;
	/** Job finish timestamp */
	finished_at?: string;
	/** Job duration in milliseconds */
	duration_ms?: number;
	/** Exit code for failed jobs */
	exit_code?: number;
	/** Truncated stdout/stderr (max 10KB) */
	output?: string;
	/** Link to crash event */
	crash_event_id?: string;
}

/**
 * Details for a successful check-in completion.
 */
export interface CheckInOk {
	/** Job duration in milliseconds */
	durationMs?: number;
	/** Optional output message */
	output?: string;
}

/**
 * Details for a failed check-in completion.
 */
export interface CheckInError {
	/** Job duration in milliseconds */
	durationMs?: number;
	/** Exit code */
	exitCode?: number;
	/** Error output */
	output?: string;
	/** Linked crash event ID */
	crashEventId?: string;
}

/**
 * Options for creating a CronsClient.
 */
export interface CronsClientOptions {
	/** Base URL of the Loom server */
	baseUrl: string;
	/** Organization ID */
	orgId: string;
	/** API key for authentication */
	apiKey?: string;
	/** Authorization token (alternative to apiKey) */
	authToken?: string;
	/** Environment (e.g., 'production', 'staging') */
	environment?: string;
	/** Release version */
	release?: string;
	/** Request timeout in milliseconds (default: 30000) */
	timeoutMs?: number;
	/** Whether to log debug information (default: false) */
	debug?: boolean;
	/** Optional crash client for error linking */
	crashClient?: CrashClientInterface;
}

/**
 * Interface for crash client integration.
 */
export interface CrashClientInterface {
	captureException(error: unknown, options?: { tags?: Record<string, string> }): string;
}

/**
 * SDK version.
 */
export const SDK_VERSION = '0.1.0';

/**
 * SDK name.
 */
export const SDK_NAME = '@loom/crons';
