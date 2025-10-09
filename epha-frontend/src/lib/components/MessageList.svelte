<script lang="ts">
	import type { Message, User } from '$lib/api/types';
	import { Circle, CheckCircle } from '@lucide/svelte';

	let { messages = $bindable([]), currentUser = $bindable(null) } = $props();

	let messageContainer: HTMLElement;

	// Auto-scroll to bottom when new messages arrive
	$effect(() => {
		if (messageContainer) {
			const scrollHeight = messageContainer.scrollHeight;
			const scrollTop = messageContainer.scrollTop;
			const clientHeight = messageContainer.clientHeight;

			// Only auto-scroll if user is already near the bottom
			const isNearBottom = scrollHeight - scrollTop - clientHeight < 100;

			if (isNearBottom) {
				messageContainer.scrollTop = scrollHeight;
			}
		}
	});

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

	function groupMessagesByDate(messages: Message[]): Map<string, Message[]> {
		const groups = new Map<string, Message[]>();

		for (const message of messages) {
			const date = formatDate(message.created_at);
			if (!groups.has(date)) {
				groups.set(date, []);
			}
			groups.get(date)!.push(message);
		}

		return groups;
	}

	const messageGroups = $derived(groupMessagesByDate(messages));

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
	<div bind:this={messageContainer} class="h-full overflow-auto">
		<div class="space-y-4 p-4">
			{#each Array.from(messageGroups.entries()) as [date, dateMessages], groupIndex}
				<!-- Date Header -->
				<div class="flex items-center justify-center my-4">
					<div class="bg-surface-200-800 px-3 py-1 rounded-full">
						<span class="text-xs font-medium">
							{date}
						</span>
					</div>
				</div>

				<!-- Messages for this date -->
				{#each dateMessages as message, messageIndex}
					{@const globalIndex = messages.indexOf(message)}
					{@const isOwn = currentUser && message.sender === currentUser.name}
					{@const isUnread = globalIndex === unreadIndex}

					<!-- Unread Divider -->
					{#if isUnread}
						<div class="flex items-center justify-center my-4">
							<div class="flex items-center gap-2 text-xs text-surface-500">
								<div class="h-px bg-surface-300-700 flex-1"></div>
								<div class="flex items-center gap-1">
									<CheckCircle class="w-3 h-3" />
									<span>Unread messages</span>
								</div>
								<div class="h-px bg-surface-300-700 flex-1"></div>
							</div>
						</div>
					{/if}

					<!-- Message Bubble -->
					<div class="flex {isOwn ? 'justify-end' : 'justify-start'} mb-2">
						<div class="max-w-xs lg:max-w-md">
							<!-- Sender Info (only for others' messages) -->
							{#if !isOwn}
								<div class="flex items-center gap-2 mb-1 px-2">
									<Circle class="w-2 h-2 text-green-500 fill-green-500" />
									<span class="text-xs font-medium">
										{message.sender}
									</span>
									<span class="text-xs text-surface-500">
										{formatTime(message.created_at)}
									</span>
								</div>
							{/if}

							<!-- Message Content -->
							<div class="rounded-2xl px-4 py-2 {isOwn
								? 'bg-primary-500 text-white rounded-br-sm'
								: 'bg-surface-200-800 rounded-bl-sm'}">
								<p class="text-sm whitespace-pre-wrap break-words">
									{message.content}
								</p>
							</div>

							<!-- Timestamp (only for own messages) -->
							{#if isOwn}
								<div class="flex items-center justify-end gap-2 mt-1 px-2">
									<span class="text-xs text-surface-500">
										{formatTime(message.created_at)}
									</span>
								</div>
							{/if}
						</div>
					</div>
				{/each}
			{/each}
		</div>
	</div>
{/if}