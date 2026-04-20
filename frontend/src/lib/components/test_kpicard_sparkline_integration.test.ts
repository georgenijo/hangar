/**
 * Integration Test: KpiCard + Sparkline Component Integration
 * Priority: HIGH - Tests cross-component interaction from issue-20
 *
 * KpiCard imports and conditionally renders Sparkline.
 * This test verifies the integration works correctly with:
 * - Optional sparkline data
 * - Empty sparkline arrays
 * - Valid sparkline data rendering
 */
import { render } from '@testing-library/svelte';
import { describe, it, expect } from 'vitest';
import KpiCard from './KpiCard.svelte';
import type { KpiCardData } from '$lib/types';

describe('KpiCard + Sparkline Integration', () => {
	it('renders KpiCard with sparkline data and Sparkline component appears', () => {
		const data: KpiCardData = {
			label: 'CPU Usage',
			value: '45%',
			sparkline: [20, 30, 25, 40, 45, 42, 48]
		};

		const { container } = render(KpiCard, { props: { data } });

		// KpiCard should render
		expect(container.textContent).toContain('CPU Usage');
		expect(container.textContent).toContain('45%');

		// Sparkline SVG should be present
		const svg = container.querySelector('svg.sparkline');
		expect(svg).not.toBeNull();
		expect(svg?.querySelector('polyline.sparkline-line')).not.toBeNull();
	});

	it('renders KpiCard without sparkline when data is undefined', () => {
		const data: KpiCardData = {
			label: 'Active Sessions',
			value: '12'
		};

		const { container } = render(KpiCard, { props: { data } });

		expect(container.textContent).toContain('Active Sessions');
		expect(container.textContent).toContain('12');

		// No Sparkline should be rendered
		const svg = container.querySelector('svg.sparkline');
		expect(svg).toBeNull();
	});

	it('does not render sparkline when array is empty', () => {
		const data: KpiCardData = {
			label: 'Idle Hosts',
			value: '0',
			sparkline: []
		};

		const { container } = render(KpiCard, { props: { data } });

		expect(container.textContent).toContain('Idle Hosts');

		// Empty sparkline array should not render SVG (per line 24 condition)
		const svg = container.querySelector('svg.sparkline');
		expect(svg).toBeNull();
	});

	it('renders KpiCard with trend and sparkline together', () => {
		const data: KpiCardData = {
			label: 'API Latency',
			value: '125ms',
			trend: { direction: 'down', text: '-15%' },
			sparkline: [150, 145, 140, 130, 125]
		};

		const { container } = render(KpiCard, { props: { data } });

		expect(container.textContent).toContain('API Latency');
		expect(container.textContent).toContain('125ms');
		expect(container.textContent).toContain('↓ -15%');

		// Both trend and sparkline should be present
		expect(container.querySelector('.trend-down')).not.toBeNull();
		expect(container.querySelector('svg.sparkline')).not.toBeNull();
	});

	it('renders KpiCard with alert state and sparkline', () => {
		const data: KpiCardData = {
			label: 'Error Rate',
			value: '8.5%',
			alert: true,
			sparkline: [2, 3, 4, 6, 8.5]
		};

		const { container } = render(KpiCard, { props: { data } });

		const kpiDiv = container.querySelector('.kpi');
		expect(kpiDiv?.classList.contains('kpi-alert')).toBe(true);
		expect(container.textContent).toContain('Error Rate');
		expect(container.querySelector('svg.sparkline')).not.toBeNull();
	});

	it('handles single data point sparkline', () => {
		const data: KpiCardData = {
			label: 'Requests',
			value: '1',
			sparkline: [42]
		};

		const { container } = render(KpiCard, { props: { data } });

		// Single point should still render sparkline
		const svg = container.querySelector('svg.sparkline');
		expect(svg).not.toBeNull();

		const polyline = svg?.querySelector('polyline');
		expect(polyline).not.toBeNull();
		// Single point will have one coordinate pair
		expect(polyline?.getAttribute('points')).toBeTruthy();
	});

	it('renders multiple KpiCards with different sparkline configurations', () => {
		const cards: KpiCardData[] = [
			{ label: 'Card 1', value: '10', sparkline: [5, 10, 15] },
			{ label: 'Card 2', value: '20' }, // no sparkline
			{ label: 'Card 3', value: '30', sparkline: [] }, // empty
			{ label: 'Card 4', value: '40', sparkline: [40, 38, 40] }
		];

		const { container } = render(KpiCard, { props: { data: cards[0] } });
		const { container: c2 } = render(KpiCard, { props: { data: cards[1] } });
		const { container: c3 } = render(KpiCard, { props: { data: cards[2] } });
		const { container: c4 } = render(KpiCard, { props: { data: cards[3] } });

		// Card 1 and 4 should have sparklines
		expect(container.querySelector('svg.sparkline')).not.toBeNull();
		expect(c4.querySelector('svg.sparkline')).not.toBeNull();

		// Card 2 and 3 should not have sparklines
		expect(c2.querySelector('svg.sparkline')).toBeNull();
		expect(c3.querySelector('svg.sparkline')).toBeNull();
	});
});
