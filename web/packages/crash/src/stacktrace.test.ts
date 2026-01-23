/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { describe, it, expect } from 'vitest';
import {
	parseStackTrace,
	parseStackString,
	getExceptionType,
	getExceptionValue,
	findCulprit
} from './stacktrace';

describe('parseStackTrace', () => {
	it('should parse Chrome/V8 stack traces', () => {
		const error = new Error('Test error');
		error.stack = `Error: Test error
    at myFunction (https://example.com/app.js:10:15)
    at handleClick (https://example.com/app.js:20:5)
    at HTMLButtonElement.onclick (https://example.com/app.js:30:1)`;

		const result = parseStackTrace(error);

		expect(result.frames.length).toBe(3);
		// Frames are reversed (most recent call last)
		expect(result.frames[2].function).toBe('myFunction');
		expect(result.frames[2].filename).toBe('https://example.com/app.js');
		expect(result.frames[2].lineno).toBe(10);
		expect(result.frames[2].colno).toBe(15);
		expect(result.frames[2].in_app).toBe(true);
	});

	it('should parse Firefox stack traces', () => {
		const error = new Error('Test error');
		error.stack = `myFunction@https://example.com/app.js:10:15
handleClick@https://example.com/app.js:20:5
onclick@https://example.com/app.js:30:1`;

		const result = parseStackTrace(error);

		expect(result.frames.length).toBe(3);
		expect(result.frames[2].function).toBe('myFunction');
		expect(result.frames[2].filename).toBe('https://example.com/app.js');
		expect(result.frames[2].lineno).toBe(10);
	});

	it('should mark node_modules as not in_app', () => {
		const error = new Error('Test error');
		error.stack = `Error: Test error
    at myFunction (https://example.com/app.js:10:15)
    at libraryFunction (https://example.com/node_modules/lib/index.js:5:10)`;

		const result = parseStackTrace(error);

		expect(result.frames[1].in_app).toBe(true);
		expect(result.frames[0].in_app).toBe(false);
	});

	it('should handle empty stack', () => {
		const error = new Error('Test error');
		error.stack = '';

		const result = parseStackTrace(error);

		expect(result.frames).toEqual([]);
	});

	it('should handle undefined stack', () => {
		const error = new Error('Test error');
		delete (error as unknown as Record<string, unknown>).stack;

		const result = parseStackTrace(error);

		expect(result.frames).toEqual([]);
	});
});

describe('parseStackString', () => {
	it('should parse a raw stack string', () => {
		const stack = `    at myFunction (https://example.com/app.js:10:15)
    at handleClick (https://example.com/app.js:20:5)`;

		const result = parseStackString(stack);

		expect(result.frames.length).toBe(2);
		expect(result.frames[1].function).toBe('myFunction');
	});
});

describe('getExceptionType', () => {
	it('should return Error name for Error objects', () => {
		expect(getExceptionType(new Error('test'))).toBe('Error');
		expect(getExceptionType(new TypeError('test'))).toBe('TypeError');
		expect(getExceptionType(new ReferenceError('test'))).toBe('ReferenceError');
	});

	it('should return "Error" for strings', () => {
		expect(getExceptionType('something went wrong')).toBe('Error');
	});

	it('should return "null" for null', () => {
		expect(getExceptionType(null)).toBe('null');
	});

	it('should return "undefined" for undefined', () => {
		expect(getExceptionType(undefined)).toBe('undefined');
	});

	it('should return "UnknownError" for other types', () => {
		expect(getExceptionType({ error: true })).toBe('UnknownError');
		expect(getExceptionType(123)).toBe('UnknownError');
	});
});

describe('getExceptionValue', () => {
	it('should return message for Error objects', () => {
		expect(getExceptionValue(new Error('test message'))).toBe('test message');
	});

	it('should return the string itself for strings', () => {
		expect(getExceptionValue('error message')).toBe('error message');
	});

	it('should return "null" for null', () => {
		expect(getExceptionValue(null)).toBe('null');
	});

	it('should return "undefined" for undefined', () => {
		expect(getExceptionValue(undefined)).toBe('undefined');
	});

	it('should stringify other types', () => {
		expect(getExceptionValue(123)).toBe('123');
		expect(getExceptionValue({ error: true })).toBe('[object Object]');
	});
});

describe('findCulprit', () => {
	it('should find the first in_app frame function', () => {
		const stacktrace = {
			frames: [
				{ function: 'libraryFn', in_app: false },
				{ function: 'myFunction', in_app: true },
				{ function: 'anotherFn', in_app: true }
			]
		};

		expect(findCulprit(stacktrace)).toBe('anotherFn');
	});

	it('should return undefined if no in_app frames', () => {
		const stacktrace = {
			frames: [
				{ function: 'libraryFn', in_app: false },
				{ function: 'anotherLib', in_app: false }
			]
		};

		expect(findCulprit(stacktrace)).toBeUndefined();
	});

	it('should return undefined for empty frames', () => {
		expect(findCulprit({ frames: [] })).toBeUndefined();
	});
});
