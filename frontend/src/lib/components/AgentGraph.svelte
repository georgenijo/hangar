<script lang="ts">
	import type { AgentNode, AgentEdge } from '$lib/types';

	interface Props {
		nodes: AgentNode[];
		edges: AgentEdge[];
		width?: number;
		height?: number;
	}

	let { nodes, edges, width = 600, height = 300 }: Props = $props();

	// Generate SVG path for edges (curved paths)
	function generateEdgePath(from: AgentNode, to: AgentNode): string {
		const fromX = from.x + 60; // Node width is ~120, center at +60
		const fromY = from.y + 20; // Node height is ~40, center at +20
		const toX = to.x + 60;
		const toY = to.y + 20;

		// Create a curved path using quadratic bezier
		const midX = (fromX + toX) / 2;
		const midY = (fromY + toY) / 2;
		const controlX = midX;
		const controlY = midY + Math.abs(toY - fromY) * 0.3;

		return `M ${fromX} ${fromY} Q ${controlX} ${controlY} ${toX} ${toY}`;
	}

	// Find node by id
	function findNode(id: string): AgentNode | undefined {
		return nodes.find((n) => n.id === id);
	}

	const edgePaths = $derived(
		edges.map((edge) => {
			const from = findNode(edge.from);
			const to = findNode(edge.to);
			if (!from || !to) return null;
			return {
				path: generateEdgePath(from, to),
				state: edge.state
			};
		})
	);
</script>

<svg viewBox="0 0 {width} {height}" class="graph-svg">
	<!-- Render edges first (behind nodes) -->
	{#each edgePaths as edgePath}
		{#if edgePath}
			<path
				d={edgePath.path}
				class="edge"
				class:edge-done={edgePath.state === 'done'}
				class:edge-live={edgePath.state === 'active'}
			/>
		{/if}
	{/each}

	<!-- Render nodes -->
	{#each nodes as node}
		<g class="node node-{node.state}">
			<!-- Node rectangle -->
			<rect x={node.x} y={node.y} width="120" height="40" rx="6" />

			<!-- Node label -->
			<text x={node.x + 60} y={node.y + 16} class="node-title" text-anchor="middle">
				{node.label}
			</text>

			<!-- Node sublabel -->
			{#if node.sublabel}
				<text x={node.x + 60} y={node.y + 28} class="node-sub" text-anchor="middle">
					{node.sublabel}
				</text>
			{/if}

			<!-- Node tag -->
			{#if node.tag}
				<text x={node.x + 60} y={node.y + 36} class="node-tag" text-anchor="middle">
					{node.tag}
				</text>
			{/if}
		</g>
	{/each}
</svg>
