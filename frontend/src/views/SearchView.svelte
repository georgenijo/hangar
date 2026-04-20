<script lang="ts">
	import type { SearchResult } from '$lib/types';
	import { search, ApiError } from '$lib/api';

	let query = $state('');
	let results = $state<SearchResult[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let activeFilter = $state<'all' | 'sessions' | 'events' | 'code' | 'prs'>('all');

	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let abortController: AbortController | null = null;

	function formatRelativeTime(ts: number): string {
		const diff = Date.now() - ts;
		const s = Math.floor(diff / 1000);
		if (s < 60) return `${s}s ago`;
		const m = Math.floor(s / 60);
		if (m < 60) return `${m}m ago`;
		const h = Math.floor(m / 60);
		if (h < 24) return `${h}h ago`;
		const d = Math.floor(h / 24);
		return `${d}d ago`;
	}

	function kindBadge(kind: string): string {
		// Map kind to display badge
		if (kind.includes('AgentEvent')) return 'event';
		if (kind.includes('Session')) return 'session';
		if (kind.includes('Output')) return 'code';
		return 'event';
	}

	$effect(() => {
		const q = query;
		const filter = activeFilter;

		if (debounceTimer) clearTimeout(debounceTimer);
		if (abortController) abortController.abort();

		if (!q.trim()) {
			results = [];
			error = null;
			loading = false;
			return;
		}

		debounceTimer = setTimeout(async () => {
			abortController = new AbortController();
			const signal = abortController.signal;
			loading = true;
			error = null;

			try {
				// Map filter to kinds parameter
				let kinds: string[] | undefined;
				if (filter === 'events') {
					kinds = ['AgentEvent'];
				} else if (filter === 'sessions') {
					kinds = ['StateChanged', 'SessionCreated'];
				} else if (filter === 'code') {
					kinds = ['OutputAppended'];
				}
				// 'all' and 'prs' use no filter (prs not implemented yet)

				results = await search({
					q,
					kinds,
					limit: 100,
					signal
				});
			} catch (e) {
				if (e instanceof DOMException && e.name === 'AbortError') return;
				if (e instanceof ApiError && e.status === 400) {
					error = `Invalid search: ${e.body}`;
				} else {
					error = 'Search failed';
				}
				results = [];
			} finally {
				loading = false;
			}
		}, 300);
	});
</script>

<div id="view-search" class="view">
	<div class="search-header">
		<div class="search-input-row">
			<input
				class="search-input"
				type="text"
				placeholder="Search events, sessions, code…"
				bind:value={query}
				autofocus
			/>
			{#if loading}
				<span class="spinner">⟳</span>
			{/if}
		</div>

		<div class="filter-chips">
			<button
				class="filter-chip"
				class:active={activeFilter === 'all'}
				onclick={() => (activeFilter = 'all')}
			>
				All
			</button>
			<button
				class="filter-chip"
				class:active={activeFilter === 'sessions'}
				onclick={() => (activeFilter = 'sessions')}
			>
				Sessions
			</button>
			<button
				class="filter-chip"
				class:active={activeFilter === 'events'}
				onclick={() => (activeFilter = 'events')}
			>
				Events
			</button>
			<button
				class="filter-chip"
				class:active={activeFilter === 'code'}
				onclick={() => (activeFilter = 'code')}
			>
				Code
			</button>
			<button
				class="filter-chip"
				class:active={activeFilter === 'prs'}
				onclick={() => (activeFilter = 'prs')}
			>
				PRs
			</button>
		</div>
	</div>

	{#if error}
		<div class="search-error">{error}</div>
	{/if}

	{#if !loading && query.trim() && results.length === 0 && !error}
		<div class="empty-state">No results found</div>
	{/if}

	<div class="results">
		{#each results as result (result.event_id)}
			<div class="result-card">
				<div class="result-header">
					<span class="result-kind-badge {kindBadge(result.kind)}">{kindBadge(result.kind)}</span>
					<span class="result-kind-detail">{result.kind}</span>
					<span class="result-ts">{formatRelativeTime(result.ts)}</span>
				</div>
				<div class="result-snippet">
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html result.snippet}
				</div>
			</div>
		{/each}
	</div>
</div>

<style>
	.view {
		display: flex;
		flex-direction: column;
		gap: 16px;
		height: 100%;
		overflow: hidden;
	}

	.search-header {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.search-input-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.search-input {
		flex: 1;
		background: var(--surface-2);
		border: 1px solid var(--border);
		border-radius: 6px;
		color: var(--text);
		font-size: 0.95rem;
		font-family: var(--mono);
		padding: 10px 14px;
	}

	.search-input:focus {
		outline: none;
		border-color: var(--accent);
		background: var(--surface-3);
	}

	.spinner {
		color: var(--text-3);
		animation: spin 1s linear infinite;
		display: inline-block;
		font-size: 1.2rem;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	.filter-chips {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.filter-chip {
		background: var(--surface-2);
		border: 1px solid var(--border);
		border-radius: 16px;
		padding: 6px 16px;
		font-size: 0.85rem;
		color: var(--text-2);
		cursor: pointer;
		transition:
			all 0.15s ease,
			transform 0.1s ease;
	}

	.filter-chip:hover {
		background: var(--surface-3);
		border-color: var(--accent);
		color: var(--text);
	}

	.filter-chip.active {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--bg);
		font-weight: 500;
	}

	.search-error {
		color: var(--warn);
		font-size: 0.85rem;
		padding: 8px 12px;
		background: rgba(244, 67, 54, 0.1);
		border: 1px solid rgba(244, 67, 54, 0.3);
		border-radius: 6px;
	}

	.empty-state {
		color: var(--text-3);
		font-size: 0.9rem;
		text-align: center;
		padding: 40px 0;
	}

	.results {
		display: flex;
		flex-direction: column;
		gap: 8px;
		overflow-y: auto;
		flex: 1;
	}

	.result-card {
		background: var(--surface-2);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 12px 16px;
		transition: border-color 0.15s ease;
		cursor: pointer;
	}

	.result-card:hover {
		border-color: var(--accent);
	}

	.result-header {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-bottom: 8px;
		font-size: 0.8rem;
	}

	.result-kind-badge {
		display: inline-block;
		padding: 2px 8px;
		border-radius: 10px;
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.02em;
	}

	.result-kind-badge.event {
		background: rgba(91, 139, 212, 0.2);
		color: #5b8bd4;
	}

	.result-kind-badge.session {
		background: rgba(156, 39, 176, 0.2);
		color: #9c27b0;
	}

	.result-kind-badge.code {
		background: rgba(76, 175, 80, 0.2);
		color: #4caf50;
	}

	.result-kind-detail {
		color: var(--text-3);
		font-size: 0.75rem;
		font-family: var(--mono);
	}

	.result-ts {
		margin-left: auto;
		color: var(--text-3);
		font-size: 0.75rem;
	}

	.result-snippet {
		font-size: 0.88rem;
		color: var(--text-2);
		line-height: 1.5;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.result-snippet :global(mark) {
		background: rgba(255, 220, 0, 0.35);
		color: var(--text);
		padding: 1px 3px;
		border-radius: 2px;
		font-weight: 500;
	}
</style>
