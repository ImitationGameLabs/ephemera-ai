<script lang="ts">
	import type { Message } from '$lib/api/types';
	import { Circle } from '@lucide/svelte';

	export interface MessageGroupData {
		user: string;
		time: string;
		messages: Message[];
		isOwn: boolean;
	}

	interface MessageGroupProps {
		group: MessageGroupData;
	}

	let { group }: MessageGroupProps = $props();

	function formatTime(timestamp: string): string {
		return new Date(timestamp).toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<!-- Message Group -->
<div class="flex {group.isOwn ? 'justify-end' : 'justify-start'} mb-4">
	<div class="max-w-xs lg:max-w-md">
		<!-- Group Header: User name and timestamp (shown once per group) -->
		<div class="flex items-center gap-2 mb-1 px-2">
			{#if !group.isOwn}
				<Circle class="w-2 h-2 text-green-500 fill-green-500" />
			{/if}
			<span class="text-xs font-medium">
				{group.user}
			</span>
			<span class="text-xs text-surface-500">
				{formatTime(group.time)}
			</span>
		</div>

		<!-- Message Bubbles in this group -->
		{#each group.messages as message, messageIndex}
			<div class="mb-2" data-message-id={message.id}>
				<!-- Message Content -->
				<div class="rounded-2xl px-4 py-2 {group.isOwn
					? 'bg-primary-500 text-white rounded-br-sm'
					: 'bg-surface-200-800 rounded-bl-sm'}">
					<p class="text-sm whitespace-pre-wrap break-words">
						{message.content}
					</p>
				</div>
			</div>
		{/each}
	</div>
</div>