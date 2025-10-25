<script lang="ts">
	import MessageList from '$lib/components/atrium/chat/MessageList.svelte';
	import MessageInput from '$lib/components/atrium/chat/MessageInput.svelte';
	import { CircleAlert, MessageSquare, Users } from '@lucide/svelte';
	import type { ChatInterfaceProps } from '$lib/types/chat';
	import { chatScrollManager } from '$lib/actions/chatScrollManager';

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
		onClearNotifications = $bindable(() => {})
	}: ChatInterfaceProps = $props();

	let messagesContainer = $state<HTMLElement>();
	let showScrollToBottom = $state(false);

	// Handle scroll events for loading more messages and scroll-to-bottom button
	function handleScroll() {
		if (!messagesContainer) return;

		const { scrollTop, scrollHeight, clientHeight } = messagesContainer;
		const distanceFromBottom = scrollHeight - scrollTop - clientHeight;

		// Show scroll to bottom button if not near bottom
		showScrollToBottom = distanceFromBottom > 200;

		// Clear notifications if user scrolls to bottom (viewing latest messages)
		if (distanceFromBottom < 10) {
			onClearNotifications();
		}
	}

	// Scroll to bottom (exposed for external use)
	function scrollToBottom() {
		if (messagesContainer) {
			messagesContainer.scrollTop = messagesContainer.scrollHeight;
		}
	}
</script>

<div class="flex-1 flex flex-col max-w-4xl mx-auto h-full w-full min-h-0">
	<!-- Messages Container -->
	<div class="flex-1 relative min-h-0">
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
			<div
				bind:this={messagesContainer}
				class="h-full overflow-auto min-h-0"
				onscroll={handleScroll}
			>
				{#if loading && !initialLoadDone}
					<div class="flex items-center justify-center h-full">
						<div class="text-center">
							<div class="loading loading-spinner w-8 h-8 mx-auto mb-4"></div>
							<p class="text-surface-600-400">Loading messages...</p>
						</div>
					</div>
				{:else if messages.length === 0}
					<div class="flex items-center justify-center h-full">
						<div class="text-center max-w-md mx-6">
							<div class="w-16 h-16 bg-surface-100-900 rounded-full flex items-center justify-center mx-auto mb-4">
								<MessageSquare class="w-8 h-8 text-surface-500" />
							</div>
							<h3 class="text-lg font-medium mb-2">No messages yet</h3>
							<p class="text-sm text-surface-600-400">
								Be the first to start the conversation!
							</p>
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
						messages={messages}
						currentUser={currentUser}
					/>
				{/if}
			</div>

			<!-- Scroll to bottom button -->
			{#if showScrollToBottom}
				<button
					class="absolute bottom-4 right-4 btn preset-filled-primary rounded-full w-12 h-12 p-0 flex items-center justify-center shadow-lg"
					onclick={scrollToBottom}
					title="Scroll to bottom"
				>
					<Users class="w-5 h-5" />
				</button>
			{/if}
		{/if}
	</div>

	<!-- Message Input -->
	<div>
		{#if sendingError}
			<div class="mx-4 mb-2 p-3 bg-error-100-900 border border-error-200-800 rounded-lg">
				<div class="flex items-center gap-2">
					<CircleAlert class="w-4 h-4 text-error-600-400" />
					<span class="text-sm text-error-700-300">{sendingError}</span>
				</div>
			</div>
		{/if}
		<MessageInput onSendMessage={onSendMessage} />
	</div>
</div>