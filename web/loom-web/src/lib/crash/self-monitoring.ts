/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

/**
 * Self-monitoring for loom-web.
 *
 * Automatically initializes crash monitoring for the web application by
 * fetching configuration from the server. This enables Loom to monitor itself
 * without any user configuration.
 */

import { CrashClient } from '@loom/crash';
import { browser } from '$app/environment';

/**
 * Configuration returned by the self-monitoring API.
 */
interface WebCrashConfig {
	project_id: string;
	api_key: string;
	release: string;
	environment: string;
}

/**
 * Global crash client instance.
 */
let crashClient: CrashClient | null = null;

/**
 * Initialize self-monitoring for loom-web.
 *
 * This fetches the crash SDK configuration from the server and initializes
 * the CrashClient with global error handlers.
 *
 * Call this once during application initialization.
 *
 * @param debug - Enable debug logging
 * @returns The CrashClient instance, or null if initialization failed
 */
export async function initializeSelfMonitoring(
	debug = false
): Promise<CrashClient | null> {
	// Only run in browser
	if (!browser) {
		return null;
	}

	// Don't initialize twice
	if (crashClient) {
		return crashClient;
	}

	try {
		// Fetch configuration from server
		const response = await fetch('/api/self-monitoring/web-config');

		if (!response.ok) {
			if (debug) {
				console.warn(
					'[Crash] Self-monitoring not available:',
					response.status,
					response.statusText
				);
			}
			return null;
		}

		const config: WebCrashConfig = await response.json();

		// Create crash client
		crashClient = new CrashClient({
			baseUrl: window.location.origin,
			project: config.project_id,
			apiKey: config.api_key,
			release: config.release,
			environment: config.environment,
			debug,
			sessionTracking: true
		});

		// Install global error handlers
		crashClient.installGlobalHandler();

		if (debug) {
			console.log('[Crash] Self-monitoring initialized for loom-web');
		}

		return crashClient;
	} catch (error) {
		if (debug) {
			console.error('[Crash] Failed to initialize self-monitoring:', error);
		}
		return null;
	}
}

/**
 * Get the current crash client instance.
 *
 * Returns null if self-monitoring has not been initialized.
 */
export function getCrashClient(): CrashClient | null {
	return crashClient;
}

/**
 * Capture an exception using the self-monitoring crash client.
 *
 * This is a convenience function that handles the case where
 * self-monitoring is not initialized.
 *
 * @param error - The error to capture
 * @returns The event ID, or empty string if not initialized
 */
export function captureException(error: unknown): string {
	if (!crashClient) {
		return '';
	}
	return crashClient.captureException(error);
}

/**
 * Capture a message using the self-monitoring crash client.
 *
 * @param message - The message to capture
 * @returns The event ID, or empty string if not initialized
 */
export function captureMessage(message: string): string {
	if (!crashClient) {
		return '';
	}
	return crashClient.captureMessage(message);
}

/**
 * Shutdown the self-monitoring crash client.
 *
 * Call this during application shutdown to flush pending events.
 */
export async function shutdownSelfMonitoring(): Promise<void> {
	if (crashClient) {
		await crashClient.shutdown();
		crashClient = null;
	}
}
