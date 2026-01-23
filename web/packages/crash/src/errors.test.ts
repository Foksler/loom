/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { describe, it, expect } from 'vitest';
import {
	CrashError,
	ConfigurationError,
	InvalidBaseUrlError,
	AuthenticationError,
	RateLimitedError,
	ClientClosedError,
	CaptureError,
	StackParseError,
	NetworkError,
	ServerError
} from './errors';

describe('CrashError', () => {
	it('should create a CrashError', () => {
		const error = new CrashError('test message');

		expect(error.name).toBe('CrashError');
		expect(error.message).toBe('test message');
		expect(error.isRetryable()).toBe(false);
	});

	it('should support cause', () => {
		const cause = new Error('original error');
		const error = new CrashError('wrapped error', { cause });

		expect(error.cause).toBe(cause);
	});
});

describe('ConfigurationError', () => {
	it('should create a ConfigurationError', () => {
		const error = new ConfigurationError('missing config');

		expect(error.name).toBe('ConfigurationError');
		expect(error.message).toBe('missing config');
		expect(error.isRetryable()).toBe(false);
	});
});

describe('InvalidBaseUrlError', () => {
	it('should create with default message', () => {
		const error = new InvalidBaseUrlError();

		expect(error.name).toBe('InvalidBaseUrlError');
		expect(error.message).toBe('Invalid base URL');
	});

	it('should create with custom message', () => {
		const error = new InvalidBaseUrlError('Custom message');

		expect(error.message).toBe('Custom message');
	});
});

describe('AuthenticationError', () => {
	it('should create an AuthenticationError', () => {
		const error = new AuthenticationError();

		expect(error.name).toBe('AuthenticationError');
		expect(error.message).toBe('Authentication failed');
	});
});

describe('RateLimitedError', () => {
	it('should create a RateLimitedError', () => {
		const error = new RateLimitedError('Rate limited', 60);

		expect(error.name).toBe('RateLimitedError');
		expect(error.retryAfterSecs).toBe(60);
		expect(error.isRetryable()).toBe(true);
	});

	it('should work without retryAfterSecs', () => {
		const error = new RateLimitedError();

		expect(error.retryAfterSecs).toBeUndefined();
	});
});

describe('ClientClosedError', () => {
	it('should create a ClientClosedError', () => {
		const error = new ClientClosedError();

		expect(error.name).toBe('ClientClosedError');
		expect(error.message).toBe('Crash client has been closed');
	});
});

describe('CaptureError', () => {
	it('should create a CaptureError', () => {
		const error = new CaptureError('TypeError');

		expect(error.name).toBe('CaptureError');
		expect(error.exceptionType).toBe('TypeError');
		expect(error.message).toBe('Failed to capture crash: TypeError');
		expect(error.isRetryable()).toBe(true);
	});

	it('should support custom message', () => {
		const error = new CaptureError('ReferenceError', 'Custom capture failure');

		expect(error.message).toBe('Custom capture failure');
	});
});

describe('StackParseError', () => {
	it('should create a StackParseError', () => {
		const error = new StackParseError('Invalid stack format');

		expect(error.name).toBe('StackParseError');
		expect(error.message).toBe('Invalid stack format');
	});
});

describe('NetworkError', () => {
	it('should create a NetworkError', () => {
		const error = new NetworkError();

		expect(error.name).toBe('NetworkError');
		expect(error.message).toBe('Network request failed');
		expect(error.isRetryable()).toBe(true);
	});
});

describe('ServerError', () => {
	it('should create a ServerError', () => {
		const error = new ServerError(500);

		expect(error.name).toBe('ServerError');
		expect(error.statusCode).toBe(500);
		expect(error.message).toBe('Server error: 500');
	});

	it('should be retryable for 5xx errors', () => {
		expect(new ServerError(500).isRetryable()).toBe(true);
		expect(new ServerError(502).isRetryable()).toBe(true);
		expect(new ServerError(503).isRetryable()).toBe(true);
	});

	it('should not be retryable for 4xx errors', () => {
		expect(new ServerError(400).isRetryable()).toBe(false);
		expect(new ServerError(404).isRetryable()).toBe(false);
		expect(new ServerError(429).isRetryable()).toBe(false);
	});
});
