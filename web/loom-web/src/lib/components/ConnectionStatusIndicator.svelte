<script lang="ts">
	import type { ConnectionStatus } from '../realtime/types';
	import { Badge } from '../ui';
	import { i18n } from '../i18n';

	interface Props {
		status: ConnectionStatus;
		attemptCount?: number;
		onreconnect?: () => void;
	}

	let { status, attemptCount = 0, onreconnect }: Props = $props();

	const statusConfig: Record<
		ConnectionStatus,
		{ labelKey: string; variant: 'success' | 'warning' | 'error' | 'muted'; dotClass: string }
	> = {
		connected: { labelKey: 'connection.connected', variant: 'success', dotClass: 'bg-green-500' },
		connecting: { labelKey: 'connection.connecting', variant: 'muted', dotClass: 'bg-gray-400' },
		disconnected: {
			labelKey: 'connection.disconnected',
			variant: 'error',
			dotClass: 'bg-red-500',
		},
		reconnecting: {
			labelKey: 'connection.reconnecting',
			variant: 'warning',
			dotClass: 'bg-yellow-500',
		},
		error: { labelKey: 'connection.error', variant: 'error', dotClass: 'bg-red-500' },
	};

	const config = $derived(statusConfig[status]);
	const isClickable = $derived(
		(status === 'disconnected' || status === 'error') && onreconnect !== undefined
	);
	const label = $derived(i18n.t(config.labelKey));
	const attemptLabel = $derived(
		attemptCount > 0 && status === 'reconnecting'
			? i18n.t('connection.attempt').replace('{count}', String(attemptCount))
			: null
	);
	const title = $derived(isClickable ? i18n.t('connection.clickToReconnect') : undefined);

	function handleClick() {
		if (isClickable && onreconnect) {
			onreconnect();
		}
	}
</script>

<Badge variant={config.variant} size="sm">
	{#if isClickable}
		<button
			type="button"
			class="inline-flex items-center gap-1.5 cursor-pointer bg-transparent border-none p-0 m-0 text-inherit font-inherit"
			onclick={handleClick}
			{title}
		>
			<span
				class="inline-block w-2 h-2 rounded-full {config.dotClass}"
			></span>
			<span>{label}</span>
		</button>
	{:else}
		<span class="inline-flex items-center gap-1.5">
			<span
				class="inline-block w-2 h-2 rounded-full {config.dotClass} {status === 'connecting' || status === 'reconnecting' ? 'animate-pulse' : ''}"
			></span>
			<span>{label}</span>
			{#if attemptLabel}
				<span class="text-xs opacity-75">({attemptLabel})</span>
			{/if}
		</span>
	{/if}
</Badge>
