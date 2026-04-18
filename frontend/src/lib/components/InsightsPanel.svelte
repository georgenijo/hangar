<script lang="ts">
	import { eventsStore } from '$lib/stores/events.svelte';

	function contextColor(pct: number): string {
		if (pct > 0.8) return '#f44336';
		if (pct > 0.6) return '#ff9800';
		return 'var(--accent)';
	}
</script>

<div class="sidebar">
	<div class="section">
		<div class="heading">Context</div>
		{#if eventsStore.contextUsage}
			<div class="ctx-row">
				<div class="ctx-bar">
					<div
						class="ctx-fill"
						style="width: {Math.min(eventsStore.contextUsage.pctUsed * 100, 100)}%; background: {contextColor(eventsStore.contextUsage.pctUsed)}"
					></div>
				</div>
				<span class="ctx-pct">{Math.round(eventsStore.contextUsage.pctUsed * 100)}%</span>
			</div>
			<div class="muted">{eventsStore.contextUsage.tokens.toLocaleString()} tokens</div>
		{:else}
			<span class="muted">—</span>
		{/if}
	</div>

	<div class="section">
		<div class="heading">Cost (est.)</div>
		<div class="cost-value">${eventsStore.outputCost.estimatedCost.toFixed(4)}</div>
		<div class="muted">{eventsStore.outputCost.totalTokens.toLocaleString()} output tokens</div>
		{#if eventsStore.outputCost.model}
			<div class="muted mono small">{eventsStore.outputCost.model}</div>
		{/if}
	</div>

	<div class="section">
		<div class="heading">Recent Tools</div>
		{#if eventsStore.recentToolCalls.length === 0}
			<span class="muted">No tool calls yet</span>
		{:else}
			<ul class="tool-list">
				{#each eventsStore.recentToolCalls as tc}
					<li class="tool-item">
						<span class="tool-name mono">{tc.tool}</span>
						{#if tc.finished}
							<span class="tool-status" class:ok={tc.ok} class:fail={!tc.ok}>
								{tc.ok ? '✓' : '✗'}
							</span>
						{/if}
						{#if tc.argsPreview}
							<div class="tool-args muted">{tc.argsPreview.slice(0, 80)}</div>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		gap: 1px;
		font-size: 0.8rem;
		background: var(--border);
	}

	.section {
		padding: 10px 12px;
		background: var(--surface, var(--bg));
	}

	.heading {
		font-size: 0.65rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-muted);
		margin-bottom: 6px;
	}

	.muted {
		color: var(--text-muted);
		font-size: 0.75rem;
	}

	.small {
		font-size: 0.7rem;
	}

	.mono {
		font-family: var(--font-mono);
	}

	.ctx-row {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 3px;
	}

	.ctx-bar {
		flex: 1;
		height: 6px;
		background: var(--border);
		border-radius: 3px;
		overflow: hidden;
	}

	.ctx-fill {
		height: 100%;
		border-radius: 3px;
		transition: width 0.5s;
	}

	.ctx-pct {
		font-size: 0.75rem;
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.cost-value {
		font-size: 1rem;
		font-weight: 600;
		color: var(--text);
		margin-bottom: 2px;
	}

	.tool-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.tool-item {
		display: flex;
		flex-direction: column;
		gap: 2px;
		overflow: hidden;
	}

	.tool-name {
		font-size: 0.75rem;
		white-space: nowrap;
	}

	.tool-status {
		font-size: 0.7rem;
		flex-shrink: 0;
	}

	.tool-status.ok {
		color: #4caf50;
	}

	.tool-status.fail {
		color: #f44336;
	}

	.tool-args {
		overflow: hidden;
		text-overflow: ellipsis;
		font-size: 0.7rem;
		word-break: break-word;
	}
</style>
