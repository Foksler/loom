/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import type { StackFrame, Stacktrace } from './types';

/**
 * Regular expression for Chrome/V8 stack trace format.
 * Example: "    at functionName (filename:line:col)"
 * Example: "    at filename:line:col"
 * Example: "    at async functionName (filename:line:col)"
 */
const CHROME_REGEX = /^\s*at\s+(?:(?:async\s+)?(.+?)\s+\()?(?:(.+?):(\d+):(\d+))\)?$/;

/**
 * Regular expression for Firefox stack trace format.
 * Example: "functionName@filename:line:col"
 */
const FIREFOX_REGEX = /^(.+?)@(.+?):(\d+):(\d+)$/;

/**
 * Regular expression for Safari stack trace format.
 * Example: "functionName@filename:line:col"
 */
const SAFARI_REGEX = /^(.+?)@(.+?):(\d+):(\d+)$/;

/**
 * Regular expression to detect anonymous functions.
 */
const ANONYMOUS_REGEX = /^(?:\s*at\s+)?(?:anonymous|<anonymous>|\[native code\])$/i;

/**
 * Patterns that indicate a file is from a library/framework.
 */
const LIBRARY_PATTERNS = [
	/node_modules/,
	/webpack/,
	/vite/,
	/@loom\/(http|analytics|flags)/,
	/^internal\//,
	/^node:/,
	/chrome-extension:/,
	/moz-extension:/,
	/safari-extension:/
];

/**
 * Check if a filename is from a library/framework (not user code).
 */
function isLibraryFile(filename: string): boolean {
	return LIBRARY_PATTERNS.some((pattern) => pattern.test(filename));
}

/**
 * Clean up a function name.
 */
function cleanFunctionName(name: string | undefined): string | undefined {
	if (!name) return undefined;

	// Remove async/await prefixes
	let cleaned = name.replace(/^async\s+/, '');

	// Clean up anonymous functions
	if (ANONYMOUS_REGEX.test(cleaned)) {
		return '<anonymous>';
	}

	// Remove webpack module prefixes
	cleaned = cleaned.replace(/^Object\./, '');
	cleaned = cleaned.replace(/^\$\w+\./, '');

	return cleaned || undefined;
}

/**
 * Parse a V8/Chrome stack trace line.
 */
function parseChromeFrame(line: string): StackFrame | null {
	const match = line.match(CHROME_REGEX);
	if (!match) return null;

	const [, funcName, filename, lineStr, colStr] = match;

	return {
		function: cleanFunctionName(funcName),
		filename: filename,
		abs_path: filename,
		lineno: lineStr ? parseInt(lineStr, 10) : undefined,
		colno: colStr ? parseInt(colStr, 10) : undefined,
		in_app: filename ? !isLibraryFile(filename) : true
	};
}

/**
 * Parse a Firefox/Safari stack trace line.
 */
function parseFirefoxFrame(line: string): StackFrame | null {
	const match = line.match(FIREFOX_REGEX);
	if (!match) return null;

	const [, funcName, filename, lineStr, colStr] = match;

	return {
		function: cleanFunctionName(funcName),
		filename: filename,
		abs_path: filename,
		lineno: lineStr ? parseInt(lineStr, 10) : undefined,
		colno: colStr ? parseInt(colStr, 10) : undefined,
		in_app: filename ? !isLibraryFile(filename) : true
	};
}

/**
 * Parse a single stack trace line.
 */
function parseFrame(line: string): StackFrame | null {
	// Try Chrome format first
	const chromeFrame = parseChromeFrame(line);
	if (chromeFrame) return chromeFrame;

	// Try Firefox/Safari format
	const firefoxFrame = parseFirefoxFrame(line);
	if (firefoxFrame) return firefoxFrame;

	return null;
}

/**
 * Parse a JavaScript error stack trace into frames.
 *
 * @param error - The error to parse
 * @returns Parsed stacktrace with frames
 */
export function parseStackTrace(error: Error): Stacktrace {
	if (!error.stack) {
		return { frames: [] };
	}

	const lines = error.stack.split('\n');
	const frames: StackFrame[] = [];

	for (const line of lines) {
		// Skip the first line if it's the error message
		if (line.includes(error.message) && !line.includes('at ') && !line.includes('@')) {
			continue;
		}

		const frame = parseFrame(line.trim());
		if (frame) {
			frames.push(frame);
		}
	}

	// Reverse to have most recent call last (Sentry convention)
	frames.reverse();

	return { frames };
}

/**
 * Parse a stack trace string directly.
 *
 * @param stack - The stack trace string
 * @returns Parsed stacktrace with frames
 */
export function parseStackString(stack: string): Stacktrace {
	const lines = stack.split('\n');
	const frames: StackFrame[] = [];

	for (const line of lines) {
		const frame = parseFrame(line.trim());
		if (frame) {
			frames.push(frame);
		}
	}

	// Reverse to have most recent call last
	frames.reverse();

	return { frames };
}

/**
 * Extract the exception type from an error.
 */
export function getExceptionType(error: unknown): string {
	if (error instanceof Error) {
		return error.name || 'Error';
	}
	if (typeof error === 'string') {
		return 'Error';
	}
	if (error === null) {
		return 'null';
	}
	if (error === undefined) {
		return 'undefined';
	}
	return 'UnknownError';
}

/**
 * Extract the exception message from an error.
 */
export function getExceptionValue(error: unknown): string {
	if (error instanceof Error) {
		return error.message;
	}
	if (typeof error === 'string') {
		return error;
	}
	if (error === null) {
		return 'null';
	}
	if (error === undefined) {
		return 'undefined';
	}
	try {
		return String(error);
	} catch {
		return 'Unknown error';
	}
}

/**
 * Find the culprit frame (first in-app frame).
 */
export function findCulprit(stacktrace: Stacktrace): string | undefined {
	// Find the first in-app frame from the top of the stack
	for (let i = stacktrace.frames.length - 1; i >= 0; i--) {
		const frame = stacktrace.frames[i];
		if (frame.in_app && frame.function) {
			return frame.function;
		}
	}
	return undefined;
}
