<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import SpawnModal from '$lib/components/SpawnModal.svelte';
	import type { Session } from '$lib/types';

	let { children } = $props();
	let spawnOpen = $state(false);

	$effect(() => {
		sessionsStore.startPolling();
		return () => sessionsStore.stopPolling();
	});

	$effect(() => {
		function onSpawn() {
			spawnOpen = true;
		}
		window.addEventListener('hangar:open-spawn', onSpawn);
		return () => window.removeEventListener('hangar:open-spawn', onSpawn);
	});

	function handleCreated(_session: Session) {
		spawnOpen = false;
	}

	const onIndex = $derived($page.url.pathname === '/');
</script>

<svelte:head>
	<title>Hangar</title>
</svelte:head>

<div class="app-shell">
	{#if onIndex}
		<Sidebar />
		<main class="content">
			<Topbar />
			<div class="page-content">
				{@render children()}
			</div>
		</main>
	{:else}
		<main class="content full">
			{@render children()}
		</main>
	{/if}
</div>

{#if spawnOpen}
	<SpawnModal
		open={spawnOpen}
		onclose={() => (spawnOpen = false)}
		oncreated={handleCreated}
	/>
{/if}

<style>
	.app-shell {
		display: flex;
		height: 100vh;
		overflow: hidden;
	}
	.content {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.content.full {
		width: 100%;
	}
	.page-content {
		flex: 1;
		overflow-y: auto;
	}
</style>
