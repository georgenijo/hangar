<script lang="ts">
	import { goto } from '$app/navigation';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import SessionTile from '$lib/components/SessionTile.svelte';
	import SearchPanel from '$lib/components/SearchPanel.svelte';
	import type { SearchResult } from '$lib/types';

	let searchOpen = $state(false);

	function openSpawn() {
		dispatchEvent(new CustomEvent('hangar:open-spawn'));
	}

	function handleSearchResultClick(result: SearchResult) {
		goto(`/session/${result.session_id}/replay?t=${result.ts}`);
	}
</script>

<div class="grid-page">
	<div class="page-toolbar">
		<button
			class="btn-icon"
			class:active={searchOpen}
			onclick={() => (searchOpen = !searchOpen)}
			title="Search events"
		>
			🔍
		</button>
	</div>

	{#if searchOpen}
		<div class="search-section">
			<SearchPanel
				sessions={sessionsStore.sessions}
				onResultClick={handleSearchResultClick}
			/>
		</div>
	{/if}

	{#if sessionsStore.error}
		<div class="banner error">
			{sessionsStore.error}
			<button onclick={() => { sessionsStore.retry(); sessionsStore.startPolling(); }}>Retry</button>
		</div>
	{/if}

	{#if sessionsStore.loading && sessionsStore.sessions.length === 0}
		<div class="grid">
			{#each Array(4) as _}
				<div class="skeleton-tile"></div>
			{/each}
		</div>
	{:else if sessionsStore.filteredSessions.length === 0}
		<div class="empty-state">
			<p>No sessions</p>
			<button class="btn-primary" onclick={openSpawn}>Create one</button>
		</div>
	{:else}
		<div class="grid">
			{#each sessionsStore.filteredSessions as session (session.id)}
				<SessionTile {session} />
			{/each}
		</div>
	{/if}
</div>

<style>
	.grid-page {
		padding: 16px;
	}

	.page-toolbar {
		display: flex;
		justify-content: flex-end;
		margin-bottom: 12px;
	}

	.btn-icon {
		background: none;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 1rem;
		padding: 4px 10px;
	}

	.btn-icon:hover,
	.btn-icon.active {
		border-color: var(--accent);
		color: var(--text);
	}

	.search-section {
		margin-bottom: 16px;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: 16px;
	}

	.skeleton-tile {
		height: 160px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		animation: shimmer 1.5s ease-in-out infinite;
	}

	@keyframes shimmer {
		0%, 100% { opacity: 0.4; }
		50% { opacity: 0.7; }
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 16px;
		padding: 80px 20px;
		color: var(--text-muted);
	}

	.empty-state p {
		margin: 0;
		font-size: 1.1rem;
	}

	.btn-primary {
		background: var(--accent);
		color: #000;
		border: none;
		border-radius: var(--radius);
		padding: 8px 18px;
		font-size: 0.9rem;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-primary:hover {
		opacity: 0.85;
	}

	.banner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 14px;
		border-radius: var(--radius);
		font-size: 0.85rem;
		margin-bottom: 16px;
	}

	.banner.error {
		background: rgba(244, 67, 54, 0.15);
		border: 1px solid rgba(244, 67, 54, 0.4);
		color: #f44336;
	}

	.banner button {
		background: none;
		border: 1px solid currentColor;
		border-radius: var(--radius);
		color: inherit;
		cursor: pointer;
		font-size: 0.8rem;
		padding: 4px 10px;
	}
</style>
