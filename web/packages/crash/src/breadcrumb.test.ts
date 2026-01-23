/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
	BreadcrumbManager,
	httpBreadcrumb,
	navigationBreadcrumb,
	uiBreadcrumb,
	consoleBreadcrumb,
	userBreadcrumb,
	debugBreadcrumb
} from './breadcrumb';

describe('BreadcrumbManager', () => {
	let manager: BreadcrumbManager;

	beforeEach(() => {
		manager = new BreadcrumbManager(5);
	});

	it('should add breadcrumbs', () => {
		manager.add({ category: 'test', message: 'test message' });

		expect(manager.count).toBe(1);
		expect(manager.getAll()[0].message).toBe('test message');
	});

	it('should add timestamp to breadcrumbs', () => {
		manager.add({ category: 'test', message: 'test message' });

		const crumb = manager.getAll()[0];
		expect(crumb.timestamp).toBeDefined();
		expect(new Date(crumb.timestamp!).getTime()).toBeLessThanOrEqual(Date.now());
	});

	it('should maintain max size', () => {
		for (let i = 0; i < 10; i++) {
			manager.add({ category: 'test', message: `message ${i}` });
		}

		expect(manager.count).toBe(5);
		// Oldest messages should be dropped
		expect(manager.getAll()[0].message).toBe('message 5');
		expect(manager.getAll()[4].message).toBe('message 9');
	});

	it('should clear all breadcrumbs', () => {
		manager.add({ category: 'test', message: 'test' });
		manager.add({ category: 'test', message: 'test2' });

		manager.clear();

		expect(manager.count).toBe(0);
		expect(manager.getAll()).toEqual([]);
	});

	it('should return a copy of breadcrumbs', () => {
		manager.add({ category: 'test', message: 'test' });

		const crumbs = manager.getAll();
		crumbs.push({ category: 'fake', message: 'fake' });

		expect(manager.count).toBe(1);
	});
});

describe('breadcrumb helpers', () => {
	describe('httpBreadcrumb', () => {
		it('should create an HTTP breadcrumb', () => {
			const crumb = httpBreadcrumb('GET', '/api/users', 200);

			expect(crumb.type).toBe('http');
			expect(crumb.category).toBe('http');
			expect(crumb.message).toBe('GET /api/users');
			expect(crumb.level).toBe('info');
			expect(crumb.data?.method).toBe('GET');
			expect(crumb.data?.url).toBe('/api/users');
			expect(crumb.data?.status_code).toBe(200);
		});

		it('should mark 4xx/5xx as error', () => {
			const crumb404 = httpBreadcrumb('GET', '/api/missing', 404);
			const crumb500 = httpBreadcrumb('POST', '/api/error', 500);

			expect(crumb404.level).toBe('error');
			expect(crumb500.level).toBe('error');
		});
	});

	describe('navigationBreadcrumb', () => {
		it('should create a navigation breadcrumb', () => {
			const crumb = navigationBreadcrumb('/home', '/dashboard');

			expect(crumb.type).toBe('navigation');
			expect(crumb.category).toBe('navigation');
			expect(crumb.message).toBe('Navigated from /home to /dashboard');
			expect(crumb.data?.from).toBe('/home');
			expect(crumb.data?.to).toBe('/dashboard');
		});
	});

	describe('uiBreadcrumb', () => {
		it('should create a UI breadcrumb', () => {
			const crumb = uiBreadcrumb('button.submit', 'click');

			expect(crumb.type).toBe('ui');
			expect(crumb.category).toBe('ui.click');
			expect(crumb.message).toBe('click on button.submit');
		});

		it('should default to click action', () => {
			const crumb = uiBreadcrumb('input.email');

			expect(crumb.category).toBe('ui.click');
		});
	});

	describe('consoleBreadcrumb', () => {
		it('should create a console breadcrumb', () => {
			const crumb = consoleBreadcrumb('error', 'Something went wrong', ['arg1', 'arg2']);

			expect(crumb.type).toBe('console');
			expect(crumb.category).toBe('console');
			expect(crumb.message).toBe('Something went wrong');
			expect(crumb.level).toBe('error');
			expect(crumb.data?.arguments).toEqual(['arg1', 'arg2']);
		});

		it('should map console levels correctly', () => {
			expect(consoleBreadcrumb('error', 'msg').level).toBe('error');
			expect(consoleBreadcrumb('warn', 'msg').level).toBe('warning');
			expect(consoleBreadcrumb('log', 'msg').level).toBe('info');
			expect(consoleBreadcrumb('info', 'msg').level).toBe('info');
			expect(consoleBreadcrumb('debug', 'msg').level).toBe('info');
		});
	});

	describe('userBreadcrumb', () => {
		it('should create a user action breadcrumb', () => {
			const crumb = userBreadcrumb('auth', 'User logged in', { userId: '123' });

			expect(crumb.type).toBe('user');
			expect(crumb.category).toBe('auth');
			expect(crumb.message).toBe('User logged in');
			expect(crumb.data?.userId).toBe('123');
		});
	});

	describe('debugBreadcrumb', () => {
		it('should create a debug breadcrumb', () => {
			const crumb = debugBreadcrumb('Processing started', { step: 1 });

			expect(crumb.type).toBe('debug');
			expect(crumb.category).toBe('debug');
			expect(crumb.level).toBe('debug');
			expect(crumb.message).toBe('Processing started');
		});
	});
});
