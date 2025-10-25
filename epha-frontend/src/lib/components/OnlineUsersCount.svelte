<script lang="ts">
	import { Users, Circle } from '@lucide/svelte';
	import type { User } from '$lib/api/types';

	let {
		onlineUsers = $bindable([] as User[]),
		currentUser = $bindable(null as User | null),
		loading = $bindable(false)
	} = $props();

	// Count all online users including current user
	const onlineCount = $derived(() => {
		return onlineUsers.length;
	});

	// Format display text
	const displayText = $derived(() => {
		const count = onlineCount();
		return `${count} ${count === 1 ? 'user' : 'users'} online`;
	});

	// Get online users excluding current user for avatar display
	const otherOnlineUsers = $derived(() => {
		if (!onlineUsers.length || !currentUser) return [];
		return onlineUsers.filter(user => user.name !== currentUser.name);
	});
</script>

<div class="flex items-center gap-2">
	{#if loading}
		<div class="w-4 h-4">
			<div class="loading loading-spinner w-4 h-4"></div>
		</div>
	{:else}
		<Users class="w-4 h-4 text-green-500" />
	{/if}

	<span class="text-sm {loading ? 'text-surface-600-400' : onlineCount() > 0 ? 'text-surface-700-300' : 'text-surface-500'}">
		{displayText()}
	</span>

	<!-- Show user avatars if there are other users -->
	{#if otherOnlineUsers().length > 0}
		<div class="flex -space-x-2 ml-3">
			{#each otherOnlineUsers().slice(0, 3) as user}
				<div
					class="w-6 h-6 rounded-full bg-surface-300-700 border-2 border-surface-50-950 flex items-center justify-center"
					title={user.name}
				>
					<Circle class="w-1 h-1 text-green-500 fill-green-500" />
				</div>
			{/each}

			{#if otherOnlineUsers().length > 3}
				<div
					class="w-6 h-6 rounded-full bg-surface-400-600 border-2 border-surface-50-950 flex items-center justify-center text-xs text-surface-100-900 font-medium"
					title={`+${otherOnlineUsers().length - 3} more users`}
				>
					+{otherOnlineUsers().length - 3}
				</div>
			{/if}
		</div>
	{/if}
</div>