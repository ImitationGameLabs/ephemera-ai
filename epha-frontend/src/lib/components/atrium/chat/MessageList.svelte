<script lang="ts">
	import type { Message } from '$lib/api/types';
	import type { AuthenticatedUser } from '$lib/stores/auth';
	import { Users } from '@lucide/svelte';
	import { messagesStore } from '$lib/stores/messages';
	import { useIntersectionObserver } from '$lib/actions/useIntersectionObserver';
	import EmptyState from './EmptyState.svelte';
	import LoadingState from './LoadingState.svelte';
	import UnreadDivider from './UnreadDivider.svelte';
	import MessageGroup from './MessageGroup.svelte';
	import type { MessageGroupData } from './MessageGroup.svelte';
	import DateDivider from './DateDivider.svelte';

	interface MessageListProps {
		messages: Message[];
		currentUser?: AuthenticatedUser | null;
		loading?: boolean;
		onScrollToTop?: () => void;
		onClearNotifications?: () => void;
		class?: string;
	}

	let {
		messages = $bindable([]),
		currentUser = $bindable(null),
		loading = $bindable(false),
		onScrollToTop = $bindable(() => {}),
		onClearNotifications = $bindable(() => {}),
		class: classes = $bindable('')
	}: MessageListProps = $props();

	let visibleMessageIds = $state(new Set<number>());
	let showScrollToBottom = $state(false);

	interface DateSection {
		date: string;
		groups: MessageGroupData[];
	}

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

	// Handle scroll events for loading more messages and scroll-to-bottom button
	function handleScroll(event: Event) {
		const target = event.target as HTMLElement;
		if (!target) return;

		const { scrollTop, scrollHeight, clientHeight } = target;
		const distanceFromBottom = scrollHeight - scrollTop - clientHeight;

		// Show scroll to bottom button if not near bottom
		showScrollToBottom = distanceFromBottom > 200;

		// Clear notifications if user scrolls to bottom (viewing latest messages)
		if (distanceFromBottom < 10) {
			onClearNotifications();
		}
	}

	// Scroll to bottom using modern API
	function scrollToBottom() {
		const scrollTarget = document.querySelector<HTMLElement>('[data-scroll-target="bottom"]');
		if (scrollTarget) {
			scrollTarget.scrollIntoView({ behavior: 'smooth' });
		}
	}

	function getDateOnly(timestamp: string): string {
		return new Date(timestamp).toDateString();
	}

	function groupMessagesByDate(messages: Message[]): DateSection[] {
		const dateGroups: { [key: string]: Message[] } = {};

		// First, group messages by date
		for (const message of messages) {
			const date = getDateOnly(message.created_at);
			if (!dateGroups[date]) {
				dateGroups[date] = [];
			}
			dateGroups[date].push(message);
		}

		// Then, for each date, group by user and time
		const sections: DateSection[] = [];
		const sortedDates = Object.keys(dateGroups).sort((a, b) =>
			new Date(a).getTime() - new Date(b).getTime()
		);

		for (const date of sortedDates) {
			const dateMessages = dateGroups[date];
			const groups: MessageGroupData[] = [];

			for (const message of dateMessages) {
				const isOwn = currentUser && message.sender === currentUser.user.name;
				const messageTime = new Date(message.created_at).getTime();

				const lastGroup = groups[groups.length - 1];

				if (lastGroup &&
					lastGroup.user === message.sender &&
					lastGroup.isOwn === isOwn &&
					(messageTime - new Date(lastGroup.time).getTime()) < 2 * 60 * 1000) {
					lastGroup.messages.push(message);
				} else {
					groups.push({
						user: message.sender,
						time: message.created_at,
						messages: [message],
						isOwn: isOwn || false
					});
				}
			}

			sections.push({
				date: dateMessages[0].created_at,
				groups
			});
		}

		return sections;
	}

	const dateSections = $derived(groupMessagesByDate(messages));

	function findUnreadIndex(messages: Message[], currentUser: AuthenticatedUser | null): number {
		if (!currentUser) return -1;

		const messageHeight = currentUser.user.message_height;
		return messages.findIndex(msg => msg.id > messageHeight);
	}

	const unreadIndex = $derived(findUnreadIndex(messages, currentUser));
</script>

<div class="{classes}">
	{#if messages.length === 0}
		<EmptyState />
	{:else if loading && !messages.length}
		<LoadingState />
	{:else}
		<div class="flex h-full">
			<div 
				class="flex-1 flex flex-col space-y-4 p-4 overflow-auto"
				onscroll={handleScroll}
				use:useIntersectionObserver={{
					onVisible: handleMessageVisible,
					onHidden: handleMessageHidden
				}}
			>
				{#each dateSections as dateSection, dateIndex}
					<!-- Date Divider -->
					<DateDivider date={dateSection.date} />

					<!-- Message Groups for this date -->
					{#each dateSection.groups as group, groupIndex}
						<!-- Check for unread divider before this group -->
						{@const firstGlobalIndex = messages.indexOf(group.messages[0])}
						{@const isUnread = firstGlobalIndex === unreadIndex}

						{#if isUnread}
							<UnreadDivider />
						{/if}

						<!-- Message Group -->
						<MessageGroup {group} />
					{/each}
				{/each}
				<!-- Scroll target for scrollToBottom functionality -->
				<div data-scroll-target="bottom" style="height: 1px;"></div>
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
		</div>
	{/if}
</div>