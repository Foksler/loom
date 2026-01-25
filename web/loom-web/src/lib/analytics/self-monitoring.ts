/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

/**
 * Self-monitoring analytics for loom-web.
 *
 * Automatically initializes product analytics for the web application by
 * fetching configuration from the server. This enables Loom to track its
 * own usage without any user configuration.
 */

import { AnalyticsClient } from '@loom/analytics';
import { browser } from '$app/environment';

/**
 * Configuration returned by the self-monitoring API.
 */
interface WebAnalyticsConfig {
	api_key: string;
	release: string;
	environment: string;
}

/**
 * Global analytics client instance.
 */
let analyticsClient: AnalyticsClient | null = null;

/**
 * Initialize self-monitoring analytics for loom-web.
 *
 * This fetches the analytics SDK configuration from the server and initializes
 * the AnalyticsClient with autocapture enabled.
 *
 * Call this once during application initialization.
 *
 * @param debug - Enable debug logging
 * @returns The AnalyticsClient instance, or null if initialization failed
 */
export async function initializeAnalytics(
	debug = false
): Promise<AnalyticsClient | null> {
	// Only run in browser
	if (!browser) {
		return null;
	}

	// Don't initialize twice
	if (analyticsClient) {
		return analyticsClient;
	}

	try {
		// Fetch configuration from server
		const response = await fetch('/api/self-monitoring/analytics-config');

		if (!response.ok) {
			if (debug) {
				console.warn(
					'[Analytics] Self-monitoring not available:',
					response.status,
					response.statusText
				);
			}
			return null;
		}

		const config: WebAnalyticsConfig = await response.json();

		// Create analytics client
		analyticsClient = new AnalyticsClient({
			apiKey: config.api_key,
			baseUrl: window.location.origin,
			debug,
			autocapture: {
				pageview: true,
				pageleave: true
			},
			persistence: 'localStorage+cookie'
		});

		if (debug) {
			console.log('[Analytics] Self-monitoring initialized for loom-web');
		}

		return analyticsClient;
	} catch (error) {
		if (debug) {
			console.error('[Analytics] Failed to initialize self-monitoring:', error);
		}
		return null;
	}
}

/**
 * Get the current analytics client instance.
 *
 * Returns null if self-monitoring has not been initialized.
 */
export function getAnalyticsClient(): AnalyticsClient | null {
	return analyticsClient;
}

/**
 * Track an event using the self-monitoring analytics client.
 *
 * This is a convenience function that handles the case where
 * self-monitoring is not initialized.
 *
 * @param event - The event name
 * @param properties - Optional event properties
 */
export function capture(event: string, properties?: Record<string, unknown>): void {
	if (!analyticsClient) {
		return;
	}
	analyticsClient.capture(event, properties);
}

/**
 * Identify a user with the analytics client.
 *
 * Call this when a user logs in to link their anonymous activity
 * to their authenticated identity.
 *
 * @param userId - The user's unique identifier (e.g., user ID or email)
 * @param properties - Optional person properties to set
 */
export async function identify(
	userId: string,
	properties?: Record<string, unknown>
): Promise<void> {
	if (!analyticsClient) {
		return;
	}
	await analyticsClient.identify(userId, properties);
}

/**
 * Reset the analytics identity.
 *
 * Call this when a user logs out to generate a new anonymous ID.
 */
export function reset(): void {
	if (!analyticsClient) {
		return;
	}
	analyticsClient.reset();
}

/**
 * Set person properties on the current user.
 *
 * @param properties - Properties to set on the person
 */
export async function setProperties(
	properties: Record<string, unknown>
): Promise<void> {
	if (!analyticsClient) {
		return;
	}
	await analyticsClient.set(properties);
}

/**
 * Shutdown the self-monitoring analytics client.
 *
 * Call this during application shutdown to flush pending events.
 */
export async function shutdownAnalytics(): Promise<void> {
	if (analyticsClient) {
		await analyticsClient.shutdown();
		analyticsClient = null;
	}
}

// ============================================================================
// Tracking Helper Functions
// ============================================================================

/**
 * Track a link click event.
 *
 * @param linkName - Descriptive name of the link (e.g., 'project_card', 'back_button')
 * @param href - The destination URL
 * @param properties - Additional properties to track
 */
export function trackLinkClick(
	linkName: string,
	href: string,
	properties?: Record<string, unknown>
): void {
	capture('link_clicked', {
		link_name: linkName,
		href,
		...properties
	});
}

/**
 * Track a button click event.
 *
 * @param buttonName - Descriptive name of the button (e.g., 'create_weaver', 'delete_monitor')
 * @param properties - Additional properties to track
 */
export function trackButtonClick(
	buttonName: string,
	properties?: Record<string, unknown>
): void {
	capture('button_clicked', {
		button_name: buttonName,
		...properties
	});
}

/**
 * Track a form submission event.
 *
 * @param formName - Descriptive name of the form (e.g., 'create_org', 'monitor_settings')
 * @param properties - Additional properties to track
 */
export function trackFormSubmit(
	formName: string,
	properties?: Record<string, unknown>
): void {
	capture('form_submitted', {
		form_name: formName,
		...properties
	});
}

/**
 * Track a modal open event.
 *
 * @param modalName - Descriptive name of the modal
 * @param properties - Additional properties to track
 */
export function trackModalOpen(
	modalName: string,
	properties?: Record<string, unknown>
): void {
	capture('modal_opened', {
		modal_name: modalName,
		...properties
	});
}

/**
 * Track a modal close event.
 *
 * @param modalName - Descriptive name of the modal
 * @param properties - Additional properties to track
 */
export function trackModalClose(
	modalName: string,
	properties?: Record<string, unknown>
): void {
	capture('modal_closed', {
		modal_name: modalName,
		...properties
	});
}

/**
 * Track a filter change event.
 *
 * @param filterName - Name of the filter (e.g., 'status', 'time_range', 'org')
 * @param value - The new filter value
 * @param properties - Additional properties to track
 */
export function trackFilterChange(
	filterName: string,
	value: unknown,
	properties?: Record<string, unknown>
): void {
	capture('filter_changed', {
		filter_name: filterName,
		filter_value: value,
		...properties
	});
}

/**
 * Track an action event (resolve, delete, pause, etc.).
 *
 * @param action - The action performed (e.g., 'resolve_issue', 'delete_monitor')
 * @param resourceType - Type of resource (e.g., 'issue', 'monitor', 'weaver')
 * @param resourceId - ID of the resource
 * @param properties - Additional properties to track
 */
export function trackAction(
	action: string,
	resourceType: string,
	resourceId: string,
	properties?: Record<string, unknown>
): void {
	capture('action_performed', {
		action,
		resource_type: resourceType,
		resource_id: resourceId,
		...properties
	});
}
