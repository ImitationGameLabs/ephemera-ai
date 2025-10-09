<script lang="ts">
	import { browser } from '$app/environment';
	import { auth } from '$lib/stores/auth';
	import { User, LogIn, LogOut, Circle } from '@lucide/svelte';
	import LoginModal from './LoginModal.svelte';
	import { onMount } from 'svelte';

	let isLoginModalOpen = $state(false);
	let isAuthenticated = $state(false);
	let currentUser = $state<any>(null);

	onMount(() => {
		if (!browser) return;

		// Subscribe to auth store
		const unsubscribe = auth.subscribe(state => {
			isAuthenticated = state?.authenticatedUser !== null;
			currentUser = state?.authenticatedUser?.user || null;
		});

		return unsubscribe;
	});

	function handleLogout() {
		auth.logout();
	}

	function openLoginModal() {
		isLoginModalOpen = true;
	}
</script>

<div class="flex items-center gap-3">
	{#if browser && isAuthenticated && currentUser}
		<!-- User Status for logged in users -->
		<div class="flex items-center gap-2 px-3 py-2 rounded-full bg-surface-100-900 border border-surface-200-800">
			<!-- Online Status Indicator -->
			<div class="relative">
				<Circle class="w-3 h-3 text-green-500 fill-green-500" />
			</div>

			<!-- Username and Bio -->
			<div class="hidden sm:block">
				<p class="text-sm font-medium">
					{currentUser.name}
				</p>
				{#if currentUser.bio}
					<p class="text-xs truncate max-w-32">
						{currentUser.bio}
					</p>
				{/if}
			</div>

			<!-- Logout Button -->
			<button
				class="btn-icon preset-ghost hover:text-red-500"
				onclick={handleLogout}
				title="Logout"
			>
				<LogOut class="w-4 h-4" />
			</button>
		</div>
	{:else}
		<!-- Login Button for anonymous users -->
		<button
			class="btn preset-filled-primary flex items-center gap-2 px-4 py-2"
			onclick={openLoginModal}
		>
			<LogIn class="w-4 h-4" />
			<span class="hidden sm:inline">Sign In</span>
		</button>
	{/if}

	<!-- Login Modal -->
	{#if browser}
		<LoginModal bind:isOpen={isLoginModalOpen} />
	{/if}
</div>