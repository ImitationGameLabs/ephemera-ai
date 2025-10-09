<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { page } from '$app/state';
	import { Navigation } from '@skeletonlabs/skeleton-svelte';
	import { MessageSquare, Users, Menu, Settings, Brain } from '@lucide/svelte';
	// import UserStatus from '$lib/components/UserStatus.svelte'; // Temporarily disabled
	// import { auth } from '$lib/stores/auth'; // Temporarily disabled

	let { children } = $props();

	const navLinks = [
		{
			label: 'Atrium',
			href: '/atrium',
			icon: MessageSquare
		},

		{
			label: 'Memory',
			href: '/memory',
			icon: Brain
		}
	];

	// Restore auth session on app load
	// auth.restoreSession(); // Temporarily disabled
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<div class="card border-surface-100-900 grid h-screen w-full grid-cols-[auto_1fr] border-[1px]">
  <!-- Component -->
  <Navigation.Rail>
    {#snippet header()}
      <Navigation.Tile href="#" title="Menu"><Menu /></Navigation.Tile>
    {/snippet}
    {#snippet tiles()}
      <Navigation.Tile label="Atrium" href="/atrium"><MessageSquare /></Navigation.Tile>
      <Navigation.Tile label="Memory" href="/memory"><Brain /></Navigation.Tile>
    {/snippet}
    {#snippet footer()}
      <Navigation.Tile labelExpanded="Settings" href="/settings" title="settings"><Settings /></Navigation.Tile>
    {/snippet}
  </Navigation.Rail>
  <!-- Content -->
  <div class="h-screen m-2">
	{@render children()}
  </div>
</div>
