/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { HttpClient } from '@loom/http';
import type {
	CronsClientOptions,
	CheckInOk,
	CheckInError,
	CheckInResponse,
	CheckInRequest,
	CheckInUpdateRequest,
	CrashClientInterface
} from './types';
import { SDK_NAME, SDK_VERSION } from './types';
import { ConfigurationError, InvalidBaseUrlError, ClientClosedError, JobFailedError } from './errors';

/**
 * Validate base URL format.
 */
function validateBaseUrl(baseUrl: string): void {
	if (!baseUrl || typeof baseUrl !== 'string') {
		throw new InvalidBaseUrlError('Base URL is required');
	}
	try {
		new URL(baseUrl);
	} catch {
		throw new InvalidBaseUrlError(`Invalid base URL: ${baseUrl}`);
	}
}

/**
 * Cron monitoring client for tracking scheduled job execution.
 *
 * @example
 * ```typescript
 * const crons = new CronsClient({
 *   baseUrl: 'https://loom.example.com',
 *   orgId: 'my-org-id',
 *   authToken: 'lt_xxx',
 * });
 *
 * // Manual check-in pattern
 * const checkinId = await crons.checkinStart('email-digest');
 * try {
 *   await sendEmailDigest();
 *   await crons.checkinOk(checkinId, { output: 'Sent 150 emails' });
 * } catch (error) {
 *   await crons.checkinError(checkinId, { output: error.message });
 * }
 *
 * // Or use the convenience wrapper
 * await crons.withMonitor('email-digest', async () => {
 *   await sendEmailDigest();
 * });
 *
 * // Shutdown when done
 * crons.close();
 * ```
 */
export class CronsClient {
	private readonly httpClient: HttpClient;
	private readonly orgId: string;
	private readonly environment?: string;
	private readonly release?: string;
	private readonly debug: boolean;
	private readonly crashClient?: CrashClientInterface;

	private isClosed = false;

	constructor(options: CronsClientOptions) {
		validateBaseUrl(options.baseUrl);

		if (!options.orgId) {
			throw new ConfigurationError('Organization ID is required');
		}

		this.orgId = options.orgId;
		this.environment = options.environment;
		this.release = options.release;
		this.debug = options.debug ?? false;
		this.crashClient = options.crashClient;

		// Create HTTP client
		const headers: Record<string, string> = {
			'Content-Type': 'application/json'
		};

		if (options.authToken) {
			headers['Authorization'] = `Bearer ${options.authToken}`;
		} else if (options.apiKey) {
			headers['X-Crons-Api-Key'] = options.apiKey;
		}

		this.httpClient = new HttpClient({
			baseUrl: options.baseUrl,
			timeoutMs: options.timeoutMs ?? 30000,
			userAgent: `${SDK_NAME}/${SDK_VERSION}`,
			defaultHeaders: headers
		});
	}

	/**
	 * Start a check-in (job starting).
	 *
	 * @param monitorSlug - The monitor slug
	 * @returns The check-in ID
	 */
	async checkinStart(monitorSlug: string): Promise<string> {
		if (this.isClosed) {
			throw new ClientClosedError();
		}

		const request: CheckInRequest = {
			org_id: this.orgId,
			status: 'in_progress',
			started_at: new Date().toISOString(),
			environment: this.environment,
			release: this.release
		};

		const response = await this.httpClient.postJson<CheckInResponse>(
			`/api/crons/monitors/${encodeURIComponent(monitorSlug)}/checkins`,
			request
		);

		if (this.debug) {
			console.log(`[Crons] Check-in started for ${monitorSlug}: ${response.id}`);
		}

		return response.id;
	}

	/**
	 * Complete a check-in successfully.
	 *
	 * @param checkinId - The check-in ID from checkinStart
	 * @param details - Optional details about the successful run
	 */
	async checkinOk(checkinId: string, details?: CheckInOk): Promise<void> {
		if (this.isClosed) {
			throw new ClientClosedError();
		}

		const request: CheckInUpdateRequest = {
			status: 'ok',
			finished_at: new Date().toISOString(),
			duration_ms: details?.durationMs,
			output: details?.output
		};

		await this.httpClient.patch(
			`/api/crons/checkins/${encodeURIComponent(checkinId)}`,
			request
		);

		if (this.debug) {
			console.log(`[Crons] Check-in completed successfully: ${checkinId}`);
		}
	}

	/**
	 * Complete a check-in with error.
	 *
	 * @param checkinId - The check-in ID from checkinStart
	 * @param details - Details about the failed run
	 */
	async checkinError(checkinId: string, details?: CheckInError): Promise<void> {
		if (this.isClosed) {
			throw new ClientClosedError();
		}

		const request: CheckInUpdateRequest = {
			status: 'error',
			finished_at: new Date().toISOString(),
			duration_ms: details?.durationMs,
			exit_code: details?.exitCode,
			output: details?.output,
			crash_event_id: details?.crashEventId
		};

		await this.httpClient.patch(
			`/api/crons/checkins/${encodeURIComponent(checkinId)}`,
			request
		);

		if (this.debug) {
			console.log(`[Crons] Check-in completed with error: ${checkinId}`);
		}
	}

	/**
	 * Convenience wrapper that handles check-in lifecycle.
	 *
	 * @param monitorSlug - The monitor slug
	 * @param fn - The async function to execute
	 * @returns The result of the function
	 * @throws JobFailedError if the function throws
	 *
	 * @example
	 * ```typescript
	 * const result = await crons.withMonitor('email-digest', async () => {
	 *   await sendEmailDigest();
	 *   return { sent: 150 };
	 * });
	 * ```
	 */
	async withMonitor<T>(monitorSlug: string, fn: () => Promise<T>): Promise<T> {
		if (this.isClosed) {
			throw new ClientClosedError();
		}

		const startTime = Date.now();
		const checkinId = await this.checkinStart(monitorSlug);

		try {
			const result = await fn();
			const durationMs = Date.now() - startTime;

			await this.checkinOk(checkinId, { durationMs });

			return result;
		} catch (error) {
			const durationMs = Date.now() - startTime;

			// Optionally capture in crash system
			let crashEventId: string | undefined;
			if (this.crashClient) {
				try {
					crashEventId = this.crashClient.captureException(error, {
						tags: { monitor_slug: monitorSlug }
					});
				} catch (captureError) {
					if (this.debug) {
						console.error('[Crons] Failed to capture crash:', captureError);
					}
				}
			}

			const errorMessage = error instanceof Error ? error.message : String(error);

			await this.checkinError(checkinId, {
				durationMs,
				exitCode: 1,
				output: errorMessage,
				crashEventId
			});

			throw new JobFailedError(monitorSlug, error);
		}
	}

	/**
	 * Check if the client is closed.
	 */
	isClosed_(): boolean {
		return this.isClosed;
	}

	/**
	 * Close the client.
	 * After closing, no more check-ins can be made.
	 */
	close(): void {
		if (this.isClosed) {
			return;
		}

		this.isClosed = true;

		if (this.debug) {
			console.log('[Crons] Client closed');
		}
	}
}
