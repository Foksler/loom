/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

export {
	initializeAnalytics,
	getAnalyticsClient,
	capture,
	identify,
	reset,
	setProperties,
	shutdownAnalytics,
	trackLinkClick,
	trackButtonClick,
	trackFormSubmit,
	trackModalOpen,
	trackModalClose,
	trackFilterChange,
	trackAction
} from './self-monitoring';

export { default as AnalyticsProvider } from './AnalyticsProvider.svelte';
