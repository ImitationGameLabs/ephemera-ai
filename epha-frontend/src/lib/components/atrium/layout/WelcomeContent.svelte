<script lang="ts">
	import { MessageSquare } from '@lucide/svelte';
	import type { WelcomeContentProps } from '$lib/types/chat';

	let { onSignIn = $bindable(() => {}) }: WelcomeContentProps = $props();
</script>

<div class="flex items-center justify-center h-full">
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

		<button
			class="btn preset-filled-primary-300-700 w-full max-w-xs mx-auto"
			onclick={onSignIn}
		>
			Sign In to Continue
		</button>
	</div>
</div>