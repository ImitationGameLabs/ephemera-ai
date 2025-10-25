<script lang="ts">
	import { auth, AuthMode } from '$lib/stores/auth';
	import { messagesStore } from '$lib/stores/messages';
	import { onMount } from 'svelte';
	import LoginModal from '$lib/components/LoginModal.svelte';
	import WelcomeContent from '$lib/components/WelcomeContent.svelte';
	import ChatInterface from '$lib/components/ChatInterface.svelte';
	import StatusIndicators from '$lib/components/StatusIndicators.svelte';
	import DisconnectedBanner from '$lib/components/DisconnectedBanner.svelte';
	import OnlineUsersCount from '$lib/components/OnlineUsersCount.svelte';
	import UserStatus from '$lib/components/UserStatus.svelte';
	import { dialogueAtriumAPI } from '$lib/api/dialogue-atrium';
	import { heartbeatManager } from '$lib/services/heartbeat';
	import type { User } from '$lib/api/types';

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
	let currentMessages: any[] = $state([]);
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

		// Set up polling for users every 3 seconds
		const usersInterval = setInterval(refreshUsers, 3000);

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

			// Clear error after 3 seconds
			setTimeout(() => {
				sendingError = null;
			}, 3000);
		}
	}
</script>

<div class="flex flex-col flex-1 bg-surface-50-950 min-h-0 h-full">
	<!-- Header -->
	<div class="bg-surface-100-900 border-b border-surface-200-800 px-6 py-4">
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
	{:else if $auth.authMode === 'offline'}
		<!-- Offline mode: Show chat interface with disconnected banner -->
		<div class="flex flex-col flex-1 min-h-0">
			<DisconnectedBanner onRetry={() => heartbeatManager.resetAndRetry()} />
			<ChatInterface
				messages={currentMessages}
				loading={currentLoading}
				error={currentError}
				initialLoadDone={initialLoadDone}
				sendingError={sendingError}
				notifications={notifications}
				currentUser={$auth.authenticatedUser?.user}
				isOffline={$auth.authMode === AuthMode.OFFLINE}
				onSendMessage={handleSendMessage}
				onRetryLoad={loadInitialMessages}
				onClearNotifications={() => messagesStore.clearNotifications()}
			/>
		</div>
	{:else}
		<!-- Online mode: Normal chat interface -->
		<ChatInterface
			messages={currentMessages}
			loading={currentLoading}
			error={currentError}
			initialLoadDone={initialLoadDone}
			sendingError={sendingError}
			notifications={notifications}
			currentUser={$auth.authenticatedUser?.user}
			isOffline={false}
			onSendMessage={handleSendMessage}
			onRetryLoad={loadInitialMessages}
			onClearNotifications={() => messagesStore.clearNotifications()}
		/>
	{/if}
</div>

<!-- Login Modal -->
<LoginModal bind:isOpen={isLoginModalOpen} />