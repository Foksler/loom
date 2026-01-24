/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

import { logger } from '../logging';

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

// Crons SSE Events
export interface CronInitEvent {
	type: 'init';
	monitors: MonitorState[];
}

export interface MonitorState {
	id: string;
	slug: string;
	name: string;
	status: 'active' | 'paused';
	health: 'healthy' | 'failing' | 'missed' | 'timeout' | 'unknown';
	last_checkin_at: string | null;
	next_expected_at: string | null;
}

export interface CronCheckinEvent {
	type: 'checkin_started' | 'checkin_ok' | 'checkin_error';
	monitor_id: string;
	monitor_slug: string;
	checkin_id: string;
	status?: string;
	duration_ms?: number;
}

export interface CronMonitorMissedEvent {
	type: 'monitor_missed';
	monitor_id: string;
	monitor_slug: string;
	expected_at: string;
}

export interface CronMonitorUpdatedEvent {
	type: 'monitor_updated' | 'monitor_created' | 'monitor_deleted';
	monitor_id: string;
	monitor_slug: string;
}

export type CronEvent =
	| CronInitEvent
	| CronCheckinEvent
	| CronMonitorMissedEvent
	| CronMonitorUpdatedEvent;

// Crash SSE Events
export interface CrashInitEvent {
	type: 'init';
	issues: IssueState[];
}

export interface IssueState {
	id: string;
	short_id: string;
	title: string;
	status: 'unresolved' | 'resolved' | 'ignored';
	first_seen: string;
	last_seen: string;
	event_count: number;
	user_count: number;
}

export interface CrashNewIssueEvent {
	type: 'issue.new';
	issue: IssueState;
}

export interface CrashIssueRegressedEvent {
	type: 'issue.regressed';
	issue_id: string;
	short_id: string;
	title: string;
}

export interface CrashIssueResolvedEvent {
	type: 'issue.resolved';
	issue_id: string;
	short_id: string;
}

export interface CrashNewEventEvent {
	type: 'event.new';
	issue_id: string;
	event_id: string;
}

export type CrashEvent =
	| CrashInitEvent
	| CrashNewIssueEvent
	| CrashIssueRegressedEvent
	| CrashIssueResolvedEvent
	| CrashNewEventEvent;

// Event Handlers
export type CronEventHandler = (event: CronEvent) => void;
export type CrashEventHandler = (event: CrashEvent) => void;
export type StatusHandler = (status: ConnectionStatus) => void;

/**
 * SSE client for cron monitoring events
 */
export class CronsSSEClient {
	private eventSource: EventSource | null = null;
	private handlers = new Set<CronEventHandler>();
	private statusHandlers = new Set<StatusHandler>();
	private status: ConnectionStatus = 'disconnected';
	private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
	private reconnectAttempts = 0;
	private maxReconnectAttempts = 5;

	constructor(
		private serverUrl: string,
		private orgId: string
	) {}

	connect(): void {
		if (this.eventSource) {
			return;
		}

		this.setStatus('connecting');
		this.reconnectAttempts++;

		const url = `${this.serverUrl}/api/crons/stream?org_id=${encodeURIComponent(this.orgId)}`;

		try {
			this.eventSource = new EventSource(url, { withCredentials: true });

			this.eventSource.onopen = () => {
				logger.info('Crons SSE connected', { orgId: this.orgId });
				this.setStatus('connected');
				this.reconnectAttempts = 0;
			};

			this.eventSource.onerror = (e) => {
				logger.error('Crons SSE error', { orgId: this.orgId, error: e });
				this.setStatus('error');
				this.scheduleReconnect();
			};

			// Listen for all event types
			for (const eventType of [
				'init',
				'checkin_started',
				'checkin_ok',
				'checkin_error',
				'monitor_missed',
				'monitor_updated',
				'monitor_created',
				'monitor_deleted',
			]) {
				this.eventSource.addEventListener(eventType, (e) => {
					this.handleEvent(eventType, e as MessageEvent);
				});
			}
		} catch (error) {
			logger.error('Crons SSE connection failed', { orgId: this.orgId, error: String(error) });
			this.setStatus('error');
			this.scheduleReconnect();
		}
	}

	disconnect(): void {
		if (this.reconnectTimeout) {
			clearTimeout(this.reconnectTimeout);
			this.reconnectTimeout = null;
		}
		if (this.eventSource) {
			this.eventSource.close();
			this.eventSource = null;
		}
		this.setStatus('disconnected');
		this.reconnectAttempts = 0;
	}

	onEvent(handler: CronEventHandler): () => void {
		this.handlers.add(handler);
		return () => this.handlers.delete(handler);
	}

	onStatus(handler: StatusHandler): () => void {
		this.statusHandlers.add(handler);
		return () => this.statusHandlers.delete(handler);
	}

