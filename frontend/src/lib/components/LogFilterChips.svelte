<script lang="ts">
	import type { LogSource } from '$lib/types';

	let {
		sources,
		activeSources,
		onToggle
	}: {
		sources: LogSource[];
		activeSources: Set<string>;
		onToggle: (name: string) => void;
	} = $props();

	function kindColor(kind: string): string {
		switch (kind) {
			case 'journalctl':
				return '#4a9eff';
			case 'file':
				return '#4caf50';
			case 'unit':
				return '#ff9800';
			case 'pane_scrollback':
				return '#ab47bc';
			default:
				return '#888';
		}
	}
</script>

<div class="chips">
	{#each sources as source (source.name)}
		{@const active = activeSources.size === 0 || activeSources.has(source.name)}
		<button
			class="chip"
			class:active
			onclick={() => onToggle(source.name)}
			title={source.kind}
		>
			<span class="dot" style="background:{kindColor(source.kind)}"></span>
			{source.name}
		</button>
	{/each}
</div>

<style>
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		align-items: center;
	}

	.chip {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 3px 10px;
		border-radius: 12px;
		border: 1px solid var(--border);
		background: transparent;
		color: var(--text-muted);
		font-size: 0.78rem;
		cursor: pointer;
		transition: all 0.15s;
	}

	.chip.active {
		background: color-mix(in srgb, var(--accent) 15%, transparent);
		border-color: var(--accent);
		color: var(--text);
	}

	.chip:hover {
		border-color: var(--accent);
		color: var(--text);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex-shrink: 0;
	}
</style>
