import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import KpiCard from './KpiCard.svelte';
import type { KpiCardData } from '$lib/types';

describe('KpiCard', () => {
	it('renders label and value', () => {
		const data: KpiCardData = {
			label: 'Active Sessions',
			value: '42'
		};
		const { container } = render(KpiCard, { props: { data } });

		expect(container.textContent).toContain('Active Sessions');
		expect(container.textContent).toContain('42');
	});

	it('renders without sparkline when not provided', () => {
		const data: KpiCardData = {
			label: 'Total Cost',
			value: '$123.45'
		};
		const { container } = render(KpiCard, { props: { data } });

		const sparkline = container.querySelector('.sparkline');
		expect(sparkline).toBeNull();
	});

	it('renders sparkline when data is provided', () => {
		const data: KpiCardData = {
			label: '7d Spend',
			value: '$89.12',
			sparkline: [10, 20, 15, 30, 25]
		};
		const { container } = render(KpiCard, { props: { data } });

		const sparkline = container.querySelector('.sparkline');
		expect(sparkline).toBeTruthy();
	});

	it('does not render sparkline when data array is empty', () => {
		const data: KpiCardData = {
			label: 'Empty Spark',
			value: '$0.00',
			sparkline: []
		};
		const { container } = render(KpiCard, { props: { data } });

		const sparkline = container.querySelector('.sparkline');
		expect(sparkline).toBeNull();
	});

	it('applies alert class when data.alert is true', () => {
		const data: KpiCardData = {
			label: 'CPU Usage',
			value: '98%',
			alert: true
		};
		const { container } = render(KpiCard, { props: { data } });

		const kpiDiv = container.querySelector('.kpi');
		expect(kpiDiv?.classList.contains('kpi-alert')).toBe(true);
	});

	it('does not apply alert class when data.alert is false', () => {
		const data: KpiCardData = {
			label: 'CPU Usage',
			value: '45%',
			alert: false
		};
		const { container } = render(KpiCard, { props: { data } });

		const kpiDiv = container.querySelector('.kpi');
		expect(kpiDiv?.classList.contains('kpi-alert')).toBe(false);
	});

	it('does not apply alert class when data.alert is undefined', () => {
		const data: KpiCardData = {
			label: 'CPU Usage',
			value: '45%'
		};
		const { container } = render(KpiCard, { props: { data } });

		const kpiDiv = container.querySelector('.kpi');
		expect(kpiDiv?.classList.contains('kpi-alert')).toBe(false);
	});

	it('renders trend with up arrow when direction is up', () => {
		const data: KpiCardData = {
			label: 'Revenue',
			value: '$1000',
			trend: { direction: 'up', text: '+12%' }
		};
		const { container } = render(KpiCard, { props: { data } });

		expect(container.textContent).toContain('↑');
		expect(container.textContent).toContain('+12%');
		const trend = container.querySelector('.trend-up');
		expect(trend).toBeTruthy();
	});

	it('renders trend with down arrow when direction is down', () => {
		const data: KpiCardData = {
			label: 'Errors',
			value: '5',
			trend: { direction: 'down', text: '-8%' }
		};
		const { container } = render(KpiCard, { props: { data } });

		expect(container.textContent).toContain('↓');
		expect(container.textContent).toContain('-8%');
		const trend = container.querySelector('.trend-down');
		expect(trend).toBeTruthy();
	});

	it('renders trend with right arrow when direction is neutral', () => {
		const data: KpiCardData = {
			label: 'Stable',
			value: '100',
			trend: { direction: 'neutral', text: '0%' }
		};
		const { container } = render(KpiCard, { props: { data } });

		expect(container.textContent).toContain('→');
		expect(container.textContent).toContain('0%');
		const trend = container.querySelector('.trend-neutral');
		expect(trend).toBeTruthy();
	});

	it('does not render trend when not provided', () => {
		const data: KpiCardData = {
			label: 'No Trend',
			value: '50'
		};
		const { container } = render(KpiCard, { props: { data } });

		const trend = container.querySelector('.trend');
		expect(trend).toBeNull();
	});

	it('renders with all properties: label, value, trend, sparkline, alert', () => {
		const data: KpiCardData = {
			label: 'Full KPI',
			value: '$250.00',
			trend: { direction: 'up', text: '+5%' },
			sparkline: [10, 15, 12, 20, 18],
			alert: true
		};
		const { container } = render(KpiCard, { props: { data } });

		expect(container.textContent).toContain('Full KPI');
		expect(container.textContent).toContain('$250.00');
		expect(container.textContent).toContain('↑');
		expect(container.textContent).toContain('+5%');

		const kpiDiv = container.querySelector('.kpi');
		expect(kpiDiv?.classList.contains('kpi-alert')).toBe(true);

		const sparkline = container.querySelector('.sparkline');
		expect(sparkline).toBeTruthy();
	});

	it('has correct CSS classes structure', () => {
		const data: KpiCardData = {
			label: 'Test',
			value: '123'
		};
		const { container } = render(KpiCard, { props: { data } });

		expect(container.querySelector('.kpi')).toBeTruthy();
		expect(container.querySelector('.kpi-header')).toBeTruthy();
		expect(container.querySelector('.kpi-label')).toBeTruthy();
		expect(container.querySelector('.kpi-value')).toBeTruthy();
		expect(container.querySelector('.num')).toBeTruthy();
	});
});
