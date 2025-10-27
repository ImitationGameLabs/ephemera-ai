<script lang="ts">
	import MessageList from '$lib/components/atrium/chat/MessageList.svelte';
	import MessageInput from '$lib/components/atrium/chat/MessageInput.svelte';
	import { CircleAlert } from '@lucide/svelte';
	import type { Message } from '$lib/api/types';
	import type { AuthenticatedUser } from '$lib/stores/auth';

	interface ChatInterfaceProps {
		messages: Message[];
		loading: boolean;
		error: string | null;
		initialLoadDone: boolean;
		sendingError: string | null;
		notifications: NotificationState;
		currentUser: AuthenticatedUser | null;
		isOffline?: boolean;
		onSendMessage: (content: string) => void;
		onRetryLoad: () => void;
		onClearNotifications: () => void;
	}

	interface NotificationState {
		count: number;
		hasUnloadedUnread: boolean;
	}

	let {
		messages = $bindable([]),
		loading = $bindable(false),
		error = $bindable(null),
		initialLoadDone = $bindable(false),
		sendingError = $bindable(null),
		notifications = $bindable({ count: 0, hasUnloadedUnread: false }),
		currentUser = $bindable(null),
		isOffline = $bindable(false),
		onSendMessage = $bindable(() => {}),
		onRetryLoad = $bindable(() => {}),
		onClearNotifications = $bindable(() => {}),
		classes = $bindable('')
	}: ChatInterfaceProps & { classes?: string } = $props();
</script>

<div class="{classes} flex flex-col relative min-h-0">
	<!-- Messages Area -->
	<div class="flex-1 flex min-h-0">
		{#if error}
			<div class="absolute inset-0 flex items-center justify-center bg-surface-50-950/50">
				<div class="text-center p-6">
					<CircleAlert class="w-12 h-12 text-error-500 mx-auto mb-4" />
					<p class="text-lg font-medium mb-2">Failed to load messages</p>
					<p class="text-sm text-surface-600-400 mb-4">{error}</p>
					{#if !isOffline}
						<button
							class="btn preset-filled-primary"
							onclick={onRetryLoad}
						>
							Retry
						</button>
					{/if}
				</div>
			</div>
		{:else}
			<!-- Loading more messages indicator (at top) -->
			{#if loading && initialLoadDone}
				<div class="sticky top-0 bg-surface-50-950/80 backdrop-blur-sm border-b border-surface-200-800 p-3 z-10">
					<div class="flex items-center justify-center gap-2">
						<div class="loading loading-spinner w-4 h-4"></div>
						<span class="text-sm text-surface-600-400">Loading older messages...</span>
					</div>
				</div>
			{/if}

			<MessageList
				class="flex-1"
				messages={messages}
				currentUser={currentUser}
				loading={loading && !initialLoadDone}
				onClearNotifications={onClearNotifications}
			/>
		{/if}
	</div>

	<!-- Message Input -->
	<div class="sticky bottom-0 bg-surface-50-950 border-t border-surface-200-800 p-4 z-10">
		{#if sendingError}
			<div class="max-w-4xl mx-auto mb-2 p-3 bg-error-100-900 border border-error-200-800 rounded-lg">
				<div class="flex items-center gap-2">
					<CircleAlert class="w-4 h-4 text-error-600-400" />
					<span class="text-sm text-error-700-300">{sendingError}</span>
				</div>
			</div>
		{/if}
		<MessageInput onSendMessage={onSendMessage} />
	</div>
</div>