/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import type { HttpClient } from '@loom/http';
import type { SessionConfig, SessionStatus, SessionStartResponse } from './types';

/**
 * Generate a UUID v4.
 */
function generateSessionId(): string {
	if (typeof crypto !== 'undefined' && crypto.randomUUID) {
		return crypto.randomUUID();
	}
	// Fallback for older browsers
	return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
		const r = (Math.random() * 16) | 0;
		const v = c === 'x' ? r : (r & 0x3) | 0x8;
		return v.toString(16);
	});
}

/**
 * Deterministic hash of a string to a number between 0 and 1.
 * This ensures the same session ID always produces the same sampling decision.
 */
function hashToFloat(str: string): number {
	let hash = 0;
	for (let i = 0; i < str.length; i++) {
		const char = str.charCodeAt(i);
		hash = (hash << 5) - hash + char;
		hash = hash & hash; // Convert to 32-bit integer
	}
	// Normalize to 0-1 range
	return Math.abs(hash) / 2147483647;
}

/**
 * Determine if a session should be sampled based on session ID and sample rate.
 */
function shouldSample(sessionId: string, sampleRate: number): boolean {
	if (sampleRate >= 1.0) return true;
	if (sampleRate <= 0.0) return false;
	return hashToFloat(sessionId) < sampleRate;
}

/**
 * Detect the current platform.
 */
function detectPlatform(): string {
	if (typeof window !== 'undefined') {
		return 'javascript';
	}
	return 'node';
}

/**
 * Browser session tracker for tracking user engagement periods.
 *
 * Sessions are used to calculate release health metrics like crash-free rate.
 * The tracker automatically handles:
 * - Session start on initialization
 * - Session end on page unload (beforeunload, pagehide)
 * - Visibility change handling (30 minute timeout)
 * - Error and crash counting
 * - Deterministic sampling based on session ID
 */
export class SessionTracker {
	private readonly sessionId: string;
	private readonly startedAt: Date;
	private readonly config: SessionConfig;
	private readonly httpClient: HttpClient;
	private readonly debug: boolean;

	private errorCount = 0;
	private crashCount = 0;
	private ended = false;
	private sampled = false;
	private endTimeout?: ReturnType<typeof setTimeout>;

	// Visibility tracking
	private readonly visibilityTimeoutMs = 30 * 60 * 1000; // 30 minutes

	// Bound event handlers for cleanup
	private readonly boundBeforeUnload: () => void;
	private readonly boundPageHide: () => void;
	private readonly boundVisibilityChange: () => void;

	constructor(
		httpClient: HttpClient,
		config: SessionConfig,
		options?: { debug?: boolean }
	) {
		this.sessionId = generateSessionId();
		this.startedAt = new Date();
		this.config = config;
		this.httpClient = httpClient;
		this.debug = options?.debug ?? false;

		// Determine sampling based on session ID (deterministic)
		this.sampled = shouldSample(this.sessionId, config.sampleRate);

		// Bind event handlers
		this.boundBeforeUnload = () => this.end();
		this.boundPageHide = () => this.end();
		this.boundVisibilityChange = () => this.handleVisibilityChange();
	}

	/**
	 * Get the session ID.
	 */
	getSessionId(): string {
		return this.sessionId;
	}

	/**
	 * Check if this session is being sampled.
	 */
	isSampled(): boolean {
		return this.sampled;
	}

	/**
	 * Check if the session has ended.
	 */
	isEnded(): boolean {
		return this.ended;
	}

	/**
	 * Get the current error count.
	 */
	getErrorCount(): number {
		return this.errorCount;
	}

	/**
	 * Get the current crash count.
	 */
	getCrashCount(): number {
		return this.crashCount;
	}

	/**
	 * Get the appropriate endpoint path based on config.
	 */
	private getStartEndpoint(): string {
		return this.config.useSdkEndpoints ? '/api/sessions/start/sdk' : '/api/sessions/start';
	}

	/**
	 * Get the appropriate end endpoint path based on config.
	 */
	private getEndEndpoint(): string {
		return this.config.useSdkEndpoints ? '/api/sessions/end/sdk' : '/api/sessions/end';
	}

	/**
	 * Start the session and register event handlers.
	 */
	async start(): Promise<void> {
		// Only send start request if sampled
		if (!this.sampled) {
			if (this.debug) {
				console.log('[Session] Session not sampled, skipping start request');
			}
			this.installEventHandlers();
			return;
		}

		try {
			const response = await this.httpClient.postJson<SessionStartResponse>(
				this.getStartEndpoint(),
				{
					project_id: this.config.projectId,
					distinct_id: this.config.distinctId,
					person_id: this.config.personId,
					platform: detectPlatform(),
					environment: this.config.environment,
					release: this.config.release,
					user_agent: typeof navigator !== 'undefined' ? navigator.userAgent : undefined,
					sample_rate: this.config.sampleRate
				}
			);

			if (this.debug) {
				console.log('[Session] Session started:', response.session_id, 'sampled:', response.sampled);
			}
		} catch (error) {
			if (this.debug) {
				console.error('[Session] Failed to start session:', error);
			}
			// Don't throw - session tracking should not break the app
		}

		this.installEventHandlers();
	}

	/**
	 * Install browser event handlers for automatic session ending.
	 */
	private installEventHandlers(): void {
		if (typeof window === 'undefined') {
			return;
		}

		window.addEventListener('beforeunload', this.boundBeforeUnload);
		window.addEventListener('pagehide', this.boundPageHide);
		document.addEventListener('visibilitychange', this.boundVisibilityChange);

		if (this.debug) {
			console.log('[Session] Event handlers installed');
		}
	}

