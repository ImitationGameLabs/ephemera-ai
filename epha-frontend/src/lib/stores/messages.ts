import { writable, derived } from 'svelte/store';
import { dialogueAtriumAPI } from '$lib/api/dialogue-atrium';
import type { Message, User } from '$lib/api/types';

interface MessagesState {
	messages: Message[];
	loading: boolean;
	error: string | null;
	lastFetched: number | null;
	hasMore: boolean;
	currentOffset: number;
	totalCount: number | null;
}

interface NewMessagesNotification {
	count: number;
	hasUnloadedUnread: boolean;
}

function createMessagesStore() {
	const { subscribe, set, update } = writable<MessagesState>({
		messages: [],
		loading: false,
		error: null,
		lastFetched: null,
		hasMore: true,
		currentOffset: 0,
		totalCount: null
	});

	const { subscribe: notifySub, set: setNotify } = writable<NewMessagesNotification>({
		count: 0,
		hasUnloadedUnread: false
	});

	let pollingInterval: ReturnType<typeof setInterval> | null = null;
	let isPolling = false;

	// Initial load - get latest 50 messages
	async function loadInitialMessages() {
		update(state => ({ ...state, loading: true, error: null }));

		try {
			const result = await dialogueAtriumAPI.getMessages({ limit: 50, offset: 0 });

			if (result instanceof Error) {
				update(state => ({
					...state,
					loading: false,
					error: result.message
				}));
				return false;
			}

			
			// Sort messages by created_at ascending (oldest first)
			const sortedMessages = result.messages.sort((a, b) =>
				new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
			);

			update(state => ({
				...state,
				messages: sortedMessages,
				loading: false,
				lastFetched: Date.now(),
				currentOffset: result.messages.length,
				hasMore: result.messages.length === 50
			}));

			return true;
		} catch (error) {
			update(state => ({
				...state,
				loading: false,
				error: 'Failed to fetch messages'
			}));
			return false;
		}
	}

	// Load older messages (scrolling up)
	async function loadOlderMessages() {
		const currentState = get();
		if (currentState.loading || !currentState.hasMore) return false;

		update(state => ({ ...state, loading: true, error: null }));

		try {
			const result = await dialogueAtriumAPI.getMessages({
				limit: 50,
				offset: currentState.currentOffset
			});

			if (result instanceof Error) {
				update(state => ({
					...state,
					loading: false,
					error: result.message
				}));
				return false;
			}

			if (result.messages.length === 0) {
				update(state => ({
					...state,
					loading: false,
					hasMore: false
				}));
				return false;
			}

			// Sort older messages
			const sortedOlderMessages = result.messages.sort((a, b) =>
				new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
			);

			update(state => {
				// Prepend older messages to existing ones
				const allMessages = [...sortedOlderMessages, ...state.messages];
				return {
					...state,
					messages: allMessages,
					loading: false,
					currentOffset: state.currentOffset + result.messages.length,
					hasMore: result.messages.length === 50
				};
			});

			return true;
		} catch (error) {
			update(state => ({
				...state,
				loading: false,
				error: 'Failed to load older messages'
			}));
			return false;
		}
	}

	// Check for new messages (polling)
	async function checkForNewMessages(currentUser: User | null) {
		if (isPolling) return;

		isPolling = true;

		try {
			const currentState = await new Promise<MessagesState>((resolve) => {
				const unsubscribe = subscribe(state => {
					resolve(state);
					unsubscribe();
				});
			});

			// Get latest message ID we have
			const latestMessageId = currentState.messages.length > 0
				? Math.max(...currentState.messages.map(m => m.id))
				: 0;

			// Fetch messages newer than what we have
			const result = await dialogueAtriumAPI.getMessages({
				limit: 50,
				offset: 0
			});

			if (result instanceof Error) return;

			
			const newMessages = result.messages.filter(m => m.id > latestMessageId);

			if (newMessages.length > 0) {
				// Sort new messages
				const sortedNewMessages = newMessages.sort((a, b) =>
					new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
				);

				update(state => {
					const allMessages = [...state.messages, ...sortedNewMessages];
					return {
						...state,
						messages: allMessages,
						lastFetched: Date.now()
					};
				});

				// Update notification - only show notification if these are messages from others
			const messagesFromOthers = newMessages.filter(m => {
				// We need to determine if these messages are from other users
				// For now, we'll show all new messages as notifications
				return true;
			});

			if (messagesFromOthers.length > 0) {
				setNotify({
					count: messagesFromOthers.length,
					hasUnloadedUnread: false
				});
			}
			}

			// Check if there are unread messages we haven't loaded
			if (currentUser && newMessages.length === 0) {
				const userReadHeight = currentUser.message_height;
				const hasUnreadInLoaded = currentState.messages.some(m => m.id > userReadHeight);

				// If we have recent messages but none are unread, there might be unread in older messages
				if (!hasUnreadInLoaded && currentState.messages.length > 0) {
					const oldestLoadedId = Math.min(...currentState.messages.map(m => m.id));
					if (oldestLoadedId > userReadHeight + 1) {
						setNotify({
							count: 0,
							hasUnloadedUnread: true
						});
					}
				}
			}

		} catch (error) {
			console.error('Error checking for new messages:', error);
		} finally {
			isPolling = false;
		}
	}

	async function sendMessage(content: string, credentials: { username: string; password: string }) {
		try {
			const result = await dialogueAtriumAPI.sendMessage({
				content,
				username: credentials.username,
				password: credentials.password
			});

			if (result instanceof Error) {
				throw result;
			}

			// Immediately check for new messages after sending
			setTimeout(() => {
				const currentState = get();
				if (currentState.messages.length > 0) {
					checkForNewMessages(null); // Pass null since we just sent a message
				}
			}, 100);

			return result;
		} catch (error) {
			throw error;
		}
	}

	// Mark messages as read by updating user's message_height
	async function markAsRead(messageId: number, credentials: { username: string; password: string }) {
		try {
			await dialogueAtriumAPI.updateHeartbeat(credentials);
			// Note: The actual message_height update would need to be handled by the API
			// This is a placeholder for the read marking functionality
		} catch (error) {
			console.error('Failed to mark messages as read:', error);
		}
	}

	function startPolling(currentUser: User | null) {
		if (pollingInterval) {
			clearInterval(pollingInterval);
		}

		pollingInterval = setInterval(() => {
			checkForNewMessages(currentUser);
		}, 3000);
	}

	function stopPolling() {
		if (pollingInterval) {
			clearInterval(pollingInterval);
			pollingInterval = null;
		}
	}

	function get() {
		let currentState: MessagesState;
		const unsubscribe = subscribe(state => {
			currentState = state;
			unsubscribe();
		});
		return currentState!;
	}

	function clearNotifications() {
		setNotify({
			count: 0,
			hasUnloadedUnread: false
		});
	}

	function reset() {
		stopPolling();
		set({
			messages: [],
			loading: false,
			error: null,
			lastFetched: null,
			hasMore: true,
			currentOffset: 0,
			totalCount: null
		});
		clearNotifications();
	}

	return {
		subscribe,
		notifications: { subscribe: notifySub },
		loadInitialMessages,
		loadOlderMessages,
		checkForNewMessages,
		sendMessage,
		markAsRead,
		startPolling,
		stopPolling,
		clearNotifications,
		reset,
		get
	};
}

export const messagesStore = createMessagesStore();

// Derived stores for convenience
export const messages = derived(messagesStore, $messages => $messages.messages);
export const messagesLoading = derived(messagesStore, $messages => $messages.loading);
export const messagesError = derived(messagesStore, $messages => $messages.error);
export const hasMoreMessages = derived(messagesStore, $messages => $messages.hasMore);