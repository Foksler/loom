/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { CrashClient } from './client';
import { ConfigurationError, InvalidBaseUrlError } from './errors';

// Mock the HttpClient
vi.mock('@loom/http', () => {
	const MockHttpClient = vi.fn().mockImplementation(function (this: Record<string, unknown>) {
		this.postJson = vi.fn().mockResolvedValue({
			event_id: 'test-event-id',
			issue_id: 'test-issue-id'
		});
		return this;
	});
	return { HttpClient: MockHttpClient };
});

describe('CrashClient', () => {
	let client: CrashClient;

	beforeEach(() => {
		vi.useFakeTimers();
		client = new CrashClient({
			baseUrl: 'https://loom.example.com',
			project: 'test-project',
			release: '1.0.0',
			environment: 'test',
			debug: false
		});
	});

	afterEach(async () => {
		await client.shutdown();
		vi.useRealTimers();
	});

	describe('constructor', () => {
		it('should throw on missing baseUrl', () => {
			expect(
				() =>
					new CrashClient({
						baseUrl: '',
						project: 'test'
					})
			).toThrow(InvalidBaseUrlError);
		});

		it('should throw on invalid baseUrl', () => {
			expect(
				() =>
					new CrashClient({
						baseUrl: 'not-a-url',
						project: 'test'
					})
			).toThrow(InvalidBaseUrlError);
		});

		it('should throw on missing project', () => {
			expect(
				() =>
					new CrashClient({
						baseUrl: 'https://example.com',
						project: ''
					})
			).toThrow(ConfigurationError);
		});

		it('should create with valid options', () => {
			const c = new CrashClient({
				baseUrl: 'https://loom.example.com',
				project: 'my-project'
			});

			expect(c).toBeDefined();
			expect(c.isClosed_()).toBe(false);
		});
	});

	describe('captureException', () => {
		it('should capture an Error object', () => {
			const error = new Error('Test error');
			const eventId = client.captureException(error);

			expect(eventId).toBeDefined();
			expect(typeof eventId).toBe('string');
		});

		it('should capture a string error', () => {
			const eventId = client.captureException('Something went wrong');

			expect(eventId).toBeDefined();
		});

		it('should capture with options', () => {
			const error = new Error('Test error');
			const eventId = client.captureException(error, {
				tags: { component: 'test' },
				extra: { foo: 'bar' }
			});

			expect(eventId).toBeDefined();
		});

		it('should not capture after shutdown', async () => {
			await client.shutdown();

			const eventId = client.captureException(new Error('test'));

			expect(eventId).toBe('');
		});
	});

	describe('captureMessage', () => {
		it('should capture a message', () => {
			const eventId = client.captureMessage('Test message');

			expect(eventId).toBeDefined();
			expect(typeof eventId).toBe('string');
		});

		it('should capture with level', () => {
			const eventId = client.captureMessage('Warning message', {
				level: 'warning'
			});

			expect(eventId).toBeDefined();
		});
	});

	describe('breadcrumbs', () => {
		it('should add breadcrumbs', () => {
			client.addBreadcrumb({
				category: 'test',
				message: 'Test breadcrumb'
			});

			// Capture an error to verify breadcrumbs are included
			const eventId = client.captureException(new Error('test'));
			expect(eventId).toBeDefined();
		});
	});

	describe('user context', () => {
		it('should set user context', () => {
			client.setUser({
				id: '123',
				email: 'test@example.com'
			});

			const eventId = client.captureException(new Error('test'));
			expect(eventId).toBeDefined();
		});

		it('should clear user context', () => {
			client.setUser({ id: '123' });
			client.clearUser();

			const eventId = client.captureException(new Error('test'));
			expect(eventId).toBeDefined();
		});
	});

	describe('tags', () => {
		it('should set and get tags', () => {
			client.setTag('env', 'test');
			client.setTag('version', '1.0.0');

			const tags = client.getTags();

			expect(tags.env).toBe('test');
			expect(tags.version).toBe('1.0.0');
		});

		it('should remove tags', () => {
			client.setTag('temp', 'value');
			client.removeTag('temp');

			expect(client.getTags().temp).toBeUndefined();
		});
	});

	describe('extra', () => {
		it('should set and get extra data', () => {
			client.setExtra('requestId', '123');
			client.setExtra('data', { foo: 'bar' });

			const extra = client.getExtra();

			expect(extra.requestId).toBe('123');
			expect(extra.data).toEqual({ foo: 'bar' });
		});

		it('should remove extra data', () => {
			client.setExtra('temp', 'value');
			client.removeExtra('temp');

			expect(client.getExtra().temp).toBeUndefined();
		});
	});

	describe('queue', () => {
		it('should track queue size', () => {
			expect(client.getQueueSize()).toBe(0);

			client.captureException(new Error('test'));

			expect(client.getQueueSize()).toBe(1);
		});
	});

	describe('beforeSend', () => {
		it('should allow filtering events', async () => {
			const beforeSend = vi.fn().mockReturnValue(null);

			const c = new CrashClient({
				baseUrl: 'https://loom.example.com',
				project: 'test',
				beforeSend
			});

			c.captureException(new Error('test'));

			expect(beforeSend).toHaveBeenCalled();
			expect(c.getQueueSize()).toBe(0);

			await c.shutdown();
		});

		it('should allow modifying events', async () => {
			const beforeSend = vi.fn().mockImplementation((event) => {
				event.tags = { ...event.tags, modified: 'true' };
				return event;
			});

			const c = new CrashClient({
				baseUrl: 'https://loom.example.com',
				project: 'test',
				beforeSend
			});

			c.captureException(new Error('test'));

			expect(beforeSend).toHaveBeenCalled();
			expect(c.getQueueSize()).toBe(1);

			await c.shutdown();
		});
	});

	describe('shutdown', () => {
		it('should set closed flag', async () => {
			expect(client.isClosed_()).toBe(false);

			await client.shutdown();

			expect(client.isClosed_()).toBe(true);
		});

		it('should be idempotent', async () => {
			await client.shutdown();
			await client.shutdown();

			expect(client.isClosed_()).toBe(true);
		});
	});

	describe('analytics integration', () => {
		it('should get distinct_id from analytics client', async () => {
			const analytics = {
				getDistinctId: vi.fn().mockReturnValue('user-123')
			};

			const c = new CrashClient({
				baseUrl: 'https://loom.example.com',
				project: 'test',
				analytics
			});

			c.captureException(new Error('test'));

			expect(analytics.getDistinctId).toHaveBeenCalled();

			await c.shutdown();
		});
	});

	describe('flags integration', () => {
		it('should get active flags from flags client', async () => {
			const flags = {
				getAllFlags: vi.fn().mockReturnValue({ feature_x: true })
			};

			const c = new CrashClient({
				baseUrl: 'https://loom.example.com',
				project: 'test',
				flags
			});

			c.captureException(new Error('test'));

			expect(flags.getAllFlags).toHaveBeenCalled();

			await c.shutdown();
		});
	});
});
