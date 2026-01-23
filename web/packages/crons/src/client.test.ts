/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ConfigurationError, InvalidBaseUrlError, ClientClosedError, JobFailedError } from './errors';

// Mock HttpClient class
const mockPostJson = vi.fn();
const mockPatch = vi.fn();

vi.mock('@loom/http', () => {
	return {
		HttpClient: class MockHttpClient {
			constructor() {}
			postJson = mockPostJson;
			patch = mockPatch;
		}
	};
});

// Import after mock is set up
import { CronsClient } from './client';

describe('CronsClient', () => {
	beforeEach(() => {
		mockPostJson.mockReset().mockResolvedValue({ id: 'checkin-123' });
		mockPatch.mockReset().mockResolvedValue({ ok: true });
	});

	describe('constructor', () => {
		it('should create with required options', () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});
			expect(client).toBeInstanceOf(CronsClient);
		});

		it('should create with authToken', () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123',
				authToken: 'lt_xxx'
			});
			expect(client).toBeInstanceOf(CronsClient);
		});

		it('should create with apiKey', () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123',
				apiKey: 'api_xxx'
			});
			expect(client).toBeInstanceOf(CronsClient);
		});

		it('should create with all options', () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123',
				authToken: 'lt_xxx',
				environment: 'production',
				release: '1.0.0',
				timeoutMs: 5000,
				debug: true
			});
			expect(client).toBeInstanceOf(CronsClient);
		});

		it('should throw on missing baseUrl', () => {
			expect(() => {
				new CronsClient({
					baseUrl: '',
					orgId: 'org-123'
				});
			}).toThrow(InvalidBaseUrlError);
		});

		it('should throw on invalid baseUrl', () => {
			expect(() => {
				new CronsClient({
					baseUrl: 'not-a-url',
					orgId: 'org-123'
				});
			}).toThrow(InvalidBaseUrlError);
		});

		it('should throw on missing orgId', () => {
			expect(() => {
				new CronsClient({
					baseUrl: 'https://example.com',
					orgId: ''
				});
			}).toThrow(ConfigurationError);
		});
	});

	describe('isClosed_', () => {
		it('should return false initially', () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});
			expect(client.isClosed_()).toBe(false);
		});

		it('should return true after close', () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});
			client.close();
			expect(client.isClosed_()).toBe(true);
		});
	});

	describe('close', () => {
		it('should close the client', () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});
			client.close();
			expect(client.isClosed_()).toBe(true);
		});

		it('should be idempotent', () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});
			client.close();
			client.close();
			expect(client.isClosed_()).toBe(true);
		});
	});

	describe('checkinStart', () => {
		it('should throw when client is closed', async () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});
			client.close();

			await expect(client.checkinStart('my-job')).rejects.toThrow(ClientClosedError);
		});

		it('should return checkin id', async () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});

			const id = await client.checkinStart('my-job');
			expect(id).toBe('checkin-123');
		});

		it('should call postJson with correct URL and org_id in body', async () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});

			await client.checkinStart('my-job');

			expect(mockPostJson).toHaveBeenCalledWith(
				'/api/crons/monitors/my-job/checkins',
				expect.objectContaining({
					org_id: 'org-123',
					status: 'in_progress'
				})
			);
		});
	});

	describe('checkinOk', () => {
		it('should throw when client is closed', async () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});
			client.close();

			await expect(client.checkinOk('checkin-123')).rejects.toThrow(ClientClosedError);
		});

		it('should call patch with correct URL and body', async () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});

			await client.checkinOk('checkin-123', { durationMs: 1500, output: 'Done' });

			expect(mockPatch).toHaveBeenCalledWith(
				'/api/crons/checkins/checkin-123',
				expect.objectContaining({
					status: 'ok',
					duration_ms: 1500,
					output: 'Done'
				})
			);
		});
	});

	describe('checkinError', () => {
		it('should throw when client is closed', async () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});
			client.close();

			await expect(client.checkinError('checkin-123')).rejects.toThrow(ClientClosedError);
		});

		it('should call patch with correct URL and body', async () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});

			await client.checkinError('checkin-123', {
				durationMs: 5000,
				exitCode: 1,
				output: 'Failed',
				crashEventId: 'crash-456'
			});

			expect(mockPatch).toHaveBeenCalledWith(
				'/api/crons/checkins/checkin-123',
				expect.objectContaining({
					status: 'error',
					duration_ms: 5000,
					exit_code: 1,
					output: 'Failed',
					crash_event_id: 'crash-456'
				})
			);
		});
	});

	describe('withMonitor', () => {
		it('should throw when client is closed', async () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});
			client.close();

			await expect(
				client.withMonitor('my-job', async () => {
					return 'result';
				})
			).rejects.toThrow(ClientClosedError);
		});

		it('should complete successfully and return result', async () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});

			const result = await client.withMonitor('my-job', async () => {
				return 'success-result';
			});

			expect(result).toBe('success-result');
			expect(mockPostJson).toHaveBeenCalledTimes(1);
			expect(mockPatch).toHaveBeenCalledTimes(1);
			expect(mockPatch).toHaveBeenCalledWith(
				expect.stringContaining('/api/crons/checkins/'),
				expect.objectContaining({ status: 'ok' })
			);
		});

		it('should complete with error when job fails', async () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123'
			});

			await expect(
				client.withMonitor('my-job', async () => {
					throw new Error('Job crashed');
				})
			).rejects.toThrow(JobFailedError);

			expect(mockPatch).toHaveBeenCalledWith(
				expect.stringContaining('/api/crons/checkins/'),
				expect.objectContaining({ status: 'error', exit_code: 1 })
			);
		});
	});

	describe('debug mode', () => {
		let consoleSpy: ReturnType<typeof vi.spyOn>;

		beforeEach(() => {
			consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
		});

		afterEach(() => {
			consoleSpy.mockRestore();
		});

		it('should log when debug is true', () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123',
				debug: true
			});
			client.close();
			expect(consoleSpy).toHaveBeenCalledWith('[Crons] Client closed');
		});

		it('should not log when debug is false', () => {
			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123',
				debug: false
			});
			client.close();
			expect(consoleSpy).not.toHaveBeenCalled();
		});
	});
});

describe('CronsClient integration', () => {
	describe('crash client integration', () => {
		it('should accept crash client in options', () => {
			const mockCrashClient = {
				captureException: vi.fn().mockReturnValue('event-123')
			};

			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123',
				crashClient: mockCrashClient
			});

			expect(client).toBeInstanceOf(CronsClient);
		});

		it('should capture crash when job fails with crash client', async () => {
			const mockCrashClient = {
				captureException: vi.fn().mockReturnValue('crash-event-123')
			};

			const client = new CronsClient({
				baseUrl: 'https://example.com',
				orgId: 'org-123',
				crashClient: mockCrashClient
			});

			await expect(
				client.withMonitor('my-job', async () => {
					throw new Error('Job crashed');
				})
			).rejects.toThrow();

			expect(mockCrashClient.captureException).toHaveBeenCalled();
			expect(mockPatch).toHaveBeenCalledWith(
				expect.stringContaining('/api/crons/checkins/'),
				expect.objectContaining({ crash_event_id: 'crash-event-123' })
			);
		});
	});
});

describe('JobFailedError', () => {
	it('should contain original error', () => {
		const originalError = new Error('Database connection failed');
		const error = new JobFailedError('db-backup', originalError);

		expect(error.monitorSlug).toBe('db-backup');
		expect(error.originalError).toBe(originalError);
		expect(error.message).toContain('Database connection failed');
	});
});
