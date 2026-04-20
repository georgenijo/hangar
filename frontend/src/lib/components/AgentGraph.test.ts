import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import AgentGraph from './AgentGraph.svelte';
import type { AgentNode, AgentEdge } from '$lib/types';

describe('AgentGraph', () => {
	const sampleNodes: AgentNode[] = [
		{ id: 'planner', label: 'Planner', x: 50, y: 50, state: 'done' },
		{ id: 'builder', label: 'Builder', x: 200, y: 50, state: 'live' },
		{ id: 'tester', label: 'Tester', x: 350, y: 50, state: 'idle' }
	];

	const sampleEdges: AgentEdge[] = [
		{ from: 'planner', to: 'builder', state: 'done' },
		{ from: 'builder', to: 'tester', state: 'active' }
	];

	it('renders SVG with correct viewBox', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const svg = container.querySelector('svg');

		expect(svg).toBeTruthy();
		expect(svg?.getAttribute('viewBox')).toBe('0 0 600 300');
	});

	it('renders correct number of nodes', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const nodes = container.querySelectorAll('.node');

		expect(nodes.length).toBe(3);
	});

	it('renders correct number of edges', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const edges = container.querySelectorAll('.edge');

		expect(edges.length).toBe(2);
	});

	it('applies correct state classes to nodes', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const nodes = container.querySelectorAll('.node');

		expect(nodes[0].classList.contains('node-done')).toBe(true);
		expect(nodes[1].classList.contains('node-live')).toBe(true);
		expect(nodes[2].classList.contains('node-idle')).toBe(true);
	});

	it('applies correct state classes to edges', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const edges = container.querySelectorAll('.edge');

		expect(edges[0].classList.contains('edge-done')).toBe(true);
		expect(edges[1].classList.contains('edge-live')).toBe(true);
	});

	it('renders node labels', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const labels = container.querySelectorAll('.node-title');

		expect(labels.length).toBe(3);
		expect(labels[0].textContent).toBe('Planner');
		expect(labels[1].textContent).toBe('Builder');
		expect(labels[2].textContent).toBe('Tester');
	});

	it('renders node sublabels when provided', () => {
		const nodesWithSublabel: AgentNode[] = [
			{ id: 'planner', label: 'Planner', sublabel: '2m 30s', x: 50, y: 50, state: 'done' }
		];

		const { container } = render(AgentGraph, {
			props: { nodes: nodesWithSublabel, edges: [] }
		});
		const sublabels = container.querySelectorAll('.node-sub');

		expect(sublabels.length).toBe(1);
		expect(sublabels[0].textContent).toBe('2m 30s');
	});

	it('renders node tags when provided', () => {
		const nodesWithTag: AgentNode[] = [
			{ id: 'planner', label: 'Planner', tag: 'v1.0', x: 50, y: 50, state: 'done' }
		];

		const { container } = render(AgentGraph, { props: { nodes: nodesWithTag, edges: [] } });
		const tags = container.querySelectorAll('.node-tag');

		expect(tags.length).toBe(1);
		expect(tags[0].textContent).toBe('v1.0');
	});

	it('positions nodes at correct coordinates', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const rects = container.querySelectorAll('.node rect');

		expect(rects[0].getAttribute('x')).toBe('50');
		expect(rects[0].getAttribute('y')).toBe('50');
		expect(rects[1].getAttribute('x')).toBe('200');
		expect(rects[1].getAttribute('y')).toBe('50');
	});

	it('renders edges as SVG paths', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const paths = container.querySelectorAll('path.edge');

		expect(paths.length).toBe(2);

		// Each path should have a valid 'd' attribute with path commands
		paths.forEach((path) => {
			const d = path.getAttribute('d');
			expect(d).toBeTruthy();
			expect(d).toMatch(/^M\s+[\d.]+\s+[\d.]+/); // Starts with M (move) command
		});
	});

	it('handles empty nodes array', () => {
		const { container } = render(AgentGraph, { props: { nodes: [], edges: [] } });
		const svg = container.querySelector('svg');
		const nodes = container.querySelectorAll('.node');

		expect(svg).toBeTruthy();
		expect(nodes.length).toBe(0);
	});

	it('handles empty edges array', () => {
		const { container } = render(AgentGraph, { props: { nodes: sampleNodes, edges: [] } });
		const nodes = container.querySelectorAll('.node');
		const edges = container.querySelectorAll('.edge');

		expect(nodes.length).toBe(3);
		expect(edges.length).toBe(0);
	});

	it('handles edges with missing nodes gracefully', () => {
		const invalidEdges: AgentEdge[] = [
			{ from: 'nonexistent', to: 'builder', state: 'done' },
			{ from: 'planner', to: 'nonexistent', state: 'active' }
		];

		const { container } = render(AgentGraph, { props: { nodes: sampleNodes, edges: invalidEdges } });
		const edges = container.querySelectorAll('.edge');

		// Invalid edges should not be rendered
		expect(edges.length).toBe(0);
	});

	it('renders node rectangles with correct dimensions', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const rects = container.querySelectorAll('.node rect');

		rects.forEach((rect) => {
			expect(rect.getAttribute('width')).toBe('120');
			expect(rect.getAttribute('height')).toBe('40');
			expect(rect.getAttribute('rx')).toBe('6'); // Rounded corners
		});
	});

	it('respects custom width and height props', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges, width: 800, height: 400 }
		});
		const svg = container.querySelector('svg');

		expect(svg?.getAttribute('viewBox')).toBe('0 0 800 400');
	});

	it('renders edges before nodes (correct z-order)', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const svg = container.querySelector('svg');
		const children = Array.from(svg?.children || []);

		// Find first edge and first node
		const firstEdgeIndex = children.findIndex((el) => el.classList.contains('edge'));
		const firstNodeIndex = children.findIndex((el) => el.classList.contains('node'));

		// Edges should come before nodes in DOM order
		expect(firstEdgeIndex).toBeLessThan(firstNodeIndex);
	});

	it('generates curved paths for edges', () => {
		const { container } = render(AgentGraph, {
			props: { nodes: sampleNodes, edges: sampleEdges }
		});
		const paths = container.querySelectorAll('path.edge');

		// Paths should use quadratic bezier curves (Q command)
		paths.forEach((path) => {
			const d = path.getAttribute('d');
			expect(d).toMatch(/Q/); // Contains Q (quadratic bezier) command
		});
	});
});
