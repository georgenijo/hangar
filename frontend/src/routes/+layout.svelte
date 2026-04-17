<script lang="ts">
	import '../app.css';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import LabelSidebar from '$lib/components/LabelSidebar.svelte';
	import SpawnModal from '$lib/components/SpawnModal.svelte';
	import type { Session } from '$lib/types';

	let { children } = $props();

	let spawnOpen = $state(false);
	let sidebarOpen = $state(true);

	$effect(() => {
		sessionsStore.startPolling();
		return () => sessionsStore.stopPolling();
	});

	function handleCreated(_session: Session) {
		spawnOpen = false;
	}
</script>

<svelte:head>
	<title>Hangar</title>
</svelte:head>

<div class="app-shell" class:sidebar-collapsed={!sidebarOpen}>
	<aside class="sidebar">
		<LabelSidebar />
	</aside>

	<main class="content">
		<header class="topbar">
			<div class="topbar-left">
				<button
					class="hamburger"
					onclick={() => (sidebarOpen = !sidebarOpen)}
					aria-label="Toggle sidebar"
				>
					☰
				</button>
				<span class="logo">Hangar</span>
			</div>
			<button class="btn-primary" onclick={() => (spawnOpen = true)}>＋ New Session</button>
		</header>

		<div class="page-content">
			{@render children()}
		</div>
	</main>
</div>

{#if spawnOpen}
	<SpawnModal
		open={spawnOpen}
		onclose={() => (spawnOpen = false)}
		oncreated={handleCreated}
	/>
{/if}

<style>
	:root {
		--bg: #0e0e10;
		--bg-surface: #1a1a1e;
		--bg-hover: #222228;
		--accent: #9cf;
		--border: #2a2a30;
		--text: #eee;
		--text-muted: #888;
		--font-mono: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
		--font-sans: system-ui, -apple-system, sans-serif;
		--radius: 6px;
	}

	.app-shell {
		display: flex;
		height: 100vh;
		overflow: hidden;
	}

	.sidebar {
		width: 200px;
		flex-shrink: 0;
		background: var(--bg-surface);
		border-right: 1px solid var(--border);
		overflow-y: auto;
		transition: width 0.2s;
	}

	.app-shell.sidebar-collapsed .sidebar {
		width: 0;
		overflow: hidden;
	}

	.content {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.topbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 16px;
		height: 48px;
		border-bottom: 1px solid var(--border);
		background: var(--bg-surface);
		flex-shrink: 0;
	}

	.topbar-left {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.hamburger {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 1rem;
		padding: 4px;
	}

	.hamburger:hover {
		color: var(--text);
	}

	.logo {
		font-size: 1.1rem;
		font-weight: 700;
		color: var(--accent);
		letter-spacing: 0.05em;
	}

	.btn-primary {
		background: var(--accent);
		color: #000;
		border: none;
		border-radius: var(--radius);
		padding: 6px 14px;
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-primary:hover {
		opacity: 0.85;
	}

	.page-content {
		flex: 1;
		overflow-y: auto;
	}

	@media (max-width: 768px) {
		.sidebar {
			position: fixed;
			left: 0;
			top: 0;
			bottom: 0;
			z-index: 100;
			transform: translateX(0);
		}

		.app-shell.sidebar-collapsed .sidebar {
			width: 200px;
			transform: translateX(-200px);
		}
	}
</style>
