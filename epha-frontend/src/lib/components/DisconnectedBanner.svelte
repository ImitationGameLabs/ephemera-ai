<script lang="ts">
	import { UI_CONFIG } from '$lib/config/app';

	interface Props {
		onRetry: () => void;
	}

	let { onRetry }: Props = $props();

	let isRetrying = $state(false);

	async function handleRetry() {
		isRetrying = true;
		await onRetry();
		setTimeout(() => isRetrying = false, UI_CONFIG.RETRY_BUTTON_DELAY);
	}
</script>

<div class="bg-yellow-500/20 dark:bg-yellow-500/10 border-b border-yellow-500/50 px-4 py-3 flex items-center justify-between">
	<div class="flex items-center gap-3 text-yellow-700 dark:text-yellow-300">
		<svg class="w-5 h-5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
		</svg>
		<div class="text-sm">
			<div class="font-medium">You're offline</div>
			<div class="text-xs opacity-80">Chat history is available. New messages will be sent when connection is restored.</div>
		</div>
	</div>
	<button
		onclick={handleRetry}
		disabled={isRetrying}
		class="flex items-center gap-2 px-3 py-1.5 text-sm bg-yellow-500/30 hover:bg-yellow-500/50 dark:bg-yellow-500/20 dark:hover:bg-yellow-500/30 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
	>
		<svg class="w-4 h-4" class:animate-spin={isRetrying} fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
		</svg>
		{isRetrying ? 'Reconnecting...' : 'Retry Now'}
	</button>
</div>