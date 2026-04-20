import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { createHostStore } from './host.svelte';
import * as api from '../api';
import type { HostMetrics } from '../types';

// Mock the API module
vi.mock('../api');

describe('hostStore', () => {
	let hostStore: ReturnType<typeof createHostStore>;
	let mockMetrics: HostMetrics;

	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers();

		// Create fresh store instance for each test
		hostStore = createHostStore();

		mockMetrics = {
			hostname: 'test-host',
			cpu_pct: 28.5,
			ram_used_bytes: 8589934592, // 8 GB
			ram_total_bytes: 17179869184, // 16 GB
			disk_used_bytes: 107374182400, // 100 GB
			disk_total_bytes: 536870912000 // 500 GB
		};
	});

	afterEach(() => {
		hostStore.stopPolling();
		vi.useRealTimers();
	});

	describe('initial state', () => {
		it('starts with null metrics', () => {
			expect(hostStore.metrics).toBeNull();
		});

		it('starts with loading false', () => {
			expect(hostStore.loading).toBe(false);
		});

		it('starts with no error', () => {
			expect(hostStore.error).toBeNull();
		});
	});

	describe('derived values', () => {
		it('cpuPct returns 0 when metrics is null', () => {
			expect(hostStore.cpuPct).toBe(0);
		});

		it('ramPct returns 0 when metrics is null', () => {
			expect(hostStore.ramPct).toBe(0);
		});

		it('diskPct returns 0 when metrics is null', () => {
			expect(hostStore.diskPct).toBe(0);
		});

		it('cpuPct returns cpu_pct from metrics', async () => {
			vi.mocked(api.getHostMetrics).mockResolvedValue(mockMetrics);
			await hostStore.refresh();
			expect(hostStore.cpuPct).toBe(28.5);
		});

		it('ramPct calculates percentage correctly', async () => {
			vi.mocked(api.getHostMetrics).mockResolvedValue(mockMetrics);
			await hostStore.refresh();
			// 8GB / 16GB = 50%
			expect(hostStore.ramPct).toBe(50);
		});

		it('diskPct calculates percentage correctly', async () => {
			vi.mocked(api.getHostMetrics).mockResolvedValue(mockMetrics);
			await hostStore.refresh();
			// 100GB / 500GB = 20%
			expect(hostStore.diskPct).toBe(20);
		});

		it('ramPct handles zero total bytes', async () => {
			const metricsWithZero = { ...mockMetrics, ram_total_bytes: 0 };
			vi.mocked(api.getHostMetrics).mockResolvedValue(metricsWithZero);
			await hostStore.refresh();
			expect(hostStore.ramPct).toBe(0);
		});

		it('diskPct handles zero total bytes', async () => {
			const metricsWithZero = { ...mockMetrics, disk_total_bytes: 0 };
			vi.mocked(api.getHostMetrics).mockResolvedValue(metricsWithZero);
			await hostStore.refresh();
			expect(hostStore.diskPct).toBe(0);
		});
	});

	describe('refresh', () => {
		it('fetches metrics successfully', async () => {
			vi.mocked(api.getHostMetrics).mockResolvedValue(mockMetrics);

			await hostStore.refresh();

			expect(api.getHostMetrics).toHaveBeenCalledOnce();
			expect(hostStore.metrics).toEqual(mockMetrics);
			expect(hostStore.error).toBeNull();
		});

		it('handles fetch error', async () => {
			const error = new Error('Network error');
			vi.mocked(api.getHostMetrics).mockRejectedValue(error);

			await hostStore.refresh();

			expect(hostStore.metrics).toBeNull();
			expect(hostStore.error).toBe('Network error');
		});

		it('handles non-Error exceptions', async () => {
			vi.mocked(api.getHostMetrics).mockRejectedValue('string error');

			await hostStore.refresh();

			expect(hostStore.error).toBe('Failed to fetch host metrics');
		});
	});

	describe('polling lifecycle', () => {
		it('startPolling initiates immediate fetch', async () => {
			vi.mocked(api.getHostMetrics).mockResolvedValue(mockMetrics);

			hostStore.startPolling();
			await vi.waitFor(() => expect(api.getHostMetrics).toHaveBeenCalled());

			expect(api.getHostMetrics).toHaveBeenCalledOnce();
		});

		it('startPolling sets up interval at 10s', async () => {
			vi.mocked(api.getHostMetrics).mockResolvedValue(mockMetrics);

			hostStore.startPolling();
			await vi.waitFor(() => expect(api.getHostMetrics).toHaveBeenCalled());

			// Fast-forward 10 seconds
			vi.advanceTimersByTime(10_000);
			await vi.waitFor(() => expect(api.getHostMetrics).toHaveBeenCalledTimes(2));

			// Fast-forward another 10 seconds
			vi.advanceTimersByTime(10_000);
			await vi.waitFor(() => expect(api.getHostMetrics).toHaveBeenCalledTimes(3));
		});

		it('startPolling is idempotent - does not create multiple intervals', async () => {
			vi.mocked(api.getHostMetrics).mockResolvedValue(mockMetrics);

			hostStore.startPolling();
			hostStore.startPolling();
			hostStore.startPolling();

			await vi.waitFor(() => expect(api.getHostMetrics).toHaveBeenCalled());

			vi.advanceTimersByTime(10_000);
			await vi.waitFor(() => expect(api.getHostMetrics).toHaveBeenCalledTimes(2));
		});

		it('stopPolling clears the interval', async () => {
			vi.mocked(api.getHostMetrics).mockResolvedValue(mockMetrics);

			hostStore.startPolling();
			await vi.waitFor(() => expect(api.getHostMetrics).toHaveBeenCalled());

			hostStore.stopPolling();

			const callCount = vi.mocked(api.getHostMetrics).mock.calls.length;

			// Advance time - should not trigger more calls
			vi.advanceTimersByTime(30_000);
			await vi.waitFor(() => Promise.resolve());

			expect(api.getHostMetrics).toHaveBeenCalledTimes(callCount);
		});

		it('stopPolling can be called when not polling', () => {
			expect(() => hostStore.stopPolling()).not.toThrow();
		});

		it('multiple start/stop cycles work correctly', async () => {
			vi.mocked(api.getHostMetrics).mockResolvedValue(mockMetrics);

			// First cycle
			hostStore.startPolling();
			await vi.waitFor(() => expect(api.getHostMetrics).toHaveBeenCalledOnce());
			hostStore.stopPolling();

			// Second cycle
			vi.clearAllMocks();
			hostStore.startPolling();
			await vi.waitFor(() => expect(api.getHostMetrics).toHaveBeenCalledOnce());
			hostStore.stopPolling();

			expect(api.getHostMetrics).toHaveBeenCalledOnce();
		});
	});

	describe('error handling during polling', () => {
		it('continues polling after error', async () => {
			vi.mocked(api.getHostMetrics)
				.mockRejectedValueOnce(new Error('First error'))
				.mockResolvedValueOnce(mockMetrics);

			hostStore.startPolling();
			await vi.waitFor(() => expect(hostStore.error).toBe('First error'));

			vi.advanceTimersByTime(10_000);
			await vi.waitFor(() => expect(hostStore.metrics).toEqual(mockMetrics));

			expect(hostStore.error).toBeNull();
		});
	});
});
