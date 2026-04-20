<script lang="ts">
	import AgentGraph from '$lib/components/AgentGraph.svelte';
	import type { AgentNode, AgentEdge } from '$lib/types';

	// Static mock data as specified in the architecture
	const nodes: AgentNode[] = [
		{ id: 'conductor', label: 'Conductor', x: 240, y: 20, state: 'live' },
		{ id: 'planner', label: 'Planner', sublabel: 'issue-79', x: 60, y: 100, state: 'done' },
		{ id: 'architect', label: 'Architect', sublabel: 'issue-79', x: 240, y: 100, state: 'done' },
		{ id: 'builder', label: 'Builder', sublabel: 'issue-79', x: 420, y: 100, state: 'live' },
		{ id: 'tester', label: 'Tester', x: 240, y: 200, state: 'idle' }
	];

	const edges: AgentEdge[] = [
		{ from: 'conductor', to: 'planner', state: 'done' },
		{ from: 'conductor', to: 'architect', state: 'done' },
		{ from: 'conductor', to: 'builder', state: 'active' },
		{ from: 'builder', to: 'tester', state: 'pending' }
	];
</script>

<div id="view-conductor" class="view">
	<div class="conductor-graph">
		<AgentGraph {nodes} {edges} width={600} height={300} />
	</div>
	<div class="escalations-card">
		<h3>Escalations</h3>
		<p class="placeholder">No escalations pending</p>
	</div>
</div>

<style>
	.view {
		padding: 24px;
	}

	.conductor-graph {
		margin-bottom: 24px;
		background: var(--surface-2, #1a1a1a);
		border-radius: 8px;
		padding: 24px;
	}

	.escalations-card {
		background: var(--surface-2, #1a1a1a);
		border-radius: 8px;
		padding: 24px;
	}

	.escalations-card h3 {
		margin: 0 0 16px 0;
		font-size: 18px;
		font-weight: 600;
		color: var(--text, #fff);
	}

	.placeholder {
		color: var(--text-muted, #888);
		font-style: italic;
	}
</style>
