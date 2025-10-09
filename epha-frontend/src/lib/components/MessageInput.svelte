<script lang="ts">
	import { auth } from '$lib/stores/auth';
	import { dialogueAtriumAPI } from '$lib/api/dialogue-atrium';
	import { Send } from '@lucide/svelte';

	let messageText = $state('');
	let isSending = $state(false);
	let textArea: HTMLTextAreaElement;

	const canSend = $derived(
		$auth.authenticatedUser &&
		messageText.trim() &&
		!isSending
	);

	async function sendMessage() {
		if (!canSend) return;

		isSending = true;
		const content = messageText.trim();
		messageText = '';

		try {
			await dialogueAtriumAPI.sendMessage({
				content,
				username: $auth.authenticatedUser!.credentials.username,
				password: $auth.authenticatedUser!.credentials.password
			});

			// Focus back to textarea after sending
			textArea?.focus();
		} catch (error) {
			// Restore message text on error
			messageText = content;
			console.error('Failed to send message:', error);
		} finally {
			isSending = false;
		}
	}

	// Auto-resize textarea based on content
	$effect(() => {
		if (textArea) {
			textArea.style.height = 'auto';
			textArea.style.height = Math.min(textArea.scrollHeight, 120) + 'px';
		}
	});

	function handleKeyDown(event: KeyboardEvent) {
		// Send on Enter, new line on Shift+Enter
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			sendMessage();
		}
	}
</script>

<div class="border-t border-surface-200-800 p-4 bg-surface-100-900">
	{#if !$auth.authenticatedUser}
		<div class="text-center py-4">
			<p>Sign in to join the conversation</p>
		</div>
	{:else}
		<div class="max-w-4xl mx-auto">
			<div class="flex gap-2">
				<!-- Message Input -->
				<div class="flex-1 relative">
					<textarea
						bind:this={textArea}
						bind:value={messageText}
						placeholder="Type your message... (Enter to send, Shift+Enter for new line)"
						class="input preset-filled w-full resize-none min-h-[44px] max-h-[120px] pr-12"
						disabled={isSending}
						onkeydown={handleKeyDown}
					></textarea>

					<!-- Character counter (optional) -->
					{#if messageText.length > 0}
						<div class="absolute bottom-2 right-12 text-xs text-surface-400">
							{messageText.length}
						</div>
					{/if}
				</div>

				<!-- Send Button -->
				<button
					class="btn preset-filled-primary h-[44px] px-4 flex items-center justify-center"
					disabled={!canSend}
					onclick={sendMessage}
					title="Send message"
				>
					{#if isSending}
						<div class="loading loading-spinner w-5 h-5"></div>
					{:else}
						<Send class="w-5 h-5" />
					{/if}
				</button>
			</div>

			<!-- Keyboard shortcuts hint -->
			<div class="text-xs mt-2 text-center">
				Press <kbd class="px-1 py-0.5 bg-surface-200-800 rounded">Enter</kbd> to send,
				<kbd class="px-1 py-0.5 bg-surface-200-800 rounded">Shift+Enter</kbd> for new line
			</div>
		</div>
	{/if}
</div>