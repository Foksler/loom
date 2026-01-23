/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { describe, it, expect } from 'vitest';
import { fc, test } from '@fast-check/vitest';
import { SDK_NAME, SDK_VERSION } from './types';
import type {
	CheckInStatus,
	CheckInSource,
	MonitorHealth,
	MonitorStatus,
	MonitorSchedule,
	CheckInOk,
	CheckInError,
	CronsClientOptions
} from './types';

describe('SDK constants', () => {
	it('should have correct SDK name', () => {
		expect(SDK_NAME).toBe('@loom/crons');
	});

	it('should have correct SDK version', () => {
		expect(SDK_VERSION).toBe('0.1.0');
	});
});

describe('CheckInStatus type', () => {
	it('should allow valid status values', () => {
		const validStatuses: CheckInStatus[] = ['in_progress', 'ok', 'error', 'missed', 'timeout'];

		validStatuses.forEach((status) => {
			expect(status).toBeDefined();
		});
	});
});

describe('CheckInSource type', () => {
	it('should allow valid source values', () => {
		const validSources: CheckInSource[] = ['ping', 'sdk', 'manual', 'system'];

		validSources.forEach((source) => {
			expect(source).toBeDefined();
		});
	});
});

describe('MonitorHealth type', () => {
	it('should allow valid health values', () => {
		const validHealth: MonitorHealth[] = ['healthy', 'failing', 'missed', 'timeout', 'unknown'];

		validHealth.forEach((health) => {
			expect(health).toBeDefined();
		});
	});
});

describe('MonitorStatus type', () => {
	it('should allow valid status values', () => {
		const validStatuses: MonitorStatus[] = ['active', 'paused', 'disabled'];

		validStatuses.forEach((status) => {
			expect(status).toBeDefined();
		});
	});
});

describe('MonitorSchedule type', () => {
	it('should allow cron schedule', () => {
		const schedule: MonitorSchedule = { type: 'cron', expression: '0 0 * * *' };
		expect(schedule.type).toBe('cron');
		expect(schedule.expression).toBe('0 0 * * *');
	});

	it('should allow interval schedule', () => {
		const schedule: MonitorSchedule = { type: 'interval', minutes: 30 };
		expect(schedule.type).toBe('interval');
		expect(schedule.minutes).toBe(30);
	});
});

describe('CheckInOk type', () => {
	it('should allow empty object', () => {
		const ok: CheckInOk = {};
		expect(ok.durationMs).toBeUndefined();
		expect(ok.output).toBeUndefined();
	});

	it('should allow duration and output', () => {
		const ok: CheckInOk = { durationMs: 1500, output: 'Success!' };
		expect(ok.durationMs).toBe(1500);
		expect(ok.output).toBe('Success!');
	});
});

describe('CheckInError type', () => {
	it('should allow empty object', () => {
		const error: CheckInError = {};
		expect(error.durationMs).toBeUndefined();
		expect(error.exitCode).toBeUndefined();
		expect(error.output).toBeUndefined();
		expect(error.crashEventId).toBeUndefined();
	});

	it('should allow all properties', () => {
		const error: CheckInError = {
			durationMs: 5000,
			exitCode: 1,
			output: 'Connection timeout',
			crashEventId: 'event-123'
		};
		expect(error.durationMs).toBe(5000);
		expect(error.exitCode).toBe(1);
		expect(error.output).toBe('Connection timeout');
		expect(error.crashEventId).toBe('event-123');
	});
});

describe('CronsClientOptions type', () => {
	it('should require baseUrl and orgId', () => {
		const options: CronsClientOptions = {
			baseUrl: 'https://example.com',
			orgId: 'org-123'
		};
		expect(options.baseUrl).toBe('https://example.com');
		expect(options.orgId).toBe('org-123');
	});

	it('should allow authToken', () => {
		const options: CronsClientOptions = {
			baseUrl: 'https://example.com',
			orgId: 'org-123',
			authToken: 'lt_xxx'
		};
		expect(options.authToken).toBe('lt_xxx');
	});

	it('should allow apiKey', () => {
		const options: CronsClientOptions = {
			baseUrl: 'https://example.com',
			orgId: 'org-123',
			apiKey: 'api_xxx'
		};
		expect(options.apiKey).toBe('api_xxx');
	});

	it('should allow all optional properties', () => {
		const options: CronsClientOptions = {
			baseUrl: 'https://example.com',
			orgId: 'org-123',
			environment: 'production',
			release: '1.0.0',
			timeoutMs: 5000,
			debug: true
		};
		expect(options.environment).toBe('production');
		expect(options.release).toBe('1.0.0');
		expect(options.timeoutMs).toBe(5000);
		expect(options.debug).toBe(true);
	});
});

describe('proptest: CheckInStatus', () => {
	const statusArb = fc.constantFrom('in_progress', 'ok', 'error', 'missed', 'timeout');

	test.prop([statusArb])('should preserve status through assignment', (status) => {
		const s: CheckInStatus = status as CheckInStatus;
		expect(s).toBe(status);
	});
});

describe('proptest: CheckInSource', () => {
	const sourceArb = fc.constantFrom('ping', 'sdk', 'manual', 'system');

	test.prop([sourceArb])('should preserve source through assignment', (source) => {
		const s: CheckInSource = source as CheckInSource;
		expect(s).toBe(source);
	});
});

describe('proptest: MonitorHealth', () => {
	const healthArb = fc.constantFrom('healthy', 'failing', 'missed', 'timeout', 'unknown');

	test.prop([healthArb])('should preserve health through assignment', (health) => {
		const h: MonitorHealth = health as MonitorHealth;
		expect(h).toBe(health);
	});
});

describe('proptest: CheckInOk', () => {
	const okArb = fc.record({
		durationMs: fc.option(fc.nat()),
		output: fc.option(fc.string())
	});

	test.prop([okArb])('should preserve CheckInOk properties', (ok) => {
		const checkInOk: CheckInOk = {
			durationMs: ok.durationMs ?? undefined,
			output: ok.output ?? undefined
		};

		if (ok.durationMs !== null) {
			expect(checkInOk.durationMs).toBe(ok.durationMs);
		}
		if (ok.output !== null) {
			expect(checkInOk.output).toBe(ok.output);
		}
	});
});

describe('proptest: CheckInError', () => {
	const errorArb = fc.record({
		durationMs: fc.option(fc.nat()),
		exitCode: fc.option(fc.integer()),
		output: fc.option(fc.string()),
		crashEventId: fc.option(fc.string())
	});

	test.prop([errorArb])('should preserve CheckInError properties', (error) => {
		const checkInError: CheckInError = {
			durationMs: error.durationMs ?? undefined,
			exitCode: error.exitCode ?? undefined,
			output: error.output ?? undefined,
			crashEventId: error.crashEventId ?? undefined
		};

		if (error.durationMs !== null) {
			expect(checkInError.durationMs).toBe(error.durationMs);
		}
		if (error.exitCode !== null) {
			expect(checkInError.exitCode).toBe(error.exitCode);
		}
	});
});
