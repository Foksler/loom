/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { SessionTracker } from './session';
import type { SessionConfig } from './types';

// Mock HttpClient
const createMockHttpClient = () => ({
	postJson: vi.fn().mockResolvedValue({ session_id: 'test-session-id', sampled: true }),
	post: vi.fn().mockResolvedValue({ ok: true })
});

describe('SessionTracker', () => {
	let mockHttpClient: ReturnType<typeof createMockHttpClient>;
	let defaultConfig: SessionConfig;

	beforeEach(() => {
		mockHttpClient = createMockHttpClient();
		defaultConfig = {
			projectId: 'test-project',
			distinctId: 'user-123',
			environment: 'production',
			release: '1.0.0',
			sampleRate: 1.0,
			baseUrl: 'https://loom.example.com'
		};

		// Mock crypto.randomUUID for consistent session IDs in tests
		vi.spyOn(crypto, 'randomUUID').mockReturnValue('test-uuid-1234');
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('constructor', () => {
		it('should generate a session ID', () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			expect(tracker.getSessionId()).toBe('test-uuid-1234');
		});

		it('should initialize with zero error and crash counts', () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			expect(tracker.getErrorCount()).toBe(0);
			expect(tracker.getCrashCount()).toBe(0);
		});

		it('should not be ended initially', () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			expect(tracker.isEnded()).toBe(false);
		});
	});

	describe('sampling', () => {
		it('should sample when sampleRate is 1.0', () => {
			const tracker = new SessionTracker(mockHttpClient as any, {
				...defaultConfig,
				sampleRate: 1.0
			});
			expect(tracker.isSampled()).toBe(true);
		});

		it('should not sample when sampleRate is 0.0', () => {
			const tracker = new SessionTracker(mockHttpClient as any, {
				...defaultConfig,
				sampleRate: 0.0
			});
			expect(tracker.isSampled()).toBe(false);
		});

		it('should be deterministic based on session ID', () => {
			// Same session ID should always give same sampling result
			const tracker1 = new SessionTracker(mockHttpClient as any, {
				...defaultConfig,
				sampleRate: 0.5
			});
			const tracker2 = new SessionTracker(mockHttpClient as any, {
				...defaultConfig,
				sampleRate: 0.5
			});
			expect(tracker1.isSampled()).toBe(tracker2.isSampled());
		});
	});

	describe('start', () => {
		it('should send start request when sampled', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await tracker.start();

			expect(mockHttpClient.postJson).toHaveBeenCalledWith(
				'/api/sessions/start',
				expect.objectContaining({
					project_id: 'test-project',
					distinct_id: 'user-123',
					person_id: undefined,
					platform: expect.stringMatching(/^(javascript|node)$/), // Platform depends on test environment
					environment: 'production',
					release: '1.0.0',
					sample_rate: 1.0
				})
			);
		});

		it('should not send start request when not sampled', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, {
				...defaultConfig,
				sampleRate: 0.0
			});
			await tracker.start();

			expect(mockHttpClient.postJson).not.toHaveBeenCalled();
		});

		it('should not throw if start request fails', async () => {
			mockHttpClient.postJson.mockRejectedValueOnce(new Error('Network error'));

			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await expect(tracker.start()).resolves.not.toThrow();
		});
	});

	describe('recordError', () => {
		it('should increment error count', () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			expect(tracker.getErrorCount()).toBe(0);

			tracker.recordError();
			expect(tracker.getErrorCount()).toBe(1);

			tracker.recordError();
			expect(tracker.getErrorCount()).toBe(2);
		});
	});

	describe('recordCrash', () => {
		it('should increment crash count', () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			expect(tracker.getCrashCount()).toBe(0);

			tracker.recordCrash();
			expect(tracker.getCrashCount()).toBe(1);

			tracker.recordCrash();
			expect(tracker.getCrashCount()).toBe(2);
		});
	});

	describe('endAsync', () => {
		it('should send end request when sampled', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await tracker.start();
			await tracker.endAsync();

			expect(mockHttpClient.post).toHaveBeenCalledWith('/api/sessions/end', {
				project_id: 'test-project',
				session_id: 'test-uuid-1234',
				status: 'exited',
				error_count: 0,
				crash_count: 0,
				duration_ms: expect.any(Number)
			});
		});

		it('should set status to crashed when crash_count > 0', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await tracker.start();
			tracker.recordCrash();
			await tracker.endAsync();

			expect(mockHttpClient.post).toHaveBeenCalledWith(
				'/api/sessions/end',
				expect.objectContaining({
					status: 'crashed',
					crash_count: 1
				})
			);
		});

		it('should set status to errored when error_count > 0 but crash_count is 0', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await tracker.start();
			tracker.recordError();
			await tracker.endAsync();

			expect(mockHttpClient.post).toHaveBeenCalledWith(
				'/api/sessions/end',
				expect.objectContaining({
					status: 'errored',
					error_count: 1,
					crash_count: 0
				})
			);
		});

		it('should not send end request when not sampled and not crashed', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, {
				...defaultConfig,
				sampleRate: 0.0
			});
			await tracker.start();
			await tracker.endAsync();

			expect(mockHttpClient.post).not.toHaveBeenCalled();
		});

		it('should send end request for crashed sessions even when not sampled', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, {
				...defaultConfig,
				sampleRate: 0.0
			});
			await tracker.start();
			tracker.recordCrash();
			await tracker.endAsync();

			expect(mockHttpClient.post).toHaveBeenCalledWith(
				'/api/sessions/end',
				expect.objectContaining({
					status: 'crashed'
				})
			);
		});

		it('should only end once', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await tracker.start();
			await tracker.endAsync();
			await tracker.endAsync();
			await tracker.endAsync();

			// post should only be called once for the end request
			expect(mockHttpClient.post).toHaveBeenCalledTimes(1);
		});

		it('should set isEnded to true', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await tracker.start();
			expect(tracker.isEnded()).toBe(false);
			await tracker.endAsync();
			expect(tracker.isEnded()).toBe(true);
		});
	});

	describe('end (synchronous)', () => {
		it('should mark session as ended', () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			expect(tracker.isEnded()).toBe(false);
			tracker.end();
			expect(tracker.isEnded()).toBe(true);
		});

		it('should only end once', () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			tracker.end();
			tracker.end();
			tracker.end();
			// The end method uses sendBeacon in browser or async post in Node
			// In Node.js tests, post will be called
			expect(mockHttpClient.post).toHaveBeenCalledTimes(1);
		});
	});

	describe('status determination', () => {
		it('should return exited when no errors or crashes', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await tracker.start();
			await tracker.endAsync();

			expect(mockHttpClient.post).toHaveBeenCalledWith(
				'/api/sessions/end',
				expect.objectContaining({ status: 'exited' })
			);
		});

		it('should return errored when only errors', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await tracker.start();
			tracker.recordError();
			tracker.recordError();
			await tracker.endAsync();

			expect(mockHttpClient.post).toHaveBeenCalledWith(
				'/api/sessions/end',
				expect.objectContaining({ status: 'errored', error_count: 2 })
			);
		});

		it('should return crashed when crashes present (even with errors)', async () => {
			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await tracker.start();
			tracker.recordError();
			tracker.recordCrash();
			await tracker.endAsync();

			expect(mockHttpClient.post).toHaveBeenCalledWith(
				'/api/sessions/end',
				expect.objectContaining({ status: 'crashed', error_count: 1, crash_count: 1 })
			);
		});
	});

	describe('duration calculation', () => {
		it('should calculate duration in milliseconds', async () => {
			vi.useFakeTimers();

			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig);
			await tracker.start();

			// Advance time by 5 seconds
			vi.advanceTimersByTime(5000);

			await tracker.endAsync();

			expect(mockHttpClient.post).toHaveBeenCalledWith(
				'/api/sessions/end',
				expect.objectContaining({
					duration_ms: expect.any(Number)
				})
			);

			const callArgs = mockHttpClient.post.mock.calls[0][1] as { duration_ms: number };
			expect(callArgs.duration_ms).toBeGreaterThanOrEqual(5000);

			vi.useRealTimers();
		});
	});

	describe('debug mode', () => {
		it('should log debug messages when debug is enabled', async () => {
			const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

			const tracker = new SessionTracker(mockHttpClient as any, defaultConfig, { debug: true });
			await tracker.start();

			expect(consoleSpy).toHaveBeenCalled();

			consoleSpy.mockRestore();
		});
	});
});

describe('SessionTracker with CrashClient integration', () => {
	it('should track errors from captureException', async () => {
		// This test verifies the integration works correctly
		// The actual integration is tested in client.test.ts
		const mockHttpClient = createMockHttpClient();
		const tracker = new SessionTracker(mockHttpClient as any, {
			projectId: 'test-project',
			distinctId: 'user-123',
			environment: 'production',
			sampleRate: 1.0
		});

		await tracker.start();

		// Simulate what CrashClient does
		tracker.recordError();
		tracker.recordError();

		expect(tracker.getErrorCount()).toBe(2);
	});

	it('should track crashes from unhandled errors', async () => {
		const mockHttpClient = createMockHttpClient();
		const tracker = new SessionTracker(mockHttpClient as any, {
			projectId: 'test-project',
			distinctId: 'user-123',
			environment: 'production',
			sampleRate: 1.0
		});

		await tracker.start();

		// Simulate what CrashClient does for unhandled errors
		tracker.recordCrash();

		expect(tracker.getCrashCount()).toBe(1);
		expect(tracker.isSampled()).toBe(true);
	});
});
