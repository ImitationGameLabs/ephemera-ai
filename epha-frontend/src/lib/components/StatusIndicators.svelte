<script lang="ts">
	import { Circle, CircleCheck, AlertCircle, Loader2 } from '@lucide/svelte';
	import type { User } from '$lib/api/types';
	import { heartbeatManager } from '$lib/services/heartbeat';
	import type { HeartbeatStatus } from '$lib/config/heartbeat';

	let {
		currentUser = $bindable(null as User | null),
		messages = $bindable([] as any[]),
		loading = $bindable(false)
	} = $props();

	// Calculate unread count
	const unreadCount = $derived(() => {
		if (!currentUser || messages.length === 0) return 0;
		return messages.filter(msg => msg.id > currentUser.message_height).length;
	});

	// Connection status based on heartbeat
	let currentHeartbeatStatus = $state<HeartbeatStatus>('disconnected');
	heartbeatManager.status.subscribe(value => currentHeartbeatStatus = value);

	// Combined connection status
	const isConnected = $derived(currentUser !== null && currentHeartbeatStatus === 'connected');

	// Get connection status display text
	const connectionStatusText = $derived.by(() => {
		switch (currentHeartbeatStatus) {
			case 'connected': return 'Connected';
			case 'connecting': return 'Connecting...';
			case 'disconnected': return 'Disconnected';
			case 'error': return 'Connection Error';
			default: return 'Unknown';
		}
	});

	// Get connection status color
	const connectionStatusColor = $derived.by(() => {
		switch (currentHeartbeatStatus) {
			case 'connected': return 'bg-green-500';
			case 'connecting': return 'bg-yellow-500';
			case 'disconnected': return 'bg-red-500';
			case 'error': return 'bg-red-600';
			default: return 'bg-gray-500';
		}
	});

	// Get connection status icon
	const ConnectionIcon = $derived.by(() => {
		switch (currentHeartbeatStatus) {
			case 'connecting': return Loader2;
			case 'error': return AlertCircle;
			default: return Circle;
		}
	});

	// Format unread count for display
	const displayUnreadCount = $derived(() => {
		const count = unreadCount();
		return count > 99 ? '99+' : count.toString();
	});
</script>

<div class="flex items-center gap-4">
	<!-- Connection Status -->
	<div class="flex items-center gap-2">
		{#if currentHeartbeatStatus === 'connecting'}
			<ConnectionIcon class="w-4 h-4 text-yellow-500 animate-spin" />
		{:else}
			<div class="w-2 h-2 rounded-full {connectionStatusColor}"
				title={connectionStatusText}>
			</div>
		{/if}
		<span class="text-sm text-surface-600-400">
			{connectionStatusText}
		</span>
	</div>

	<!-- Unread Messages Indicator -->
	{#if unreadCount() > 0}
		<div class="flex items-center gap-2">
			<CircleCheck class="w-4 h-4 text-primary-500" />
			<span class="text-sm text-surface-700-300">
				{unreadCount()} unread message{unreadCount() !== 1 ? 's' : ''}
			</span>
		</div>
	{/if}

	<!-- Loading Status -->
	{#if loading}
		<div class="flex items-center gap-2">
			<div class="w-4 h-4">
				<div class="loading loading-spinner w-4 h-4"></div>
			</div>
			<span class="text-sm text-surface-600-400">Loading...</span>
		</div>
	{/if}
</div>