/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

/**
 * Base error class for all crons SDK errors.
 */
export class CronsError extends Error {
	constructor(message: string, options?: { cause?: unknown }) {
		super(message, { cause: options?.cause });
		this.name = 'CronsError';
	}

	/**
	 * Returns true if the error is retryable.
	 */
	isRetryable(): boolean {
		return false;
	}
}

/**
 * Error thrown when required configuration is missing.
 */
export class ConfigurationError extends CronsError {
	constructor(message: string) {
		super(message);
		this.name = 'ConfigurationError';
	}
}

/**
 * Error thrown when the base URL is invalid.
 */
export class InvalidBaseUrlError extends CronsError {
	constructor(message = 'Invalid base URL') {
		super(message);
		this.name = 'InvalidBaseUrlError';
	}
}

/**
 * Error thrown when authentication fails.
 */
export class AuthenticationError extends CronsError {
	constructor(message = 'Authentication failed') {
		super(message);
		this.name = 'AuthenticationError';
	}
}

/**
 * Error thrown when rate limited.
 */
export class RateLimitedError extends CronsError {
	readonly retryAfterSecs?: number;

	constructor(message = 'Rate limited', retryAfterSecs?: number) {
		super(message);
		this.name = 'RateLimitedError';
		this.retryAfterSecs = retryAfterSecs;
	}

	isRetryable(): boolean {
		return true;
	}
}

/**
 * Error thrown when the client is closed.
 */
export class ClientClosedError extends CronsError {
	constructor(message = 'Crons client has been closed') {
		super(message);
		this.name = 'ClientClosedError';
	}
}

/**
 * Error thrown when a check-in operation fails.
 */
export class CheckInError extends CronsError {
	readonly monitorSlug: string;

	constructor(monitorSlug: string, message?: string, options?: { cause?: unknown }) {
		super(message ?? `Check-in failed for monitor: ${monitorSlug}`, options);
		this.name = 'CheckInError';
		this.monitorSlug = monitorSlug;
	}

	isRetryable(): boolean {
		return true;
	}
}

/**
 * Error thrown when a monitor is not found.
 */
export class MonitorNotFoundError extends CronsError {
	readonly monitorSlug: string;

	constructor(monitorSlug: string, message?: string) {
		super(message ?? `Monitor not found: ${monitorSlug}`);
		this.name = 'MonitorNotFoundError';
		this.monitorSlug = monitorSlug;
	}
}

/**
 * Error thrown when a network operation fails.
 */
export class NetworkError extends CronsError {
	constructor(message = 'Network request failed', options?: { cause?: unknown }) {
		super(message, options);
		this.name = 'NetworkError';
	}

	isRetryable(): boolean {
		return true;
	}
}

/**
 * Error thrown when a server returns an error.
 */
export class ServerError extends CronsError {
	readonly statusCode: number;

	constructor(statusCode: number, message?: string, options?: { cause?: unknown }) {
		super(message ?? `Server error: ${statusCode}`, options);
		this.name = 'ServerError';
		this.statusCode = statusCode;
	}

	isRetryable(): boolean {
		return this.statusCode >= 500 && this.statusCode < 600;
	}
}

/**
 * Error thrown when a job execution fails within withMonitor.
 */
export class JobFailedError extends CronsError {
	readonly monitorSlug: string;
	readonly originalError: unknown;

	constructor(monitorSlug: string, originalError: unknown) {
		const message =
			originalError instanceof Error ? originalError.message : String(originalError);
		super(`Job failed for monitor ${monitorSlug}: ${message}`, { cause: originalError });
		this.name = 'JobFailedError';
		this.monitorSlug = monitorSlug;
		this.originalError = originalError;
	}
}
