<script lang="ts">
	import { eventsStore } from '$lib/stores/events.svelte';

	let expanded = $state(true);

	function contextColor(pct: number): string {
		if (pct > 0.8) return '#f44336';
		if (pct > 0.6) return '#ff9800';
		return 'var(--accent)';
	}
</script>

<div class="insights-panel">
	<div class="insights-header">
		<span class="insights-title">Insights</span>
		<button class="toggle-btn" onclick={() => (expanded = !expanded)}>
			{expanded ? '▾' : '▸'}
		</button>
	</div>

	{#if expanded}
		<div class="insights-body">
			<div class="insights-section">
				<div class="section-heading">Recent Tools</div>
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
									<span class="tool-args muted">{tc.argsPreview.slice(0, 40)}</span>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</div>

			<div class="insights-section">
				<div class="section-heading">Cost (est.)</div>
				<div class="cost-value">${eventsStore.outputCost.estimatedCost.toFixed(4)}</div>
				<div class="muted">{eventsStore.outputCost.totalTokens.toLocaleString()} output tokens</div>
				{#if eventsStore.outputCost.model}
					<div class="muted mono">{eventsStore.outputCost.model}</div>
				{:else}
					<div class="muted">unknown model</div>
				{/if}
			</div>

			<div class="insights-section">
				<div class="section-heading">Context</div>
				{#if eventsStore.contextUsage}
					<div class="ctx-bar-wrap">
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
					<span class="muted">No context data</span>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.insights-panel {
		background: var(--surface);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
		font-size: 0.8rem;
	}

	.insights-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 4px 12px;
		border-bottom: 1px solid var(--border);
	}

	.insights-title {
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-muted);
	}

	.toggle-btn {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.75rem;
		padding: 0 4px;
		line-height: 1;
	}

	.toggle-btn:hover {
		color: var(--text);
	}

	.insights-body {
		display: flex;
		flex-wrap: wrap;
		gap: 0;
		max-height: 120px;
		overflow: hidden;
	}

	.insights-section {
		flex: 1;
		min-width: 140px;
		padding: 6px 12px;
		border-right: 1px solid var(--border);
	}

	.insights-section:last-child {
		border-right: none;
	}

	.section-heading {
		font-size: 0.65rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-muted);
		margin-bottom: 4px;
	}

	.muted {
		color: var(--text-muted);
		font-size: 0.75rem;
	}

	.mono {
		font-family: var(--font-mono);
	}

	.tool-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.tool-item {
		display: flex;
		align-items: center;
		gap: 4px;
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
		white-space: nowrap;
		font-size: 0.7rem;
	}

	.cost-value {
		font-size: 1rem;
		font-weight: 600;
		color: var(--text);
		margin-bottom: 2px;
	}

	.ctx-bar-wrap {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 2px;
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
</style>
