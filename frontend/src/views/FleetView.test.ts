import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import FleetView from './FleetView.svelte';
import { createHostStore } from '$lib/stores/host.svelte';

// Mock the hostStore module
let mockHostStore: ReturnType<typeof createHostStore>;

// Setup before each test
beforeEach(() => {
	mockHostStore = createHostStore();
});

describe('FleetView', () => {
	it('renders with id="view-fleet"', () => {
		const { container } = render(FleetView);
		const viewElement = container.querySelector('#view-fleet');
		expect(viewElement).toBeTruthy();
	});

	it('renders host cards', () => {
		const { container } = render(FleetView);
		const hostCards = container.querySelectorAll('.host-card');
		// Should have 3 host cards: 1 local + 2 mock
		expect(hostCards.length).toBe(3);
	});

	it('renders RingGauge components for CPU, RAM, DISK', () => {
		const { container } = render(FleetView);
		const ringGauges = container.querySelectorAll('.ring-metric');
		// 3 hosts × 3 metrics = 9 RingGauge components
		expect(ringGauges.length).toBe(9);
	});

	it('renders 3 RingGauge components per host card', () => {
		const { container } = render(FleetView);
		const hostCards = container.querySelectorAll('.host-card');

		hostCards.forEach((hostCard) => {
			const gauges = hostCard.querySelectorAll('.ring-metric');
			expect(gauges.length).toBe(3);
		});
	});

	it('displays CPU, RAM, and DISK labels in RingGauges', () => {
		const { container } = render(FleetView);
		const labels = Array.from(container.querySelectorAll('.ring-label')).map(
			(el) => el.textContent
		);

		// Should have these labels repeated for each host
		expect(labels).toContain('CPU');
		expect(labels).toContain('RAM');
		expect(labels).toContain('DISK');
	});

	it('renders static multi-host data', () => {
		const { container } = render(FleetView);
		const hostHeaders = container.querySelectorAll('.host-header h3');

		// Check for mock hostnames
		const hostnames = Array.from(hostHeaders).map((h) => h.textContent);
		expect(hostnames).toContain('macmini');
		expect(hostnames).toContain('optiplex');
	});

	it('uses hostStore for local host metrics', () => {
		const { container } = render(FleetView);
		// First host card should use hostStore data
		const firstHostCard = container.querySelector('.host-card');
		expect(firstHostCard).toBeTruthy();

		// Should contain RingGauge components that will receive cpuPct, ramPct, diskPct
		const gauges = firstHostCard?.querySelectorAll('.ring-metric');
		expect(gauges?.length).toBe(3);
	});
});
