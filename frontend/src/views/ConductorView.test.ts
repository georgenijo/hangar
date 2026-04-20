import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import ConductorView from './ConductorView.svelte';

describe('ConductorView', () => {
	it('renders with id="view-conductor"', () => {
		const { container } = render(ConductorView);
		const viewElement = container.querySelector('#view-conductor');
		expect(viewElement).toBeTruthy();
	});

	it('renders AgentGraph component with static nodes', () => {
		const { container } = render(ConductorView);
		const graphSvg = container.querySelector('.graph-svg');
		expect(graphSvg).toBeTruthy();
	});

	it('contains static mock nodes', () => {
		const { container } = render(ConductorView);
		// Check for specific node labels in the static data
		const svgContent = container.querySelector('.graph-svg')?.innerHTML;
		expect(svgContent).toContain('Conductor');
		expect(svgContent).toContain('Planner');
		expect(svgContent).toContain('Architect');
		expect(svgContent).toContain('Builder');
		expect(svgContent).toContain('Tester');
	});

	it('contains static mock edges', () => {
		const { container } = render(ConductorView);
		// AgentGraph should render edges as path elements
		const edges = container.querySelectorAll('.edge');
		expect(edges.length).toBeGreaterThan(0);
	});

	it('renders escalations card', () => {
		const { container } = render(ConductorView);
		const escalationsCard = container.querySelector('.escalations-card');
		expect(escalationsCard).toBeTruthy();
	});
});
