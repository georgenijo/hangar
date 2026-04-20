import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import PipelineTrack from './PipelineTrack.svelte';
import type { PipelinePhase } from '$lib/types';

describe('PipelineTrack', () => {
	const phases: PipelinePhase[] = [
		{ name: 'Planner', id: 'planner', state: 'done' },
		{ name: 'Builder', id: 'builder', state: 'live' },
		{ name: 'Tester', id: 'tester', state: 'pending' }
	];

	it('renders phase-node with correct state classes', () => {
		const { container } = render(PipelineTrack, { props: { phases } });
		const nodes = container.querySelectorAll('.phase-node');

		expect(nodes[0].classList.contains('done')).toBe(true);
		expect(nodes[1].classList.contains('live')).toBe(true);
		expect(nodes[2].classList.contains('pending')).toBe(true);
	});

	it('renders correct number of nodes', () => {
		const { container } = render(PipelineTrack, { props: { phases } });
		const nodes = container.querySelectorAll('.phase-node');
		expect(nodes.length).toBe(3);
	});

	it('renders correct number of connecting lines', () => {
		const { container } = render(PipelineTrack, { props: { phases } });
		const lines = container.querySelectorAll('.phase-line');
		expect(lines.length).toBe(2); // N-1 lines for N nodes
	});

	it('renders phase names correctly', () => {
		const { container } = render(PipelineTrack, { props: { phases } });
		const names = container.querySelectorAll('.phase-name');

		expect(names[0].textContent).toBe('Planner');
		expect(names[1].textContent).toBe('Builder');
		expect(names[2].textContent).toBe('Tester');
	});

	it('renders phase dots', () => {
		const { container } = render(PipelineTrack, { props: { phases } });
		const dots = container.querySelectorAll('.phase-dot');
		expect(dots.length).toBe(3);
	});

	it('applies state classes to connecting lines', () => {
		const { container } = render(PipelineTrack, { props: { phases } });
		const lines = container.querySelectorAll('.phase-line');

		// First line inherits state from first phase
		expect(lines[0].classList.contains('done')).toBe(true);
		// Second line inherits state from second phase
		expect(lines[1].classList.contains('live')).toBe(true);
	});

	it('renders with single phase', () => {
		const singlePhase: PipelinePhase[] = [{ name: 'Planner', id: 'planner', state: 'done' }];
		const { container } = render(PipelineTrack, { props: { phases: singlePhase } });

		const nodes = container.querySelectorAll('.phase-node');
		const lines = container.querySelectorAll('.phase-line');

		expect(nodes.length).toBe(1);
		expect(lines.length).toBe(0); // No lines for single node
	});

	it('renders with empty phases array', () => {
		const { container } = render(PipelineTrack, { props: { phases: [] } });

		const nodes = container.querySelectorAll('.phase-node');
		expect(nodes.length).toBe(0);
	});

	it('renders phase info when provided', () => {
		const phasesWithInfo: PipelinePhase[] = [
			{ name: 'Planner', id: 'planner', state: 'done', info: '2m 30s' },
			{ name: 'Builder', id: 'builder', state: 'live' }
		];

		const { container } = render(PipelineTrack, { props: { phases: phasesWithInfo } });
		const infoElements = container.querySelectorAll('.phase-info');

		expect(infoElements.length).toBe(1);
		expect(infoElements[0].textContent).toBe('2m 30s');
	});

	it('has phase-track class on root element', () => {
		const { container } = render(PipelineTrack, { props: { phases } });
		const track = container.querySelector('.phase-track');
		expect(track).toBeTruthy();
	});
});
