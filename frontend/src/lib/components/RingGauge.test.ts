import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import RingGauge from './RingGauge.svelte';

describe('RingGauge', () => {
	const radius = 26;
	const circumference = 2 * Math.PI * radius;

	it('renders SVG circle with class "ring-metric"', () => {
		const { container } = render(RingGauge, { props: { value: 50, label: 'CPU' } });
		const ringMetric = container.querySelector('.ring-metric');
		expect(ringMetric).toBeTruthy();
		expect(ringMetric?.querySelector('svg')).toBeTruthy();
	});

	it('calculates correct stroke-dashoffset for 28%', () => {
		const { container } = render(RingGauge, { props: { value: 28, label: 'CPU' } });
		const fg = container.querySelector('.ring-fg');
		const expected = circumference * (1 - 0.28);
		expect(fg?.getAttribute('stroke-dashoffset')).toBe(expected.toString());
	});

	it('calculates correct stroke-dashoffset for 41%', () => {
		const { container } = render(RingGauge, { props: { value: 41, label: 'RAM' } });
		const fg = container.querySelector('.ring-fg');
		const expected = circumference * (1 - 0.41);
		expect(fg?.getAttribute('stroke-dashoffset')).toBe(expected.toString());
	});

	it('calculates correct stroke-dashoffset for 78%', () => {
		const { container } = render(RingGauge, { props: { value: 78, label: 'DISK' } });
		const fg = container.querySelector('.ring-fg');
		const expected = circumference * (1 - 0.78);
		expect(fg?.getAttribute('stroke-dashoffset')).toBe(expected.toString());
	});

	it('applies warn class when value >= threshold', () => {
		const { container } = render(RingGauge, {
			props: { value: 80, label: 'CPU', warn_threshold: 75 }
		});
		const fg = container.querySelector('.ring-fg');
		expect(fg?.classList.contains('ring-warn')).toBe(true);
	});

	it('does not apply warn class when value < threshold', () => {
		const { container } = render(RingGauge, {
			props: { value: 50, label: 'CPU', warn_threshold: 75 }
		});
		const fg = container.querySelector('.ring-fg');
		expect(fg?.classList.contains('ring-warn')).toBe(false);
	});

	it('handles 0% edge case', () => {
		const { container } = render(RingGauge, { props: { value: 0, label: 'CPU' } });
		const fg = container.querySelector('.ring-fg');
		const expected = circumference * (1 - 0);
		expect(fg?.getAttribute('stroke-dashoffset')).toBe(expected.toString());
	});

	it('handles 100% edge case', () => {
		const { container } = render(RingGauge, { props: { value: 100, label: 'CPU' } });
		const fg = container.querySelector('.ring-fg');
		const expected = circumference * (1 - 1);
		expect(fg?.getAttribute('stroke-dashoffset')).toBe(expected.toString());
	});

	it('handles >100% edge case', () => {
		const { container } = render(RingGauge, { props: { value: 120, label: 'CPU' } });
		const fg = container.querySelector('.ring-fg');
		const expected = circumference * (1 - 1.2);
		expect(fg?.getAttribute('stroke-dashoffset')).toBe(expected.toString());
	});

	it('sets correct stroke-dasharray equal to circumference', () => {
		const { container } = render(RingGauge, { props: { value: 50, label: 'CPU' } });
		const fg = container.querySelector('.ring-fg');
		expect(fg?.getAttribute('stroke-dasharray')).toBe(circumference.toString());
	});

	it('verifies circumference formula 2πr where r=26', () => {
		const expectedCircumference = 2 * Math.PI * 26;
		expect(circumference).toBeCloseTo(163.36, 2);
		expect(circumference).toBe(expectedCircumference);
	});
});
