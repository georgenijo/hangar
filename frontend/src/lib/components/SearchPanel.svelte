<script lang="ts">
	import type { Session, SearchResult } from '$lib/types';
	import { search, ApiError } from '$lib/api';

	let {
		sessions,
		onResultClick
	}: {
		sessions: Session[];
		onResultClick?: (result: SearchResult) => void;
	} = $props();

	let query = $state('');
	let results = $state<SearchResult[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let selectedSessionIds = $state<string[]>([]);
	let selectedKinds = $state<string[]>([]);

	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let abortController: AbortController | null = null;

	const ALL_KINDS = ['AgentEvent', 'OutputAppended', 'StateChanged'];

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

	$effect(() => {
		const q = query;
		const sids = selectedSessionIds.slice();
		const ks = selectedKinds.slice();

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
				results = await search({
					q,
					sessionIds: sids.length ? sids : undefined,
					kinds: ks.length ? ks : undefined,
					limit: 50,
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

	function toggleSessionId(id: string) {
		if (selectedSessionIds.includes(id)) {
			selectedSessionIds = selectedSessionIds.filter((s) => s !== id);
		} else {
			selectedSessionIds = [...selectedSessionIds, id];
		}
	}

	function toggleKind(k: string) {
		if (selectedKinds.includes(k)) {
			selectedKinds = selectedKinds.filter((s) => s !== k);
		} else {
			selectedKinds = [...selectedKinds, k];
		}
	}

	function sessionSlug(sessionId: string): string {
		return sessions.find((s) => s.id === sessionId)?.slug ?? sessionId.slice(0, 8);
	}
</script>

<div class="search-panel">
	<div class="search-input-row">
		<input
			class="search-input"
			type="text"
			placeholder="Search events…"
			bind:value={query}
		/>
		{#if loading}
			<span class="spinner">⟳</span>
		{/if}
	</div>

	<div class="filters">
		<details class="filter-group">
			<summary>Sessions</summary>
			<div class="filter-options">
				{#each sessions as session (session.id)}
					<label class="filter-option">
						<input
							type="checkbox"
							checked={selectedSessionIds.includes(session.id)}
							onchange={() => toggleSessionId(session.id)}
						/>
						<span class="mono">{session.slug}</span>
					</label>
				{/each}
			</div>
		</details>

		<details class="filter-group">
			<summary>Kind</summary>
			<div class="filter-options">
				{#each ALL_KINDS as kind (kind)}
					<label class="filter-option">
						<input
							type="checkbox"
							checked={selectedKinds.includes(kind)}
							onchange={() => toggleKind(kind)}
						/>
						<span>{kind}</span>
					</label>
				{/each}
			</div>
		</details>
	</div>

	{#if error}
		<div class="search-error">{error}</div>
	{/if}

	{#if !loading && query.trim() && results.length === 0 && !error}
		<div class="empty-state">No results</div>
	{/if}

	<div class="results">
		{#each results as result (result.event_id)}
			<button
				class="result-row"
				onclick={() => onResultClick?.(result)}
				type="button"
			>
				<div class="result-meta">
					<span class="session-slug mono">{sessionSlug(result.session_id)}</span>
					<span class="kind-badge">{result.kind}</span>
					<span class="ts">{formatRelativeTime(result.ts)}</span>
				</div>
				<div class="snippet">
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html result.snippet}
				</div>
			</button>
		{/each}
	</div>
</div>

<style>
	.search-panel {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
	}

	.search-input-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.search-input {
		flex: 1;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text);
		font-size: 0.9rem;
		padding: 6px 10px;
	}

	.search-input:focus {
		outline: none;
		border-color: var(--accent);
	}

	.spinner {
		color: var(--text-muted);
		animation: spin 1s linear infinite;
		display: inline-block;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	.filters {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.filter-group {
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.filter-group summary {
		cursor: pointer;
		padding: 2px 6px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
	}

	.filter-options {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 6px;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		margin-top: 4px;
		position: absolute;
		z-index: 10;
	}

	.filter-option {
		display: flex;
		align-items: center;
		gap: 6px;
		cursor: pointer;
		white-space: nowrap;
	}

	.search-error {
		color: #f44336;
		font-size: 0.8rem;
		padding: 4px 0;
	}

	.empty-state {
		color: var(--text-muted);
		font-size: 0.85rem;
		text-align: center;
		padding: 16px 0;
	}

	.results {
		display: flex;
		flex-direction: column;
		gap: 4px;
		max-height: 400px;
		overflow-y: auto;
	}

	.result-row {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 8px 10px;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		cursor: pointer;
		text-align: left;
		color: var(--text);
		width: 100%;
	}

	.result-row:hover {
		border-color: var(--accent);
	}

	.result-meta {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.session-slug {
		font-weight: 600;
		color: var(--text);
	}

	.kind-badge {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 1px 6px;
		font-size: 0.7rem;
	}

	.ts {
		margin-left: auto;
	}

	.snippet {
		font-size: 0.82rem;
		color: var(--text-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.snippet :global(mark) {
		background: rgba(255, 220, 0, 0.3);
		color: var(--text);
		padding: 0 2px;
		border-radius: 2px;
	}

	.mono {
		font-family: var(--font-mono);
	}
</style>
