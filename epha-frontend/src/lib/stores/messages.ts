import { writable, derived } from 'svelte/store';
import { dialogueAtriumAPI } from '$lib/api/dialogue-atrium';
import type { Message, User } from '$lib/api/types';
import { MESSAGE_CONFIG, STORAGE_CONFIG } from '$lib/config/app';

// Message persistence functions
const MESSAGES_STORAGE_KEY = 'atrium_messages';

function saveMessagesToStorage(messages: Message[]): void {
	if (typeof window === 'undefined') return;

	try {
		const limitedMessages = messages.slice(-STORAGE_CONFIG.MAX_STORED_MESSAGES);
		localStorage.setItem(MESSAGES_STORAGE_KEY, JSON.stringify(limitedMessages));
	} catch (error) {
		console.warn('Failed to save messages to localStorage:', error);
	}
}

function loadMessagesFromStorage(): Message[] {
	if (typeof window === 'undefined') return [];

	try {
		const stored = localStorage.getItem(MESSAGES_STORAGE_KEY);
		if (stored) {
			return JSON.parse(stored);
		}
	} catch (error) {
		console.warn('Failed to load messages from localStorage:', error);
		localStorage.removeItem(MESSAGES_STORAGE_KEY);
	}
	return [];
}

function clearMessagesFromStorage(): void {
	if (typeof window === 'undefined') return;

	try {
		localStorage.removeItem(MESSAGES_STORAGE_KEY);
	} catch (error) {
		console.warn('Failed to clear messages from localStorage:', error);
	}
}

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
	// Load messages from localStorage on initialization
	const storedMessages = loadMessagesFromStorage();

	const { subscribe, set, update } = writable<MessagesState>({
		messages: storedMessages,
		loading: false,
		error: null,
		lastFetched: null,
		hasMore: true,
		currentOffset: storedMessages.length,
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

			update(state => {
				const newState = {
					...state,
					messages: sortedMessages,
					loading: false,
					lastFetched: Date.now(),
					currentOffset: result.messages.length,
					hasMore: result.messages.length === 50
				};

				// Save to localStorage
				saveMessagesToStorage(sortedMessages);

				return newState;
			});

			return true;
		} catch (error) {
			// Check if we already have messages to preserve
			const currentState = get();
			if (currentState.messages.length > 0) {
				// Preserve existing messages, just show loading error
				update(state => ({
					...state,
					loading: false,
					error: 'Connection failed. Showing cached messages. Some messages may be missing.'
				}));
			} else {
				// No messages to show, show generic error
				update(state => ({
					...state,
					loading: false,
					error: 'Failed to fetch messages'
				}));
			}
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
				const newState = {
					...state,
					messages: allMessages,
					loading: false,
					currentOffset: state.currentOffset + result.messages.length,
					hasMore: result.messages.length === 50
				};

				// Save updated messages to localStorage
				saveMessagesToStorage(allMessages);

				return newState;
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
					const newState = {
						...state,
						messages: allMessages,
						lastFetched: Date.now()
					};

					// Save updated messages to localStorage
					saveMessagesToStorage(allMessages);

					return newState;
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
			}, MESSAGE_CONFIG.SEND_RETRY_DELAY);

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
		}, MESSAGE_CONFIG.POLLING_INTERVAL);
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
		clearMessagesFromStorage(); // Clear persisted messages on reset
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