	/**
	 * Uninstall browser event handlers.
	 */
	private uninstallEventHandlers(): void {
		if (typeof window === 'undefined') {
			return;
		}

		window.removeEventListener('beforeunload', this.boundBeforeUnload);
		window.removeEventListener('pagehide', this.boundPageHide);
		document.removeEventListener('visibilitychange', this.boundVisibilityChange);

		if (this.endTimeout) {
			clearTimeout(this.endTimeout);
			this.endTimeout = undefined;
		}

		if (this.debug) {
			console.log('[Session] Event handlers uninstalled');
		}
	}

	/**
	 * Handle visibility change events.
	 * When the page becomes hidden, schedule session end after 30 minutes.
	 * When the page becomes visible again, cancel the scheduled end.
	 */
	private handleVisibilityChange(): void {
		if (typeof document === 'undefined') {
			return;
		}

		if (document.visibilityState === 'hidden') {
			// Schedule end after 30 minutes of being hidden
			this.endTimeout = setTimeout(() => {
				this.end();
			}, this.visibilityTimeoutMs);

			if (this.debug) {
				console.log('[Session] Page hidden, scheduled end in 30 minutes');
			}
		} else {
			// Page became visible again, cancel scheduled end
			if (this.endTimeout) {
				clearTimeout(this.endTimeout);
				this.endTimeout = undefined;

				if (this.debug) {
					console.log('[Session] Page visible, cancelled scheduled end');
				}
			}
		}
	}

	/**
	 * Record a handled error in this session.
	 */
	recordError(): void {
		this.errorCount++;
	}

	/**
	 * Record an unhandled crash in this session.
	 */
	recordCrash(): void {
		this.crashCount++;
	}

	/**
	 * Get the session status based on error/crash counts.
	 */
	private getStatus(): SessionStatus {
		if (this.crashCount > 0) {
			return 'crashed';
		}
		if (this.errorCount > 0) {
			return 'errored';
		}
		return 'exited';
	}

	/**
	 * End the session.
	 *
	 * This method is called automatically on page unload, but can also be called manually.
	 * Uses sendBeacon for reliability on page unload.
	 */
	end(): void {
		if (this.ended) {
			return;
		}
		this.ended = true;

		this.uninstallEventHandlers();

		// Always send crashed sessions, even if not sampled
		if (!this.sampled && this.crashCount === 0) {
			if (this.debug) {
				console.log('[Session] Session not sampled and not crashed, skipping end request');
			}
			return;
		}

		const endedAt = new Date();
		const durationMs = endedAt.getTime() - this.startedAt.getTime();

		const body = JSON.stringify({
			project_id: this.config.projectId,
			session_id: this.sessionId,
			status: this.getStatus(),
			error_count: this.errorCount,
			crash_count: this.crashCount,
			duration_ms: durationMs
		});

		// Use sendBeacon for reliability on page unload (browser only)
		if (typeof navigator !== 'undefined' && navigator.sendBeacon) {
			const url = this.buildEndSessionUrl();
			const blob = new Blob([body], { type: 'application/json' });
			const success = navigator.sendBeacon(url, blob);

			if (this.debug) {
				console.log('[Session] Session ended via sendBeacon:', success);
			}
		} else {
			// Fallback to regular HTTP request (Node.js or browsers without sendBeacon)
			this.httpClient
				.post(this.getEndEndpoint(), JSON.parse(body))
				.then(() => {
					if (this.debug) {
						console.log('[Session] Session ended via HTTP');
					}
				})
				.catch((error) => {
					if (this.debug) {
						console.error('[Session] Failed to end session:', error);
					}
				});
		}
	}

	/**
	 * Build the full URL for the end session endpoint.
	 */
	private buildEndSessionUrl(): string {
		// Access the baseUrl from the HTTP client
		// Note: This assumes the HTTP client has the baseUrl configured
		// For sendBeacon, we need the full URL including auth headers
		// Since sendBeacon doesn't support custom headers, we need to work around this
		// The server should accept the request based on the project_id and auth context

		// Get baseUrl from the config or fall back to relative path
		const baseUrl = this.config.baseUrl ?? '';
		return `${baseUrl}${this.getEndEndpoint()}`;
	}

	/**
	 * End the session asynchronously (for use when you can wait for the response).
	 */
	async endAsync(): Promise<void> {
		if (this.ended) {
			return;
		}
		this.ended = true;

		this.uninstallEventHandlers();

		// Always send crashed sessions, even if not sampled
		if (!this.sampled && this.crashCount === 0) {
			if (this.debug) {
				console.log('[Session] Session not sampled and not crashed, skipping end request');
			}
			return;
		}

		const endedAt = new Date();
		const durationMs = endedAt.getTime() - this.startedAt.getTime();

		try {
			await this.httpClient.post(this.getEndEndpoint(), {
				project_id: this.config.projectId,
				session_id: this.sessionId,
				status: this.getStatus(),
				error_count: this.errorCount,
				crash_count: this.crashCount,
				duration_ms: durationMs
			});

			if (this.debug) {
				console.log('[Session] Session ended');
			}
		} catch (error) {
			if (this.debug) {
				console.error('[Session] Failed to end session:', error);
			}
			// Don't throw - session tracking should not break the app
		}
	}
}
