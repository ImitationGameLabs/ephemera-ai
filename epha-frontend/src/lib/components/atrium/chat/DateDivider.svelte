<script lang="ts">
	interface DateDividerProps {
		date: string;
	}

	let { date }: DateDividerProps = $props();

	function formatDate(timestamp: string): string {
		const d = new Date(timestamp);
		const today = new Date();
		const yesterday = new Date(today);
		yesterday.setDate(yesterday.getDate() - 1);

		if (d.toDateString() === today.toDateString()) {
			return 'Today';
		} else if (d.toDateString() === yesterday.toDateString()) {
			return 'Yesterday';
		} else {
			return d.toLocaleDateString();
		}
	}
</script>

<div class="flex items-center justify-center my-4">
	<div class="flex items-center gap-2 text-xs text-surface-500">
		<div class="h-px bg-surface-300-700 flex-1"></div>
		<div class="px-2 py-1 bg-surface-100-900 rounded-full">
			<span>{formatDate(date)}</span>
		</div>
		<div class="h-px bg-surface-300-700 flex-1"></div>
	</div>
</div>