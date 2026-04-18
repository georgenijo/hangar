<script lang="ts">
	import LogsView from '$lib/components/LogsView.svelte';
	import type { LogSource } from '$lib/types';

	let { data }: { data: { sources: LogSource[] } } = $props();
</script>

<div class="logs-page">
	{#if data.sources.length === 0}
		<div class="empty-state">
			<p>No log sources configured.</p>
			<small>Add sources to the <code>[logs]</code> section in <code>config.toml</code>.</small>
		</div>
	{:else}
		<LogsView sources={data.sources} />
	{/if}
</div>

<style>
	.logs-page {
		height: calc(100vh - 48px);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		gap: 8px;
		color: var(--text-muted);
	}

	.empty-state p {
		margin: 0;
		font-size: 1rem;
		color: var(--text);
	}

	.empty-state small {
		font-size: 0.85rem;
	}

	.empty-state code {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 1px 5px;
		font-family: var(--font-mono);
		font-size: 0.82rem;
	}
</style>
