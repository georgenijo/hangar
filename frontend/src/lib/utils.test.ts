import { describe, it, expect } from 'vitest';
import { formatCost, formatTokens, formatDuration, formatBytes, formatPct, clamp } from './utils';

describe('formatCost', () => {
	it('formats dollars with $ prefix and 2 decimals', () => {
		expect(formatCost(12.84)).toBe('$12.84');
		expect(formatCost(0.99)).toBe('$0.99');
		expect(formatCost(100)).toBe('$100.00');
		expect(formatCost(0.1)).toBe('$0.10');
	});

	it('handles zero', () => {
		expect(formatCost(0)).toBe('$0.00');
	});

	it('handles negative values', () => {
		expect(formatCost(-5.5)).toBe('$-5.50');
	});

	it('handles very large numbers', () => {
		expect(formatCost(9999999.99)).toBe('$9999999.99');
	});
});

describe('formatTokens', () => {
	it('formats with k suffix for thousands', () => {
		expect(formatTokens(72104)).toBe('72k');
		expect(formatTokens(1500)).toBe('2k'); // rounds
		expect(formatTokens(999)).toBe('999'); // below threshold
		expect(formatTokens(1000)).toBe('1k');
	});

	it('formats with M suffix for millions', () => {
		expect(formatTokens(1500000)).toBe('1.5M');
		expect(formatTokens(2000000)).toBe('2M');
		expect(formatTokens(1234567)).toBe('1.2M');
	});

	it('handles zero', () => {
		expect(formatTokens(0)).toBe('0');
	});

	it('handles small numbers', () => {
		expect(formatTokens(1)).toBe('1');
		expect(formatTokens(42)).toBe('42');
		expect(formatTokens(500)).toBe('500');
	});

	it('handles very large numbers', () => {
		expect(formatTokens(1000000000)).toBe('1000M');
	});
});

describe('formatDuration', () => {
	it('formats seconds', () => {
		expect(formatDuration(45)).toBe('45s');
		expect(formatDuration(0)).toBe('0s');
		expect(formatDuration(59)).toBe('59s');
	});

	it('formats minutes and seconds', () => {
		expect(formatDuration(65)).toBe('1m 5s');
		expect(formatDuration(120)).toBe('2m');
		expect(formatDuration(90)).toBe('1m 30s');
	});

	it('formats hours and minutes', () => {
		expect(formatDuration(3661)).toBe('1h 1m');
		expect(formatDuration(7200)).toBe('2h');
		expect(formatDuration(3600)).toBe('1h');
		expect(formatDuration(7260)).toBe('2h 1m');
	});

	it('handles very large durations', () => {
		expect(formatDuration(86400)).toBe('24h'); // 1 day
		expect(formatDuration(90000)).toBe('25h');
	});
});

describe('formatBytes', () => {
	it('formats bytes correctly', () => {
		expect(formatBytes(500)).toBe('500 B');
		expect(formatBytes(1024)).toBe('1.0 KB');
		expect(formatBytes(1048576)).toBe('1.0 MB');
		expect(formatBytes(1073741824)).toBe('1.0 GB');
	});

	it('handles zero', () => {
		expect(formatBytes(0)).toBe('0 B');
	});

	it('handles fractional values', () => {
		expect(formatBytes(1536)).toBe('1.5 KB');
		expect(formatBytes(2621440)).toBe('2.5 MB');
		expect(formatBytes(5368709120)).toBe('5.0 GB');
	});

	it('handles very large numbers', () => {
		expect(formatBytes(10995116277760)).toBe('10240.0 GB');
	});
});

describe('formatPct', () => {
	it('formats ratio as percentage', () => {
		expect(formatPct(0.28)).toBe('28%');
		expect(formatPct(0.5)).toBe('50%');
		expect(formatPct(1)).toBe('100%');
		expect(formatPct(0)).toBe('0%');
	});

	it('rounds to nearest integer', () => {
		expect(formatPct(0.125)).toBe('13%');
		expect(formatPct(0.126)).toBe('13%');
		expect(formatPct(0.999)).toBe('100%');
	});

	it('handles values over 1', () => {
		expect(formatPct(1.5)).toBe('150%');
	});

	it('handles negative values', () => {
		expect(formatPct(-0.25)).toBe('-25%');
	});
});

describe('clamp', () => {
	it('clamps value to range', () => {
		expect(clamp(5, 0, 10)).toBe(5);
		expect(clamp(-5, 0, 10)).toBe(0);
		expect(clamp(15, 0, 10)).toBe(10);
	});

	it('handles edge cases', () => {
		expect(clamp(0, 0, 10)).toBe(0);
		expect(clamp(10, 0, 10)).toBe(10);
	});
});
