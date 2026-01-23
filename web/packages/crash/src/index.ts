/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

// Main client
export { CrashClient } from './client';

// Types
export type {
	Platform,
	BreadcrumbLevel,
	IssueLevel,
	Breadcrumb,
	UserContext,
	DeviceContext,
	BrowserContext,
	OsContext,
	RequestContext,
	StackFrame,
	Stacktrace,
	Mechanism,
	CrashEvent,
	CaptureResponse,
	CaptureOptions,
	BatchConfig,
	BeforeSendHook,
	CrashClientOptions,
	SessionStatus,
	SessionConfig,
	SessionStartResponse,
	SessionEndResponse
} from './types';

export { SDK_NAME, SDK_VERSION, DEFAULT_BATCH_CONFIG } from './types';

// Session tracking
export { SessionTracker } from './session';

// Errors
export {
	CrashError,
	ConfigurationError,
	InvalidBaseUrlError,
	AuthenticationError,
	RateLimitedError,
	ClientClosedError,
	CaptureError,
	StackParseError,
	NetworkError,
	ServerError
} from './errors';

// Breadcrumb utilities
export {
	BreadcrumbManager,
	httpBreadcrumb,
	navigationBreadcrumb,
	uiBreadcrumb,
	consoleBreadcrumb,
	userBreadcrumb,
	debugBreadcrumb
} from './breadcrumb';

// Stack trace parsing
export {
	parseStackTrace,
	parseStackString,
	getExceptionType,
	getExceptionValue,
	findCulprit
} from './stacktrace';

// Global handlers
export {
	installGlobalErrorHandler,
	installUnhandledRejectionHandler,
	installGlobalHandlers,
	uninstallGlobalHandlers,
	wrapConsole
} from './global-handler';
