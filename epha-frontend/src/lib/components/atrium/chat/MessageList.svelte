<script lang="ts">
	import type { Message, User } from '$lib/api/types';
	import { Circle, CircleCheck } from '@lucide/svelte';
	import { messagesStore } from '$lib/stores/messages';
	import { useIntersectionObserver } from '$lib/actions/useIntersectionObserver';

	interface MessageGroup {
		user: string;
		time: string;
		messages: Message[];
		isOwn: boolean;
	}

	let { messages = $bindable([]), currentUser = $bindable(null), onScrollToTop = $bindable(() => {}) } = $props();

	let messageContainer = $state<HTMLElement>();
	let visibleMessageIds = $state(new Set<number>());

	// Handle message visibility changes for read receipts
	function handleMessageVisible(element: Element) {
		if (!currentUser?.credentials) return;

		const messageId = parseInt(element.getAttribute('data-message-id') || '0');
		visibleMessageIds = new Set([...visibleMessageIds, messageId]);

		// Mark visible messages as read
		if (messageId > currentUser.user.message_height) {
			messagesStore.markAsRead(messageId, currentUser.credentials);
		}
	}

	function handleMessageHidden(element: Element) {
		const messageId = parseInt(element.getAttribute('data-message-id') || '0');
		const newIds = new Set(visibleMessageIds);
		newIds.delete(messageId);
		visibleMessageIds = newIds;
	}

	
	function formatTime(timestamp: string): string {
		return new Date(timestamp).toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function formatDate(timestamp: string): string {
		const date = new Date(timestamp);
		const today = new Date();
		const yesterday = new Date(today);
		yesterday.setDate(yesterday.getDate() - 1);

		if (date.toDateString() === today.toDateString()) {
			return 'Today';
		} else if (date.toDateString() === yesterday.toDateString()) {
			return 'Yesterday';
		} else {
			return date.toLocaleDateString();
		}
	}

	function groupMessagesByUserAndTime(messages: Message[]): MessageGroup[] {
		const groups: MessageGroup[] = [];

		for (const message of messages) {
			const isOwn = currentUser && message.sender === currentUser.name;
			const messageTime = new Date(message.created_at).getTime();

			// Check if we can add this message to the previous group
			const lastGroup = groups[groups.length - 1];

			if (lastGroup &&
				lastGroup.user === message.sender &&
				lastGroup.isOwn === isOwn &&
				(messageTime - new Date(lastGroup.time).getTime()) < 2 * 60 * 1000) { // 2 minutes
				// Add to existing group
				lastGroup.messages.push(message);
			} else {
				// Create new group
				groups.push({
					user: message.sender,
					time: message.created_at,
					messages: [message],
					isOwn
				});
			}
		}

		return groups;
	}

	const messageGroups = $derived(groupMessagesByUserAndTime(messages));

	function findUnreadIndex(messages: Message[], currentUser: User | null): number {
		if (!currentUser) return -1;

		const messageHeight = currentUser.message_height;
		return messages.findIndex(msg => msg.id > messageHeight);
	}

	const unreadIndex = $derived(findUnreadIndex(messages, currentUser));
</script>

{#if messages.length === 0}
	<div class="flex items-center justify-center h-full">
		<div class="text-center">
			<p class="text-lg mb-2">No messages yet</p>
			<p class="text-sm">Be the first to start the conversation!</p>
		</div>
	</div>
{:else}
	<div
		bind:this={messageContainer}
		class="h-full scroll-auto min-h-0"
		use:useIntersectionObserver={{
			onVisible: handleMessageVisible,
			onHidden: handleMessageHidden
		}}
	>
		<div class="space-y-4 p-4">
			{#each messageGroups as group, groupIndex}
				<!-- Check for unread divider before this group -->
				{@const firstGlobalIndex = messages.indexOf(group.messages[0])}
				{@const isUnread = firstGlobalIndex === unreadIndex}

				{#if isUnread}
					<div class="flex items-center justify-center my-4">
						<div class="flex items-center gap-2 text-xs text-surface-500">
							<div class="h-px bg-surface-300-700 flex-1"></div>
							<div class="flex items-center gap-1">
								<CircleCheck class="w-3 h-3" />
								<span>Unread messages</span>
							</div>
							<div class="h-px bg-surface-300-700 flex-1"></div>
						</div>
					</div>
				{/if}

				<!-- Message Group -->
				<div class="flex {group.isOwn ? 'justify-end' : 'justify-start'} mb-4">
					<div class="max-w-xs lg:max-w-md">
						<!-- Group Header: User name and timestamp (shown once per group) -->
						<div class="flex items-center gap-2 mb-1 px-2">
							{#if !group.isOwn}
								<Circle class="w-2 h-2 text-green-500 fill-green-500" />
							{/if}
							<span class="text-xs font-medium">
								{group.user}
							</span>
							<span class="text-xs text-surface-500">
								{formatTime(group.time)}
							</span>
						</div>

						<!-- Message Bubbles in this group -->
						{#each group.messages as message, messageIndex}
							<div class="mb-2" data-message-id={message.id}>
								<!-- Message Content -->
								<div class="rounded-2xl px-4 py-2 {group.isOwn
									? 'bg-primary-500 text-white rounded-br-sm'
									: 'bg-surface-200-800 rounded-bl-sm'}">
									<p class="text-sm whitespace-pre-wrap break-words">
										{message.content}
									</p>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}