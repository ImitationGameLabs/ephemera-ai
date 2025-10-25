<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { Navigation } from '@skeletonlabs/skeleton-svelte';
	import { MessageSquare, Menu, Settings, Brain } from '@lucide/svelte';

	let { children } = $props();

	const links = [
		{ label: 'Atrium', href: '/atrium', icon: MessageSquare },
		{ label: 'Memory', href: '/memory', icon: Brain }
	];

	let anchorRail = 'btn hover:preset-tonal aspect-square w-full max-w-[84px] flex flex-col items-center gap-0.5';
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<div class="w-full h-screen grid grid-cols-[auto_1fr] border border-surface-200-800">
  <!-- --- -->
  <Navigation layout="rail">
    <Navigation.Header>
      <a href="/" class={anchorRail} title="Menu" aria-label="Menu">
        <Menu class="size-5" />
      </a>
    </Navigation.Header>
    <Navigation.Content>
      <Navigation.Menu>
        {#each links as link (link)}
          {@const Icon = link.icon}
          <a href={link.href} class={anchorRail}>
            <Icon class="size-5" />
            <span class="text-xs">{link.label}</span>
          </a>
        {/each}
      </Navigation.Menu>
    </Navigation.Content>
    <Navigation.Footer>
      <a href="/settings" class={anchorRail} title="Settings" aria-label="Settings">
        <Settings class="size-5" />
      </a>
    </Navigation.Footer>
  </Navigation>
  <!-- --- -->
  <div class="h-full">
	{@render children()}
  </div>
</div>
