import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import BarChart from './BarChart.svelte';

describe('BarChart', () => {
	const sampleData = [
		{ label: 'Jan', value: 100 },
		{ label: 'Feb', value: 200 },
		{ label: 'Mar', value: 150 }
	];

	it('renders SVG with correct viewBox', () => {
		const { container } = render(BarChart, { props: { data: sampleData } });
		const svg = container.querySelector('svg');

		expect(svg).toBeTruthy();
		expect(svg?.getAttribute('viewBox')).toBe('0 0 600 200');
	});

	it('renders correct number of bars', () => {
		const { container } = render(BarChart, { props: { data: sampleData } });
		const bars = container.querySelectorAll('.bar');

		expect(bars.length).toBe(3);
	});

	it('normalizes bar heights to max value', () => {
		const { container } = render(BarChart, { props: { data: sampleData, height: 200 } });
		const bars = container.querySelectorAll('rect.bar');

		// Max value is 200, so Feb bar should have max height (height - 40)
		const febBar = bars[1] as SVGRectElement;
		const janBar = bars[0] as SVGRectElement;

		const febHeight = parseFloat(febBar.getAttribute('height') || '0');
		const janHeight = parseFloat(janBar.getAttribute('height') || '0');

		// Feb (200) should have height of 160 (200-40)
		expect(febHeight).toBeCloseTo(160, 1);
		// Jan (100) should have height of 80 (half of Feb)
		expect(janHeight).toBeCloseTo(80, 1);
	});

	it('renders labels for each bar', () => {
		const { container } = render(BarChart, { props: { data: sampleData } });
		const labels = container.querySelectorAll('.bar-label');

		expect(labels.length).toBe(3);
		expect(labels[0].textContent).toBe('Jan');
		expect(labels[1].textContent).toBe('Feb');
		expect(labels[2].textContent).toBe('Mar');
	});

	it('handles horizontal orientation', () => {
		const { container } = render(BarChart, { props: { data: sampleData, horizontal: true } });
		const svg = container.querySelector('svg');

		expect(svg?.classList.contains('horizontal')).toBe(true);
	});

	it('applies bar-accent class to last two bars', () => {
		const { container } = render(BarChart, { props: { data: sampleData } });
		const bars = container.querySelectorAll('rect.bar');

		expect(bars[0].classList.contains('bar-accent')).toBe(false);
		expect(bars[1].classList.contains('bar-accent')).toBe(true);
		expect(bars[2].classList.contains('bar-accent')).toBe(true);
	});

	it('handles empty data array without error', () => {
		const { container } = render(BarChart, { props: { data: [] } });
		const svg = container.querySelector('svg');
		const bars = container.querySelectorAll('.bar');

		expect(svg).toBeTruthy();
		expect(bars.length).toBe(0);
	});

	it('handles negative values without producing invalid SVG', () => {
		const negativeData = [
			{ label: 'A', value: -50 },
			{ label: 'B', value: -100 },
			{ label: 'C', value: -25 }
		];

		const { container } = render(BarChart, { props: { data: negativeData } });
		const bars = container.querySelectorAll('rect.bar');

		// All bars should have non-negative dimensions
		bars.forEach((bar) => {
			const width = parseFloat(bar.getAttribute('width') || '0');
			const height = parseFloat(bar.getAttribute('height') || '0');

			expect(width).toBeGreaterThanOrEqual(0);
			expect(height).toBeGreaterThanOrEqual(0);
			expect(isNaN(width)).toBe(false);
			expect(isNaN(height)).toBe(false);
		});
	});

	it('handles mixed positive and negative values', () => {
		const mixedData = [
			{ label: 'A', value: 100 },
			{ label: 'B', value: -50 },
			{ label: 'C', value: 200 }
		];

		const { container } = render(BarChart, { props: { data: mixedData } });
		const bars = container.querySelectorAll('rect.bar');

		expect(bars.length).toBe(3);

		// All bars should have valid non-negative dimensions
		bars.forEach((bar) => {
			const width = parseFloat(bar.getAttribute('width') || '0');
			const height = parseFloat(bar.getAttribute('height') || '0');

			expect(width).toBeGreaterThanOrEqual(0);
			expect(height).toBeGreaterThanOrEqual(0);
			expect(isFinite(width)).toBe(true);
			expect(isFinite(height)).toBe(true);
		});
	});

	it('handles single data point', () => {
		const singleData = [{ label: 'Single', value: 42 }];

		const { container } = render(BarChart, { props: { data: singleData } });
		const bars = container.querySelectorAll('.bar');
		const labels = container.querySelectorAll('.bar-label');

		expect(bars.length).toBe(1);
		expect(labels.length).toBe(1);
		expect(labels[0].textContent).toBe('Single');
	});

	it('handles all zero values', () => {
		const zeroData = [
			{ label: 'A', value: 0 },
			{ label: 'B', value: 0 },
			{ label: 'C', value: 0 }
		];

		const { container } = render(BarChart, { props: { data: zeroData } });
		const bars = container.querySelectorAll('rect.bar');

		expect(bars.length).toBe(3);

		// All bars should have zero or near-zero height (but valid SVG)
		bars.forEach((bar) => {
			const height = parseFloat(bar.getAttribute('height') || '0');
			expect(height).toBeGreaterThanOrEqual(0);
			expect(isFinite(height)).toBe(true);
		});
	});

	it('respects custom width and height props', () => {
		const { container } = render(BarChart, {
			props: { data: sampleData, width: 800, height: 400 }
		});
		const svg = container.querySelector('svg');

		expect(svg?.getAttribute('viewBox')).toBe('0 0 800 400');
	});
});