	getStatus(): ConnectionStatus {
		return this.status;
	}

	private setStatus(status: ConnectionStatus): void {
		this.status = status;
		this.statusHandlers.forEach((h) => h(status));
	}

	private handleEvent(eventType: string, event: MessageEvent): void {
		try {
			const data = JSON.parse(event.data);
			const cronEvent: CronEvent = { type: eventType, ...data };
			this.handlers.forEach((h) => h(cronEvent));
		} catch (error) {
			logger.error('Failed to parse crons SSE event', { eventType, error: String(error) });
		}
	}

	private scheduleReconnect(): void {
		if (this.reconnectAttempts >= this.maxReconnectAttempts) {
			logger.warn('Max reconnect attempts reached for crons SSE', { orgId: this.orgId });
			return;
		}

		if (this.eventSource) {
			this.eventSource.close();
			this.eventSource = null;
		}

		const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
		logger.info('Scheduling crons SSE reconnect', {
			orgId: this.orgId,
			attempt: this.reconnectAttempts,
			delay,
		});

		this.reconnectTimeout = setTimeout(() => {
			this.reconnectTimeout = null;
			this.connect();
		}, delay);
	}
}

/**
 * SSE client for crash events
 */
export class CrashSSEClient {
	private eventSource: EventSource | null = null;
	private handlers = new Set<CrashEventHandler>();
	private statusHandlers = new Set<StatusHandler>();
	private status: ConnectionStatus = 'disconnected';
	private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
	private reconnectAttempts = 0;
	private maxReconnectAttempts = 5;

	constructor(
		private serverUrl: string,
		private projectId: string
	) {}

	connect(): void {
		if (this.eventSource) {
			return;
		}

		this.setStatus('connecting');
		this.reconnectAttempts++;

		const url = `${this.serverUrl}/api/crash/projects/${encodeURIComponent(this.projectId)}/stream`;

		try {
			this.eventSource = new EventSource(url, { withCredentials: true });

			this.eventSource.onopen = () => {
				logger.info('Crash SSE connected', { projectId: this.projectId });
				this.setStatus('connected');
				this.reconnectAttempts = 0;
			};

			this.eventSource.onerror = (e) => {
				logger.error('Crash SSE error', { projectId: this.projectId, error: e });
				this.setStatus('error');
				this.scheduleReconnect();
			};

			// Listen for all event types
			for (const eventType of [
				'init',
				'issue.new',
				'issue.regressed',
				'issue.resolved',
				'event.new',
			]) {
				this.eventSource.addEventListener(eventType, (e) => {
					this.handleEvent(eventType, e as MessageEvent);
				});
			}
		} catch (error) {
			logger.error('Crash SSE connection failed', {
				projectId: this.projectId,
				error: String(error),
			});
			this.setStatus('error');
			this.scheduleReconnect();
		}
	}

	disconnect(): void {
		if (this.reconnectTimeout) {
			clearTimeout(this.reconnectTimeout);
			this.reconnectTimeout = null;
		}
		if (this.eventSource) {
			this.eventSource.close();
			this.eventSource = null;
		}
		this.setStatus('disconnected');
		this.reconnectAttempts = 0;
	}

	onEvent(handler: CrashEventHandler): () => void {
		this.handlers.add(handler);
		return () => this.handlers.delete(handler);
	}

	onStatus(handler: StatusHandler): () => void {
		this.statusHandlers.add(handler);
		return () => this.statusHandlers.delete(handler);
	}

	getStatus(): ConnectionStatus {
		return this.status;
	}

	private setStatus(status: ConnectionStatus): void {
		this.status = status;
		this.statusHandlers.forEach((h) => h(status));
	}

	private handleEvent(eventType: string, event: MessageEvent): void {
		try {
			const data = JSON.parse(event.data);
			const crashEvent: CrashEvent = { type: eventType as CrashEvent['type'], ...data };
			this.handlers.forEach((h) => h(crashEvent));
		} catch (error) {
			logger.error('Failed to parse crash SSE event', { eventType, error: String(error) });
		}
	}

	private scheduleReconnect(): void {
		if (this.reconnectAttempts >= this.maxReconnectAttempts) {
			logger.warn('Max reconnect attempts reached for crash SSE', { projectId: this.projectId });
			return;
		}

		if (this.eventSource) {
			this.eventSource.close();
			this.eventSource = null;
		}

		const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
		logger.info('Scheduling crash SSE reconnect', {
			projectId: this.projectId,
			attempt: this.reconnectAttempts,
			delay,
		});

		this.reconnectTimeout = setTimeout(() => {
			this.reconnectTimeout = null;
			this.connect();
		}, delay);
	}
}
