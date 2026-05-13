<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { sidebarStore } from '$lib/stores/sidebar.svelte';
	import LabelSidebar from '$lib/components/LabelSidebar.svelte';
	import SpawnModal from '$lib/components/SpawnModal.svelte';
	import type { Session } from '$lib/types';

	let { children } = $props();

	// hook test edit
	let spawnOpen = $state(false);
	let collapsed = $derived(sidebarStore.dashboardCollapsed);

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

	$effect(() => {
		let leaderPending = false;
		let leaderTimer: ReturnType<typeof setTimeout> | null = null;

		function onKey(e: KeyboardEvent) {
			const tag = (e.target as HTMLElement).tagName;
			if (tag === 'INPUT' || tag === 'TEXTAREA' || !!(e.target as HTMLElement).closest('[contenteditable]')) return;

			if (e.ctrlKey && e.key === '\\') {
				e.preventDefault();
				const path = $page.url.pathname;
				if (path.startsWith('/session/')) {
					sidebarStore.toggleSession();
				} else {
					sidebarStore.toggleDashboard();
				}
				return;
			}

			if (leaderPending && e.key === 'n') {
				e.preventDefault();
				leaderPending = false;
				if (leaderTimer) { clearTimeout(leaderTimer); leaderTimer = null; }
				spawnOpen = true;
				return;
			}
			if (e.key === 'g' && !e.ctrlKey && !e.metaKey && !e.altKey) {
				leaderPending = true;
				if (leaderTimer) clearTimeout(leaderTimer);
				leaderTimer = setTimeout(() => { leaderPending = false; }, 1500);
				return;
			}
			if (leaderPending) {
				leaderPending = false;
				if (leaderTimer) { clearTimeout(leaderTimer); leaderTimer = null; }
			}
		}
		document.addEventListener('keydown', onKey);
		return () => {
			document.removeEventListener('keydown', onKey);
			if (leaderTimer) clearTimeout(leaderTimer);
		};
	});

	function handleCreated(_session: Session) {
		spawnOpen = false;
	}
</script>

<svelte:head>
	<title>Hangar</title>
</svelte:head>

<div class="app-shell" class:sidebar-collapsed={collapsed}>
	<aside class="sidebar">
		<div class="sidebar-header">
			<button
				class="collapse-btn"
				onclick={() => sidebarStore.toggleDashboard()}
				aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
				title={collapsed ? 'Expand (Ctrl+\\)' : 'Collapse (Ctrl+\\)'}
			>
				{collapsed ? '»' : '«'}
			</button>
		</div>
		{#if !collapsed}
			<nav class="nav-links">
				<a href="/" class="nav-link" class:active={$page.url.pathname === '/'}>Sessions</a>
				<a href="/logs" class="nav-link" class:active={$page.url.pathname === '/logs'}>Logs</a>
			</nav>
			<LabelSidebar />
		{/if}
	</aside>

	<main class="content">
		<header class="topbar">
			<div class="topbar-left">
				<span class="logo">Hangar</span>
			</div>
			<button class="btn-primary" onclick={() => (spawnOpen = true)} title="New session (g n)">＋ New Session</button>
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
		overflow-x: hidden;
		transition: width 0.25s ease;
	}

	.app-shell.sidebar-collapsed .sidebar {
		width: 36px;
	}

	.sidebar-header {
		display: flex;
		justify-content: flex-end;
		padding: 6px;
		border-bottom: 1px solid var(--border);
	}

	.app-shell.sidebar-collapsed .sidebar-header {
		justify-content: center;
	}

	.collapse-btn {
		background: none;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.9rem;
		line-height: 1;
		padding: 2px 8px;
	}

	.collapse-btn:hover {
		color: var(--text);
		border-color: var(--accent);
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

	.nav-links {
		display: flex;
		flex-direction: column;
		padding: 8px;
		gap: 2px;
		border-bottom: 1px solid var(--border);
	}

	.nav-link {
		display: block;
		padding: 6px 8px;
		border-radius: var(--radius);
		color: var(--text-muted);
		text-decoration: none;
		font-size: 0.85rem;
		transition: all 0.1s;
	}

	.nav-link:hover {
		background: var(--bg-hover);
		color: var(--text);
	}

	.nav-link.active {
		background: color-mix(in srgb, var(--accent) 15%, transparent);
		color: var(--accent);
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
			width: 36px;
			transform: translateX(0);
		}
	}
</style>
