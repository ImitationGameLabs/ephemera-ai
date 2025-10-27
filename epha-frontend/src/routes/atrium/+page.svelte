<script lang="ts">
	import { auth, AuthMode } from '$lib/stores/auth';
	import { messagesStore } from '$lib/stores/messages';
	import { onMount } from 'svelte';
	import LoginModal from '$lib/components/atrium/layout/LoginModal.svelte';
	import WelcomeContent from '$lib/components/atrium/layout/WelcomeContent.svelte';
	import ChatInterface from '$lib/components/atrium/chat/ChatInterface.svelte';
	import StatusIndicators from '$lib/components/atrium/layout/StatusIndicators.svelte';
	import DisconnectedBanner from '$lib/components/atrium/layout/DisconnectedBanner.svelte';
	import OnlineUsersCount from '$lib/components/atrium/layout/OnlineUsersCount.svelte';
	import UserStatus from '$lib/components/atrium/layout/UserStatus.svelte';
	import { dialogueAtriumAPI } from '$lib/api/dialogue-atrium';
	import { heartbeatManager } from '$lib/services/heartbeat';
	import { MESSAGE_CONFIG } from '$lib/config/app';
	import { CircleAlert } from '@lucide/svelte';
	import type { User, Message } from '$lib/api/types';

	// Modal state
	let isLoginModalOpen = $state(false);

	// Online users state
	let users = $state<User[]>([]);
	let usersLoading = $state(true);
	let usersError = $state<string | null>(null);

	// Chat state
	let initialLoadDone = $state(false);
	let sendingError = $state<string | null>(null);

	
	// Messages state - subscribe to store
	let currentMessages: Message[] = $state([]);
	let currentLoading = $state(false);
	let currentError = $state<string | null>(null);
	let notifications = $state({ count: 0, hasUnloadedUnread: false });

	// Subscribe to messages store
	$effect(() => {
		const unsubscribeMessages = messagesStore.subscribe(state => {
			currentMessages = state.messages;
			currentLoading = state.loading;
			currentError = state.error;
		});

		const unsubscribeNotifications = messagesStore.notifications.subscribe(value => {
			notifications = value;
		});

		return () => {
			unsubscribeMessages();
			unsubscribeNotifications();
		};
	});

	// Initialize on mount
	onMount(() => {
		auth.restoreSession();

		// Only start users polling, let message loading depend on auth state
		refreshUsers();

		// Set up polling for users
		const usersInterval = setInterval(refreshUsers, MESSAGE_CONFIG.POLLING_INTERVAL);

		// Cleanup on unmount
		return () => {
			clearInterval(usersInterval);
			messagesStore.stopPolling();
		};
	});

	// React to auth state changes
	$effect(() => {
		if ($auth.authenticatedUser) {
			if ($auth.authMode === 'online') {
				// User is online, start messages polling and reload messages
				messagesStore.startPolling($auth.authenticatedUser.user);
				loadInitialMessages(); // Reload messages when coming back online
			} else if ($auth.authMode === 'offline') {
				// User is offline, stop polling but keep messages visible
				messagesStore.stopPolling();
				// Don't try to load messages when offline
			}
		} else {
			// User is not authenticated, stop polling and reset messages
			messagesStore.stopPolling();
			messagesStore.reset();
		}
	});

	// Refresh users function
	async function refreshUsers() {
		try {
			usersError = null;
			const result = await dialogueAtriumAPI.getAllUsers();
			if (result instanceof Error) {
				usersError = result.message;
			} else {
				users = result;
			}
		} catch (e) {
			usersError = 'Failed to fetch users';
			console.error('Error fetching users:', e);
		} finally {
			usersLoading = false;
		}
	}

	// Load initial messages
	async function loadInitialMessages() {
		try {
			const success = await messagesStore.loadInitialMessages();
			initialLoadDone = true;
			return success;
		} catch (error) {
			console.error('Failed to load initial messages:', error);
			initialLoadDone = true;
			return false;
		}
	}

	// Handle send message
	async function handleSendMessage(content: string) {
		if (!$auth.authenticatedUser) return;

		try {
			sendingError = null;
			await messagesStore.sendMessage(content, $auth.authenticatedUser.credentials);
		} catch (error) {
			console.error('Failed to send message:', error);
			sendingError = error instanceof Error ? error.message : 'Failed to send message';

			// Clear error after polling interval
			setTimeout(() => {
				sendingError = null;
			}, MESSAGE_CONFIG.POLLING_INTERVAL);
		}
	}
</script>

<div class="flex-1 flex flex-col bg-surface-50-950 min-h-0">
	<!-- Header -->
	<div class="sticky top-0 bg-surface-100-900 border-b border-surface-200-800 p-6 z-10 min-h-0">
		<div class="max-w-6xl mx-auto">
			<div class="flex items-center justify-between">
				<div>
					<h1 class="text-2xl font-bold">
						Atrium
					</h1>
					<p class="">
						Shared space for conversation
					</p>
				</div>

				<!-- Status Indicators & Online Users -->
				<div class="flex items-center gap-6">
					<StatusIndicators
						currentUser={$auth.authenticatedUser?.user}
						messages={currentMessages}
						loading={currentLoading}
					/>

					<OnlineUsersCount
						onlineUsers={users.filter(u => u.status?.online || false)}
						currentUser={$auth.authenticatedUser?.user}
						loading={usersLoading}
					/>

					<!-- User Status with Logout -->
					<UserStatus />
				</div>
			</div>
		</div>
	</div>

	<!-- Chat Area -->
	{#if !$auth.authenticatedUser}
		<WelcomeContent onSignIn={() => isLoginModalOpen = true} />
	{:else}
		<!-- Offline mode: Show chat interface with disconnected banner -->
		{#if $auth.authMode === AuthMode.OFFLINE}
			<DisconnectedBanner onRetry={() => heartbeatManager.resetAndRetry()} />
		{/if}
		<ChatInterface
			classes="w-full max-w-4xl mx-auto min-h-0 border-2"
			messages={currentMessages}
			loading={currentLoading}
			error={currentError}
			initialLoadDone={initialLoadDone}
			sendingError={sendingError}
			notifications={notifications}
			currentUser={$auth.authenticatedUser}
			isOffline={$auth.authMode === AuthMode.OFFLINE}
			onSendMessage={handleSendMessage}
			onRetryLoad={loadInitialMessages}
			onClearNotifications={() => messagesStore.clearNotifications()}
		/>
	{/if}
</div>

<!-- Login Modal -->
<LoginModal bind:isOpen={isLoginModalOpen} />