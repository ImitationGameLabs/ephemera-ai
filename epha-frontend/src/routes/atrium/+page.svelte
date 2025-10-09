<script lang="ts">
	import { auth } from '$lib/stores/auth';
	import { onMount } from 'svelte';
	import LoginModal from '$lib/components/LoginModal.svelte';
	import { MessageSquare, Users, Circle } from '@lucide/svelte';
	import { dialogueAtriumAPI } from '$lib/api/dialogue-atrium';
	import type { User } from '$lib/api/types';

	// Modal state
	let isLoginModalOpen = $state(false);

	// Online users state
	let users = $state<User[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Derived online user count
	let onlineUsersCount = $derived(users.filter(u => u.status.online).length);

	// Refresh users function
	async function refreshUsers() {
		try {
			error = null;
			const result = await dialogueAtriumAPI.getAllUsers();
			if (result instanceof Error) {
				error = result.message;
			} else {
				users = result;
			}
		} catch (e) {
			error = 'Failed to fetch users';
			console.error('Error fetching users:', e);
		} finally {
			loading = false;
		}
	}

	// Restore session on mount and start polling
	onMount(() => {
		auth.restoreSession();

		// Initial load
		refreshUsers();

		// Set up polling every 3 seconds
		const interval = setInterval(refreshUsers, 3000);

		// Cleanup on unmount
		return () => clearInterval(interval);
	});
</script>

<div class="flex flex-col h-full bg-surface-50-950">
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

				<!-- Online Users Count -->
				<div class="flex items-center gap-2 px-4 py-2 rounded-full bg-surface-50-950 border border-surface-200-800">
					<Circle class="w-3 h-3 text-green-500 fill-green-500" />
					<Users class="w-4 h-4 text-surface-600-400" />
					<span class="text-sm font-medium">
						{#if loading}
							Loading...
						{:else if error}
							Error
						{:else}
							{onlineUsersCount} Online
						{/if}
					</span>
				</div>
			</div>
		</div>
	</div>

	<!-- Welcome Content -->
	<div class="flex-1 flex items-center justify-center">
		<div class="text-center max-w-md mx-6">
			<div class="mb-8">
				<div class="w-16 h-16 bg-primary-100-900 rounded-full flex items-center justify-center mx-auto mb-4">
					<MessageSquare class="w-8 h-8 text-primary-500" />
				</div>
				<h2 class="text-3xl font-bold mb-4">Welcome to Atrium</h2>
				<p class="text-surface-600-400 mb-8">
					Join the conversation and connect with others in this shared space.
				</p>
			</div>

			{#if !$auth.authenticatedUser}
				<button
					class="btn preset-filled-primary w-full max-w-xs mx-auto"
					onclick={() => isLoginModalOpen = true}
				>
					Sign In to Continue
				</button>
			{:else}
				<div class="space-y-4">
					<p class="text-lg">
						Welcome back, <span class="font-semibold">{$auth.authenticatedUser?.user.name}</span>!
					</p>
					<p class="text-sm text-surface-600-400">
						Chat interface coming soon...
					</p>
				</div>
			{/if}
		</div>
	</div>
</div>

<!-- Login Modal -->
<LoginModal bind:isOpen={isLoginModalOpen} />