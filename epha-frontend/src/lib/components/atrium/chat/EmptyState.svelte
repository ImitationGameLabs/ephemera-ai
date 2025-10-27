<script lang="ts">
	import { MessageSquare } from '@lucide/svelte';
</script>

<div class="flex items-center justify-center relative">
	<div class="text-center max-w-md mx-6">
		<div class="w-16 h-16 bg-surface-100-900 rounded-full flex items-center justify-center mx-auto mb-4">
			<MessageSquare class="w-8 h-8 text-surface-500" />
		</div>
		<h3 class="text-lg font-medium mb-2">No messages yet</h3>
		<p class="text-sm text-surface-600-400">
			Be the first to start the conversation!
		</p>
	</div>
</div>