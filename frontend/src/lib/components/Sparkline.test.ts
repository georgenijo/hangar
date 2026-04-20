import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import Sparkline from './Sparkline.svelte';

describe('Sparkline', () => {
	it('renders SVG polyline with normalized points', () => {
		const { container } = render(Sparkline, { props: { data: [10, 20, 15] } });
		const polyline = container.querySelector('.sparkline-line');
		expect(polyline).toBeTruthy();
		expect(polyline?.getAttribute('points')).not.toBe('');
	});

	it('generates polyline points for simple data array', () => {
		const { container } = render(Sparkline, { props: { data: [10, 20, 30] } });
		const polyline = container.querySelector('.sparkline-line');
		const points = polyline?.getAttribute('points') || '';

		// Should have 3 points
		const pointsArray = points.trim().split(' ');
		expect(pointsArray.length).toBe(3);

		// Each point should have x,y coordinates
		pointsArray.forEach((point) => {
			expect(point).toMatch(/^\d+\.\d+,\d+\.\d+$/);
		});
	});

	it('handles empty data array', () => {
		const { container } = render(Sparkline, { props: { data: [] } });
		const polyline = container.querySelector('.sparkline-line');
		expect(polyline?.getAttribute('points')).toBe('');
	});

	it('handles single data point', () => {
		const { container } = render(Sparkline, { props: { data: [42] } });
		const polyline = container.querySelector('.sparkline-line');
		const points = polyline?.getAttribute('points') || '';
		expect(points).not.toBe('');

		// Should have 1 point
		const pointsArray = points.trim().split(' ');
		expect(pointsArray.length).toBe(1);
	});

	it('normalizes data to fit viewBox with padding', () => {
		const width = 100;
		const height = 24;
		const data = [0, 100]; // Min and max values

		const { container } = render(Sparkline, { props: { data, width, height } });
		const polyline = container.querySelector('.sparkline-line');
		const points = polyline?.getAttribute('points') || '';
		const pointsArray = points.trim().split(' ');

		expect(pointsArray.length).toBe(2);

		// First point should be at x=0
		const [x0] = pointsArray[0].split(',');
		expect(parseFloat(x0)).toBe(0);

		// Last point should be at x=width
		const [x1] = pointsArray[1].split(',');
		expect(parseFloat(x1)).toBe(width);

		// Y values should be within [2, height-2] range (2px padding)
		pointsArray.forEach((point) => {
			const [, y] = point.split(',');
			const yVal = parseFloat(y);
			expect(yVal).toBeGreaterThanOrEqual(2);
			expect(yVal).toBeLessThanOrEqual(height - 2);
		});
	});

	it('applies correct viewBox dimensions', () => {
		const width = 100;
		const height = 24;
		const { container } = render(Sparkline, { props: { data: [10, 20], width, height } });
		const svg = container.querySelector('.sparkline');
		expect(svg?.getAttribute('viewBox')).toBe(`0 0 ${width} ${height}`);
	});

	it('uses default width and height when not provided', () => {
		const { container } = render(Sparkline, { props: { data: [10, 20] } });
		const svg = container.querySelector('.sparkline');
		expect(svg?.getAttribute('viewBox')).toBe('0 0 100 24');
	});

	it('applies custom color to polyline', () => {
		const customColor = 'red';
		const { container } = render(Sparkline, {
			props: { data: [10, 20], color: customColor }
		});
		const polyline = container.querySelector('.sparkline-line');
		expect(polyline?.getAttribute('style')).toContain(`stroke: ${customColor}`);
	});

	it('uses default accent color when color not provided', () => {
		const { container } = render(Sparkline, { props: { data: [10, 20] } });
		const polyline = container.querySelector('.sparkline-line');
		expect(polyline?.getAttribute('style')).toContain('stroke: var(--accent)');
	});

	it('handles all equal values in data array', () => {
		const { container } = render(Sparkline, { props: { data: [50, 50, 50] } });
		const polyline = container.querySelector('.sparkline-line');
		const points = polyline?.getAttribute('points') || '';

		// Should still generate points
		expect(points).not.toBe('');
		const pointsArray = points.trim().split(' ');
		expect(pointsArray.length).toBe(3);
	});

	it('normalizes negative values correctly', () => {
		const { container } = render(Sparkline, { props: { data: [-10, 0, 10] } });
		const polyline = container.querySelector('.sparkline-line');
		const points = polyline?.getAttribute('points');

		expect(points).toBeTruthy();
		expect(points).not.toBe('');

		// Should have 3 points
		const pointsArray = points!.trim().split(' ');
		expect(pointsArray.length).toBe(3);
	});
});
