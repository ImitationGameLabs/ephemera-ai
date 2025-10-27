<script lang="ts">
	import { CircleCheck } from '@lucide/svelte';
</script>

<div class="flex items-center justify-center my-4">
	<div class="flex items-center gap-2 text-xs text-surface-500">
		<div class="h-px bg-surface-300-700 flex-1"></div>
		<div class="flex items-center gap-1">
			<CircleCheck class="w-3 h-3" />
			<span>Unread messages</span>
		</div>
		<div class="h-px bg-surface-300-700 flex-1"></div>
	</div>
</div>