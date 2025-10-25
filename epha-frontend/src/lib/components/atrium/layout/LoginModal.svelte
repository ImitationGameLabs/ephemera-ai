<script lang="ts">
	import { auth } from '$lib/stores/auth';
	import { X, User, Lock, MessageSquare } from '@lucide/svelte';

	let { isOpen = $bindable(false) } = $props();

	// Form states
	let username = $state('');
	let password = $state('');
	let bio = $state('');

	// UI mode: 'login' for sign-in form, 'register' for sign-up form
	let mode = $state<'login' | 'register'>('login');

	// Form submission state: true while API call is in progress
	let isSubmitting = $state(false);

	// Local error state
	let error = $state<string | null>(null);

	// Close modal and reset form
	function closeModal() {
		isOpen = false;
		resetForm();
	}

	// Handle click on backdrop to close modal
	function handleBackdropClick(event: MouseEvent) {
		// Only close if clicking the backdrop itself, not its children
		if (event.target === event.currentTarget) {
			closeModal();
		}
	}

	// Handle keyboard events for accessibility
	function handleBackdropKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			closeModal();
		}
	}

	function resetForm() {
		username = '';
		password = '';
		bio = '';
		mode = 'login';
		isSubmitting = false;
		error = null;
	}

	// Handle login
	async function handleLogin() {
		if (!username || !password) return;

		isSubmitting = true;

		try {
			const loginError = await auth.login(username, password);
			if (loginError) {
				// Set local error from loginError
				error = loginError.message || 'Login failed';
			} else {
				closeModal();
			}
		} finally {
			isSubmitting = false;
		}
	}

	// Handle registration
	async function handleRegister() {
		if (!username || !password || !bio) return;

		isSubmitting = true;

		try {
			const registerError = await auth.register({
				name: username,
				bio,
				password
			});
			if (registerError) {
				// Set local error from registerError
				error = registerError.message || 'Registration failed';
			} else {
				closeModal();
			}
		} finally {
			isSubmitting = false;
		}
	}

	// Close on escape key
	$effect(() => {
		if (isOpen) {
			const handleEsc = (e: KeyboardEvent) => {
				if (e.key === 'Escape') {
					closeModal();
				}
			};
			document.addEventListener('keydown', handleEsc);
			return () => document.removeEventListener('keydown', handleEsc);
		}
	});
</script>

{#if isOpen}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
		role="dialog"
		aria-modal="true"
		aria-labelledby="modal-title"
		tabindex="-1"
		onclick={handleBackdropClick}
		onkeydown={handleBackdropKeydown}
	>
		<div class="card bg-surface-100-900 w-full max-w-md mx-4 shadow-2xl" role="document">
			<!-- Header -->
			<div class="flex items-center justify-between p-6 border-b border-surface-200-800">
				<div class="flex items-center gap-3">
					<div class="w-10 h-10 rounded-full bg-primary-500/20 flex items-center justify-center">
						{#if mode === 'register'}
							<User class="w-5 h-5 text-primary-500" />
						{:else}
							<MessageSquare class="w-5 h-5 text-primary-500" />
						{/if}
					</div>
					<div>
						<h2 id="modal-title" class="text-xl font-semibold">
							{mode === 'register' ? 'Join Atrium' : 'Welcome Back'}
						</h2>
						<p class="text-sm">
							{mode === 'register' ? 'Create your account' : 'Sign in to continue'}
						</p>
					</div>
				</div>
				<button class="btn-icon preset-ghost" onclick={closeModal}>
					<X class="w-5 h-5" />
				</button>
			</div>

			<!-- Form Content -->
			<div class="p-6">
				{#if error}
					<div class="alert preset-error mb-4">
						<span>{error}</span>
					</div>
				{/if}

				<form onsubmit={e => { e.preventDefault(); mode === 'register' ? handleRegister() : handleLogin(); }} class="space-y-4">
					<!-- Username Field -->
					<div>
						<label for="username" class="block text-sm font-medium mb-2">
							Username
						</label>
						<div class="relative">
							<User class="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4" />
							<input
								id="username"
								type="text"
								bind:value={username}
								placeholder="Enter your username"
								class="input w-full pl-10"
								required
								disabled={isSubmitting}
							/>
						</div>
					</div>

					<!-- Password Field -->
					<div>
						<label for="password" class="block text-sm font-medium mb-2">
							Password
						</label>
						<div class="relative">
							<Lock class="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4" />
							<input
								id="password"
								type="password"
								bind:value={password}
								placeholder="Enter your password"
								class="input w-full pl-10"
								required
								disabled={isSubmitting}
							/>
						</div>
					</div>

					<!-- Bio Field (only for registration) -->
					{#if mode === 'register'}
						<div>
							<label for="bio" class="block text-sm font-medium mb-2">
								Bio
							</label>
							<textarea
								id="bio"
								bind:value={bio}
								placeholder="Tell us about yourself"
								class="input preset-filled w-full resize-none"
								rows="3"
								required
								disabled={isSubmitting}
							></textarea>
						</div>
					{/if}

					<!-- Submit Button -->
					<button
						type="submit"
						class="btn preset-filled-primary-300-700 w-full"
						disabled={isSubmitting || !username || !password || (mode === 'register' && !bio)}
					>
						{#if isSubmitting}
							<span class="loading loading-spinner w-4 h-4"></span>
							{mode === 'register' ? 'Creating Account...' : 'Signing In...'}
						{:else}
							{mode === 'register' ? 'Create Account' : 'Sign In'}
						{/if}
					</button>
				</form>

				<!-- Toggle Login/Register -->
				<div class="mt-6 text-center">
					<p class="text-sm">
						{mode === 'register' ? 'Already have an account?' : "Don't have an account?"}
						<button
							class="btn preset-link text-primary-500 hover:text-primary-600 ml-1"
							onclick={() => {
								mode = mode === 'login' ? 'register' : 'login';
								error = null;
							}}
							disabled={isSubmitting}
						>
							{mode === 'register' ? 'Sign In' : 'Sign Up'}
						</button>
					</p>
				</div>
			</div>
		</div>
	</div>
{/if}