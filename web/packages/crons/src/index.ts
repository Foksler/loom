/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

/**
 * Cron monitoring SDK for Loom.
 *
 * This package provides:
 * - Check-in tracking for scheduled jobs
 * - Automatic duration calculation
 * - Integration with @loom/crash for error linking
 * - Convenience wrapper for job execution
 *
 * @example
 * ```typescript
 * import { CronsClient } from '@loom/crons';
 *
 * const crons = new CronsClient({
 *   baseUrl: 'https://loom.example.com',
 *   orgId: 'my-org-id',
 *   authToken: 'lt_xxx',
 * });
 *
 * // Simple wrapper pattern
 * await crons.withMonitor('daily-cleanup', async () => {
 *   await cleanupOldRecords();
 * });
 *
 * // Manual pattern for more control
 * const id = await crons.checkinStart('email-digest');
 * try {
 *   await sendEmails();
 *   await crons.checkinOk(id, { output: 'Sent 150 emails' });
 * } catch (e) {
 *   await crons.checkinError(id, { output: e.message });
 * }
 * ```
 */

export { CronsClient } from './client';

export type {
	CronsClientOptions,
	CheckInOk,
	CheckInError,
	CheckIn,
	CheckInStatus,
	CheckInSource,
	CheckInResponse,
	CheckInRequest,
	CheckInUpdateRequest,
	Monitor,
	MonitorHealth,
	MonitorStatus,
	MonitorSchedule,
	CrashClientInterface
} from './types';

export { SDK_NAME, SDK_VERSION } from './types';

export {
	CronsError,
	ConfigurationError,
	InvalidBaseUrlError,
	AuthenticationError,
	RateLimitedError,
	ClientClosedError,
	CheckInError as CheckInOperationError,
	MonitorNotFoundError,
	NetworkError,
	ServerError,
	JobFailedError
} from './errors';
