<!--
  Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
  SPDX-License-Identifier: Proprietary
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { i18n } from '$lib/i18n';

	interface Props {
		weaverId: string;
		onDisconnect?: () => void;
	}

	let { weaverId, onDisconnect }: Props = $props();

	let terminalContainer: HTMLDivElement;
	let terminal: import('@xterm/xterm').Terminal | null = null;
	let fitAddon: import('@xterm/addon-fit').FitAddon | null = null;
	let ws: WebSocket | null = null;
	let keepAliveInterval: ReturnType<typeof setInterval> | null = null;

	const KEEPALIVE_INTERVAL_MS = 15000; // Send ping every 15 seconds

	let connectionStatus = $state<'connecting' | 'connected' | 'disconnected' | 'error'>('connecting');
	let errorMessage = $state<string | null>(null);

	function getWebSocketUrl(): string {
		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		return `${protocol}//${window.location.host}/api/weaver/${encodeURIComponent(weaverId)}/attach`;
	}

	async function initTerminal() {
		const { Terminal } = await import('@xterm/xterm');
		const { FitAddon } = await import('@xterm/addon-fit');
		const { WebLinksAddon } = await import('@xterm/addon-web-links');

		terminal = new Terminal({
			cursorBlink: true,
			fontSize: 14,
			fontFamily: 'Menlo, Monaco, "Courier New", monospace',
			theme: {
				background: '#1e1e1e',
				foreground: '#d4d4d4',
				cursor: '#d4d4d4',
				cursorAccent: '#1e1e1e',
				selectionBackground: '#264f78',
				black: '#000000',
				red: '#cd3131',
				green: '#0dbc79',
				yellow: '#e5e510',
				blue: '#2472c8',
				magenta: '#bc3fbc',
				cyan: '#11a8cd',
				white: '#e5e5e5',
				brightBlack: '#666666',
				brightRed: '#f14c4c',
				brightGreen: '#23d18b',
				brightYellow: '#f5f543',
				brightBlue: '#3b8eea',
				brightMagenta: '#d670d6',
				brightCyan: '#29b8db',
				brightWhite: '#ffffff',
			},
		});

		fitAddon = new FitAddon();
		terminal.loadAddon(fitAddon);
		terminal.loadAddon(new WebLinksAddon());

		terminal.open(terminalContainer);
		fitAddon.fit();

		terminal.onData((data) => {
			if (ws && ws.readyState === WebSocket.OPEN) {
				ws.send(data);
			}
		});

		terminal.onBinary((data) => {
			if (ws && ws.readyState === WebSocket.OPEN) {
				const buffer = new Uint8Array(data.length);
				for (let i = 0; i < data.length; i++) {
					buffer[i] = data.charCodeAt(i);
				}
				ws.send(buffer);
			}
		});

		connectWebSocket();
	}

	function connectWebSocket() {
		const url = getWebSocketUrl();
		connectionStatus = 'connecting';
		errorMessage = null;

		ws = new WebSocket(url);
		ws.binaryType = 'arraybuffer';

		ws.onopen = () => {
			connectionStatus = 'connected';
			terminal?.focus();
			startKeepAlive();
			// Send Ctrl+L to refresh the terminal display
			// This triggers a redraw so user sees current PTY state
			sendTerminalRefresh();
		};

		ws.onmessage = (event) => {
			if (event.data instanceof ArrayBuffer) {
				const decoder = new TextDecoder();
				terminal?.write(decoder.decode(event.data));
			} else if (typeof event.data === 'string') {
				terminal?.write(event.data);
			}
		};

		ws.onclose = (event) => {
			connectionStatus = 'disconnected';
			stopKeepAlive();
			if (event.code !== 1000) {
				errorMessage = `Connection closed: ${event.reason || 'Unknown reason'}`;
			}
			onDisconnect?.();
		};

		ws.onerror = () => {
			connectionStatus = 'error';
			stopKeepAlive();
			errorMessage = 'Failed to connect to weaver';
		};
	}

	function sendTerminalRefresh() {
		if (ws && ws.readyState === WebSocket.OPEN) {
			// Send Ctrl+L (ASCII 12 = Form Feed) to trigger terminal redraw
			ws.send(new Uint8Array([12]));
		}
	}

	function startKeepAlive() {
		stopKeepAlive();
		keepAliveInterval = setInterval(() => {
			if (ws && ws.readyState === WebSocket.OPEN) {
				// Send empty ping frame to keep connection alive
				ws.send(new Uint8Array(0));
			}
		}, KEEPALIVE_INTERVAL_MS);
	}

	function stopKeepAlive() {
		if (keepAliveInterval) {
			clearInterval(keepAliveInterval);
			keepAliveInterval = null;
		}
	}

	function reconnect() {
		if (ws) {
			ws.close();
		}
		terminal?.clear();
		connectWebSocket();
	}

	function handleResize() {
		if (fitAddon) {
			fitAddon.fit();
		}
	}

	onMount(() => {
		initTerminal();

		const resizeObserver = new ResizeObserver(() => {
			handleResize();
		});
		resizeObserver.observe(terminalContainer);

		return () => {
			resizeObserver.disconnect();
			stopKeepAlive();
			if (ws) {
				ws.close();
			}
			if (terminal) {
				terminal.dispose();
			}
		};
	});
</script>

<svelte:head>
	<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@xterm/xterm@5.5.0/css/xterm.min.css" />
</svelte:head>

<div class="flex flex-col h-full">
	<div class="flex items-center justify-between px-3 py-2 bg-gray-800 border-b border-gray-700">
		<div class="flex items-center gap-2">
			<div
				class="w-2 h-2 rounded-full"
				class:bg-green-500={connectionStatus === 'connected'}
				class:bg-yellow-500={connectionStatus === 'connecting'}
				class:bg-red-500={connectionStatus === 'error' || connectionStatus === 'disconnected'}
			></div>
			<span class="text-sm text-gray-300">
				{#if connectionStatus === 'connected'}
					{i18n._('weavers.terminal.connected')}
				{:else if connectionStatus === 'connecting'}
					{i18n._('weavers.terminal.connecting')}
				{:else if connectionStatus === 'error'}
					{i18n._('weavers.terminal.error')}
				{:else}
					{i18n._('weavers.terminal.disconnected')}
				{/if}
			</span>
		</div>
		{#if connectionStatus === 'disconnected' || connectionStatus === 'error'}
			<button
				onclick={reconnect}
				class="px-2 py-1 text-xs bg-gray-700 hover:bg-gray-600 text-gray-200 rounded"
			>
				{i18n._('weavers.terminal.reconnect')}
			</button>
		{/if}
	</div>

	{#if errorMessage}
		<div class="px-3 py-2 bg-red-900/50 text-red-200 text-sm">
			{errorMessage}
		</div>
	{/if}

	<div
		bind:this={terminalContainer}
		class="flex-1 min-h-0"
		style="background: #1e1e1e;"
	></div>
</div>

<style>
	:global(.xterm) {
		height: 100%;
		padding: 8px;
	}
	:global(.xterm-viewport) {
		overflow-y: auto !important;
	}
</style>
