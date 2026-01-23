/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

/**
 * Base error class for all crash SDK errors.
 */
export class CrashError extends Error {
	constructor(message: string, options?: { cause?: unknown }) {
		super(message, { cause: options?.cause });
		this.name = 'CrashError';
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
export class ConfigurationError extends CrashError {
	constructor(message: string) {
		super(message);
		this.name = 'ConfigurationError';
	}
}

/**
 * Error thrown when the base URL is invalid.
 */
export class InvalidBaseUrlError extends CrashError {
	constructor(message = 'Invalid base URL') {
		super(message);
		this.name = 'InvalidBaseUrlError';
	}
}

/**
 * Error thrown when authentication fails.
 */
export class AuthenticationError extends CrashError {
	constructor(message = 'Authentication failed') {
		super(message);
		this.name = 'AuthenticationError';
	}
}

/**
 * Error thrown when rate limited.
 */
export class RateLimitedError extends CrashError {
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
export class ClientClosedError extends CrashError {
	constructor(message = 'Crash client has been closed') {
		super(message);
		this.name = 'ClientClosedError';
	}
}

/**
 * Error thrown when a capture operation fails.
 */
export class CaptureError extends CrashError {
	readonly exceptionType: string;

	constructor(exceptionType: string, message?: string, options?: { cause?: unknown }) {
		super(message ?? `Failed to capture crash: ${exceptionType}`, options);
		this.name = 'CaptureError';
		this.exceptionType = exceptionType;
	}

	isRetryable(): boolean {
		return true;
	}
}

/**
 * Error thrown when stack trace parsing fails.
 */
export class StackParseError extends CrashError {
	constructor(message: string, options?: { cause?: unknown }) {
		super(message, options);
		this.name = 'StackParseError';
	}
}

/**
 * Error thrown when a network operation fails.
 */
export class NetworkError extends CrashError {
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
export class ServerError extends CrashError {
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
