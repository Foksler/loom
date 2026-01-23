/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { describe, it, expect } from 'vitest';
import { SDK_NAME, SDK_VERSION, DEFAULT_BATCH_CONFIG } from './types';

describe('SDK constants', () => {
	it('should have correct SDK name', () => {
		expect(SDK_NAME).toBe('@loom/crash');
	});

	it('should have a version string', () => {
		expect(SDK_VERSION).toBeDefined();
		expect(typeof SDK_VERSION).toBe('string');
		expect(SDK_VERSION).toMatch(/^\d+\.\d+\.\d+$/);
	});
});

describe('DEFAULT_BATCH_CONFIG', () => {
	it('should have reasonable defaults', () => {
		expect(DEFAULT_BATCH_CONFIG.flushIntervalMs).toBe(5000);
		expect(DEFAULT_BATCH_CONFIG.maxBatchSize).toBe(10);
		expect(DEFAULT_BATCH_CONFIG.maxQueueSize).toBe(100);
	});

	it('should have all required fields', () => {
		expect(DEFAULT_BATCH_CONFIG).toHaveProperty('flushIntervalMs');
		expect(DEFAULT_BATCH_CONFIG).toHaveProperty('maxBatchSize');
		expect(DEFAULT_BATCH_CONFIG).toHaveProperty('maxQueueSize');
	});
});
