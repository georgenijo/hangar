<script lang="ts">
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import type { LabelEntry } from '$lib/types';

	function isSelected(label: LabelEntry): boolean {
		return sessionsStore.selectedLabels.some(
			(l) => l.key === label.key && l.value === label.value
		);
	}
</script>

<div class="sidebar-content">
	<div class="sidebar-header">Labels</div>

	{#if sessionsStore.allLabels.length === 0}
		<p class="empty">No labels</p>
	{:else}
		<ul class="label-list">
			{#each sessionsStore.allLabels as label}
				<li>
					<button
						class="label-chip"
						class:selected={isSelected(label)}
						onclick={() => sessionsStore.toggleLabelFilter(label)}
					>
						{#if label.value}
							{label.key}=<span class="label-value">{label.value}</span>
						{:else}
							{label.key}
						{/if}
					</button>
				</li>
			{/each}
		</ul>

		{#if sessionsStore.selectedLabels.length > 0}
			<button class="clear-btn" onclick={() => sessionsStore.clearLabelFilters()}>
				Clear all
			</button>
		{/if}
	{/if}
</div>

<style>
	.sidebar-content {
		padding: 12px 8px;
	}

	.sidebar-header {
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--text-muted);
		padding: 0 4px 8px;
	}

	.empty {
		font-size: 0.8rem;
		color: var(--text-muted);
		padding: 0 4px;
	}

	.label-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.label-chip {
		display: block;
		width: 100%;
		text-align: left;
		background: none;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		cursor: pointer;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		padding: 4px 8px;
		transition: all 0.1s;
	}

	.label-chip:hover {
		border-color: var(--accent);
		color: var(--text);
	}

	.label-chip.selected {
		background: var(--accent);
		border-color: var(--accent);
		color: #000;
	}

	.label-value {
		font-weight: 600;
	}

	.clear-btn {
		margin-top: 12px;
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.75rem;
		padding: 4px;
		text-decoration: underline;
	}

	.clear-btn:hover {
		color: var(--text);
	}
</style>
