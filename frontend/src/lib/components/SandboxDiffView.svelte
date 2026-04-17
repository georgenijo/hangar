<script lang="ts">
	import { getFsDiff, mergeOverlay } from '$lib/api';
	import type { FsDiffEntry, FsDiffResponse } from '$lib/types';

	let { sessionId, onmerged }: { sessionId: string; onmerged?: () => void } = $props();

	let entries = $state<FsDiffEntry[]>([]);
	let total = $state(0);
	let truncated = $state(false);
	let offset = $state(0);
	let loading = $state(false);
	let merging = $state(false);
	let error = $state<string | null>(null);
	let mergeError = $state<string | null>(null);
	let mergeSuccess = $state(false);
	let confirmingMerge = $state(false);

	const LIMIT = 100;

	async function loadDiff() {
		loading = true;
		error = null;
		try {
			const resp: FsDiffResponse = await getFsDiff(sessionId, { limit: LIMIT, offset: 0 });
			entries = resp.entries;
			total = resp.total;
			truncated = resp.truncated;
			offset = resp.entries.length;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load diff';
		} finally {
			loading = false;
		}
	}

	async function loadMore() {
		loading = true;
		error = null;
		try {
			const resp: FsDiffResponse = await getFsDiff(sessionId, { limit: LIMIT, offset });
			entries = [...entries, ...resp.entries];
			total = resp.total;
			truncated = resp.truncated;
			offset += resp.entries.length;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load more';
		} finally {
			loading = false;
		}
	}

	async function doMerge() {
		confirmingMerge = false;
		merging = true;
		mergeError = null;
		try {
			await mergeOverlay(sessionId);
			mergeSuccess = true;
			onmerged?.();
		} catch (e) {
			mergeError = e instanceof Error ? e.message : 'Merge failed';
		} finally {
			merging = false;
		}
	}

	function kindPrefix(kind: FsDiffEntry['kind']): string {
		switch (kind) {
			case 'added': return '+';
			case 'modified': return '~';
			case 'deleted': return '-';
		}
	}

	function kindClass(kind: FsDiffEntry['kind']): string {
		switch (kind) {
			case 'added': return 'added';
			case 'modified': return 'modified';
			case 'deleted': return 'deleted';
		}
	}

	$effect(() => {
		loadDiff();
	});
</script>

<div class="diff-view">
	<div class="diff-header">
		<span class="diff-title">Overlay changes</span>
		{#if total > 0}
			<span class="diff-count">showing {entries.length} of {total}</span>
		{/if}
		{#if !mergeSuccess}
			<button
				class="btn-merge"
				disabled={merging}
				onclick={() => (confirmingMerge = true)}
			>
				{merging ? 'Merging…' : 'Merge to host'}
			</button>
		{/if}
	</div>

	{#if confirmingMerge}
		<div class="confirm-dialog">
			<p>This will create a restic backup and apply all overlay changes to the host filesystem. Continue?</p>
			<div class="confirm-actions">
				<button class="btn-cancel" onclick={() => (confirmingMerge = false)}>Cancel</button>
				<button class="btn-confirm" onclick={doMerge}>Apply changes</button>
			</div>
		</div>
	{/if}

	{#if mergeSuccess}
		<div class="merge-success">✓ Overlay merged to host filesystem</div>
	{/if}

	{#if mergeError}
		<div class="merge-error">Merge failed: {mergeError}</div>
	{/if}

	{#if error}
		<div class="diff-error">{error}</div>
	{/if}

	{#if loading && entries.length === 0}
		<div class="diff-loading">Loading…</div>
	{:else if entries.length === 0 && !loading}
		<div class="diff-empty">No changes in overlay</div>
	{:else}
		<div class="diff-list">
			{#each entries as entry}
				<div class="diff-entry {kindClass(entry.kind)}">
					<span class="diff-prefix">{kindPrefix(entry.kind)}</span>
					<span class="diff-path">{entry.path}</span>
				</div>
			{/each}
		</div>
		{#if truncated}
			<button class="btn-more" onclick={loadMore} disabled={loading}>
				{loading ? 'Loading…' : `Load more (${total - entries.length} remaining)`}
			</button>
		{/if}
	{/if}
</div>

<style>
	.diff-view {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		overflow: hidden;
	}

	.diff-header {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 12px;
		border-bottom: 1px solid var(--border);
		background: var(--bg);
	}

	.diff-title {
		font-weight: 600;
		color: var(--text);
		flex: 1;
	}

	.diff-count {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.diff-list {
		max-height: 300px;
		overflow-y: auto;
		padding: 4px 0;
	}

	.diff-entry {
		display: flex;
		gap: 8px;
		padding: 2px 12px;
		line-height: 1.6;
	}

	.diff-prefix {
		font-weight: 700;
		width: 12px;
		flex-shrink: 0;
	}

	.diff-entry.added .diff-prefix { color: #4caf50; }
	.diff-entry.modified .diff-prefix { color: #ff9800; }
	.diff-entry.deleted .diff-prefix { color: #f44336; }

	.diff-entry.added .diff-path { color: #4caf50; }
	.diff-entry.modified .diff-path { color: var(--text); }
	.diff-entry.deleted .diff-path { color: #f44336; text-decoration: line-through; }

	.btn-merge {
		background: #7c3aed;
		border: none;
		border-radius: var(--radius);
		color: #fff;
		cursor: pointer;
		font-size: 0.75rem;
		font-weight: 600;
		padding: 4px 10px;
	}

	.btn-merge:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-merge:not(:disabled):hover {
		opacity: 0.85;
	}

	.confirm-dialog {
		padding: 12px;
		background: rgba(255, 152, 0, 0.1);
		border-bottom: 1px solid rgba(255, 152, 0, 0.3);
		font-family: inherit;
		font-size: 0.8rem;
		color: var(--text);
	}

	.confirm-dialog p {
		margin: 0 0 8px;
	}

	.confirm-actions {
		display: flex;
		gap: 8px;
		justify-content: flex-end;
	}

	.btn-cancel {
		background: none;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.75rem;
		padding: 4px 10px;
	}

	.btn-confirm {
		background: #7c3aed;
		border: none;
		border-radius: var(--radius);
		color: #fff;
		cursor: pointer;
		font-size: 0.75rem;
		font-weight: 600;
		padding: 4px 10px;
	}

	.merge-success {
		padding: 8px 12px;
		color: #4caf50;
		font-size: 0.8rem;
	}

	.merge-error {
		padding: 8px 12px;
		color: #f44336;
		font-size: 0.8rem;
	}

	.diff-error {
		padding: 8px 12px;
		color: #f44336;
		font-size: 0.8rem;
	}

	.diff-loading,
	.diff-empty {
		padding: 12px;
		color: var(--text-muted);
		text-align: center;
	}

	.btn-more {
		width: 100%;
		background: none;
		border: none;
		border-top: 1px solid var(--border);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.75rem;
		padding: 6px;
	}

	.btn-more:hover:not(:disabled) {
		color: var(--text);
		background: var(--bg);
	}

	.btn-more:disabled {
		opacity: 0.5;
	}
</style>
