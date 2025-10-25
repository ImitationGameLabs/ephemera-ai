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

{#if browser && isAuthenticated && currentUser}
	<!-- User Status for logged in users -->
	<div class="flex items-center gap-2 px-3 py-2 rounded-full hover:bg-surface-200-800 transition-colors">
		<!-- Username -->
		<div class="hidden sm:block">
			<p class="text-sm font-medium text-surface-700-300">
				{currentUser.name}
			</p>
		</div>

		<!-- Logout Button -->
		<button
			class="btn-icon preset-ghost text-surface-600-400 hover:text-red-500"
			onclick={handleLogout}
			title="Logout"
		>
			<LogOut class="w-4 h-4" />
		</button>
	</div>
{/if}

<!-- Login Modal -->
{#if browser}
	<LoginModal bind:isOpen={isLoginModalOpen} />
{/if}