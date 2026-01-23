/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import type { Breadcrumb } from './types';

/**
 * Manages a circular buffer of breadcrumbs.
 */
export class BreadcrumbManager {
	private breadcrumbs: Breadcrumb[] = [];
	private readonly maxBreadcrumbs: number;

	constructor(maxBreadcrumbs: number = 100) {
		this.maxBreadcrumbs = maxBreadcrumbs;
	}

	/**
	 * Add a breadcrumb to the buffer.
	 * If the buffer is full, the oldest breadcrumb is removed.
	 */
	add(breadcrumb: Breadcrumb): void {
		const crumb: Breadcrumb = {
			timestamp: new Date().toISOString(),
			...breadcrumb
		};

		this.breadcrumbs.push(crumb);

		// Remove oldest if over limit
		while (this.breadcrumbs.length > this.maxBreadcrumbs) {
			this.breadcrumbs.shift();
		}
	}

	/**
	 * Get all breadcrumbs in chronological order.
	 */
	getAll(): Breadcrumb[] {
		return [...this.breadcrumbs];
	}

	/**
	 * Clear all breadcrumbs.
	 */
	clear(): void {
		this.breadcrumbs = [];
	}

	/**
	 * Get the number of breadcrumbs.
	 */
	get count(): number {
		return this.breadcrumbs.length;
	}
}

/**
 * Create a breadcrumb for an HTTP request.
 */
export function httpBreadcrumb(
	method: string,
	url: string,
	statusCode?: number,
	data?: Record<string, unknown>
): Breadcrumb {
	return {
		type: 'http',
		category: 'http',
		message: `${method} ${url}`,
		level: statusCode && statusCode >= 400 ? 'error' : 'info',
		data: {
			method,
			url,
			status_code: statusCode,
			...data
		}
	};
}

/**
 * Create a breadcrumb for navigation.
 */
export function navigationBreadcrumb(from: string, to: string): Breadcrumb {
	return {
		type: 'navigation',
		category: 'navigation',
		message: `Navigated from ${from} to ${to}`,
		level: 'info',
		data: {
			from,
			to
		}
	};
}

/**
 * Create a breadcrumb for a UI interaction.
 */
export function uiBreadcrumb(
	element: string,
	action: string = 'click',
	data?: Record<string, unknown>
): Breadcrumb {
	return {
		type: 'ui',
		category: 'ui.' + action,
		message: `${action} on ${element}`,
		level: 'info',
		data
	};
}

/**
 * Create a breadcrumb for a console message.
 */
export function consoleBreadcrumb(
	level: 'log' | 'info' | 'warn' | 'error' | 'debug',
	message: string,
	args?: unknown[]
): Breadcrumb {
	return {
		type: 'console',
		category: 'console',
		message,
		level: level === 'error' ? 'error' : level === 'warn' ? 'warning' : 'info',
		data: args ? { arguments: args } : undefined
	};
}

/**
 * Create a breadcrumb for a user action.
 */
export function userBreadcrumb(
	action: string,
	message: string,
	data?: Record<string, unknown>
): Breadcrumb {
	return {
		type: 'user',
		category: action,
		message,
		level: 'info',
		data
	};
}

/**
 * Create a debug breadcrumb.
 */
export function debugBreadcrumb(message: string, data?: Record<string, unknown>): Breadcrumb {
	return {
		type: 'debug',
		category: 'debug',
		message,
		level: 'debug',
		data
	};
}
