/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { describe, it, expect } from 'vitest';
import {
	CronsError,
	ConfigurationError,
	InvalidBaseUrlError,
	AuthenticationError,
	RateLimitedError,
	ClientClosedError,
	CheckInError,
	MonitorNotFoundError,
	NetworkError,
	ServerError,
	JobFailedError
} from './errors';

describe('CronsError', () => {
	it('should create with message', () => {
		const error = new CronsError('test error');
		expect(error.message).toBe('test error');
		expect(error.name).toBe('CronsError');
	});

	it('should create with cause', () => {
		const cause = new Error('original error');
		const error = new CronsError('test error', { cause });
		expect(error.cause).toBe(cause);
	});

	it('should not be retryable by default', () => {
		const error = new CronsError('test error');
		expect(error.isRetryable()).toBe(false);
	});
});

describe('ConfigurationError', () => {
	it('should create with message', () => {
		const error = new ConfigurationError('missing field');
		expect(error.message).toBe('missing field');
		expect(error.name).toBe('ConfigurationError');
	});

	it('should be instance of CronsError', () => {
		const error = new ConfigurationError('missing field');
		expect(error).toBeInstanceOf(CronsError);
	});
});

describe('InvalidBaseUrlError', () => {
	it('should create with default message', () => {
		const error = new InvalidBaseUrlError();
		expect(error.message).toBe('Invalid base URL');
		expect(error.name).toBe('InvalidBaseUrlError');
	});

	it('should create with custom message', () => {
		const error = new InvalidBaseUrlError('Bad URL format');
		expect(error.message).toBe('Bad URL format');
	});
});

describe('AuthenticationError', () => {
	it('should create with default message', () => {
		const error = new AuthenticationError();
		expect(error.message).toBe('Authentication failed');
		expect(error.name).toBe('AuthenticationError');
	});

	it('should create with custom message', () => {
		const error = new AuthenticationError('Invalid token');
		expect(error.message).toBe('Invalid token');
	});
});

describe('RateLimitedError', () => {
	it('should create with default message', () => {
		const error = new RateLimitedError();
		expect(error.message).toBe('Rate limited');
		expect(error.name).toBe('RateLimitedError');
	});

	it('should create with retry after', () => {
		const error = new RateLimitedError('Rate limited', 60);
		expect(error.retryAfterSecs).toBe(60);
	});

	it('should be retryable', () => {
		const error = new RateLimitedError();
		expect(error.isRetryable()).toBe(true);
	});
});

describe('ClientClosedError', () => {
	it('should create with default message', () => {
		const error = new ClientClosedError();
		expect(error.message).toBe('Crons client has been closed');
		expect(error.name).toBe('ClientClosedError');
	});

	it('should create with custom message', () => {
		const error = new ClientClosedError('Cannot make requests after close');
		expect(error.message).toBe('Cannot make requests after close');
	});
});

describe('CheckInError', () => {
	it('should create with monitor slug', () => {
		const error = new CheckInError('daily-cleanup');
		expect(error.message).toBe('Check-in failed for monitor: daily-cleanup');
		expect(error.name).toBe('CheckInError');
		expect(error.monitorSlug).toBe('daily-cleanup');
	});

	it('should create with custom message', () => {
		const error = new CheckInError('daily-cleanup', 'Network timeout');
		expect(error.message).toBe('Network timeout');
		expect(error.monitorSlug).toBe('daily-cleanup');
	});

	it('should create with cause', () => {
		const cause = new Error('original');
		const error = new CheckInError('daily-cleanup', 'Failed', { cause });
		expect(error.cause).toBe(cause);
	});

	it('should be retryable', () => {
		const error = new CheckInError('daily-cleanup');
		expect(error.isRetryable()).toBe(true);
	});
});

describe('MonitorNotFoundError', () => {
	it('should create with monitor slug', () => {
		const error = new MonitorNotFoundError('unknown-monitor');
		expect(error.message).toBe('Monitor not found: unknown-monitor');
		expect(error.name).toBe('MonitorNotFoundError');
		expect(error.monitorSlug).toBe('unknown-monitor');
	});

	it('should create with custom message', () => {
		const error = new MonitorNotFoundError('unknown-monitor', 'Custom message');
		expect(error.message).toBe('Custom message');
	});
});

describe('NetworkError', () => {
	it('should create with default message', () => {
		const error = new NetworkError();
		expect(error.message).toBe('Network request failed');
		expect(error.name).toBe('NetworkError');
	});

	it('should create with cause', () => {
		const cause = new Error('DNS failed');
		const error = new NetworkError('Connection failed', { cause });
		expect(error.cause).toBe(cause);
	});

	it('should be retryable', () => {
		const error = new NetworkError();
		expect(error.isRetryable()).toBe(true);
	});
});

describe('ServerError', () => {
	it('should create with status code', () => {
		const error = new ServerError(500);
		expect(error.message).toBe('Server error: 500');
		expect(error.name).toBe('ServerError');
		expect(error.statusCode).toBe(500);
	});

	it('should create with custom message', () => {
		const error = new ServerError(503, 'Service unavailable');
		expect(error.message).toBe('Service unavailable');
	});

	it('should be retryable for 5xx errors', () => {
		expect(new ServerError(500).isRetryable()).toBe(true);
		expect(new ServerError(502).isRetryable()).toBe(true);
		expect(new ServerError(503).isRetryable()).toBe(true);
	});

	it('should not be retryable for 4xx errors', () => {
		expect(new ServerError(400).isRetryable()).toBe(false);
		expect(new ServerError(401).isRetryable()).toBe(false);
		expect(new ServerError(404).isRetryable()).toBe(false);
	});
});

describe('JobFailedError', () => {
	it('should create with error', () => {
		const originalError = new Error('Job crashed');
		const error = new JobFailedError('daily-cleanup', originalError);
		expect(error.message).toBe('Job failed for monitor daily-cleanup: Job crashed');
		expect(error.name).toBe('JobFailedError');
		expect(error.monitorSlug).toBe('daily-cleanup');
		expect(error.originalError).toBe(originalError);
		expect(error.cause).toBe(originalError);
	});

	it('should handle non-Error values', () => {
		const error = new JobFailedError('daily-cleanup', 'string error');
		expect(error.message).toBe('Job failed for monitor daily-cleanup: string error');
		expect(error.originalError).toBe('string error');
	});

	it('should handle null values', () => {
		const error = new JobFailedError('daily-cleanup', null);
		expect(error.message).toBe('Job failed for monitor daily-cleanup: null');
	});
});
