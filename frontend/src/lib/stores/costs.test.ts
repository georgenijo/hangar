import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { costsStore } from './costs.svelte';
import * as api from '../api';
import type { DailyCost, ModelCost } from '../types';

// Mock the API module
vi.mock('../api');

describe('costsStore', () => {
	let mockDailyCosts: DailyCost[];
	let mockModelCosts: ModelCost[];

	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers();

		mockDailyCosts = [
			{ date: '2026-04-14', dollars: 10.5 },
			{ date: '2026-04-15', dollars: 15.2 },
			{ date: '2026-04-16', dollars: 8.3 },
			{ date: '2026-04-17', dollars: 12.1 },
			{ date: '2026-04-18', dollars: 9.7 },
			{ date: '2026-04-19', dollars: 14.8 },
			{ date: '2026-04-20', dollars: 11.4 }
		];

		mockModelCosts = [
			{ model: 'claude-opus-4', dollars: 45.2 },
			{ model: 'claude-sonnet-3.5', dollars: 28.7 },
			{ model: 'claude-haiku-3', dollars: 8.1 }
		];

		// Reset store state
		costsStore.stopPolling();
	});

	afterEach(() => {
		costsStore.stopPolling();
		vi.useRealTimers();
	});

	describe('initial state', () => {
		it('starts with empty dailyCosts array', () => {
			expect(costsStore.dailyCosts).toEqual([]);
		});

		it('starts with empty modelCosts array', () => {
			expect(costsStore.modelCosts).toEqual([]);
		});

		it('starts with loading false', () => {
			expect(costsStore.loading).toBe(false);
		});

		it('starts with no error', () => {
			expect(costsStore.error).toBeNull();
		});
	});

	describe('derived values', () => {
		it('totalSpend returns 0 when dailyCosts is empty', () => {
			expect(costsStore.totalSpend).toBe(0);
		});

		it('totalSpend calculates sum of all daily costs', async () => {
			vi.mocked(api.getCostsDaily).mockResolvedValue(mockDailyCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			await costsStore.refresh();

			// 10.5 + 15.2 + 8.3 + 12.1 + 9.7 + 14.8 + 11.4 = 82.0
			expect(costsStore.totalSpend).toBe(82.0);
		});

		it('last7DaysSpend returns last 7 entries', async () => {
			vi.mocked(api.getCostsDaily).mockResolvedValue(mockDailyCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			await costsStore.refresh();

			expect(costsStore.last7DaysSpend).toBe(82.0);
		});

		it('last7DaysSpend handles more than 7 days', async () => {
			const extendedCosts = [
				{ date: '2026-04-10', dollars: 5.0 },
				{ date: '2026-04-11', dollars: 6.0 },
				{ date: '2026-04-12', dollars: 7.0 },
				...mockDailyCosts
			];

			vi.mocked(api.getCostsDaily).mockResolvedValue(extendedCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			await costsStore.refresh();

			// Should only sum last 7: 8.3 + 12.1 + 9.7 + 14.8 + 11.4 = 82.0 (from original 7)
			expect(costsStore.last7DaysSpend).toBe(82.0);
		});

		it('last7DaysSpend handles less than 7 days', async () => {
			const shortCosts = [
				{ date: '2026-04-19', dollars: 14.8 },
				{ date: '2026-04-20', dollars: 11.4 }
			];

			vi.mocked(api.getCostsDaily).mockResolvedValue(shortCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			await costsStore.refresh();

			expect(costsStore.last7DaysSpend).toBe(26.2);
		});

		it('dailyAmounts returns array of dollar values', async () => {
			vi.mocked(api.getCostsDaily).mockResolvedValue(mockDailyCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			await costsStore.refresh();

			expect(costsStore.dailyAmounts).toEqual([10.5, 15.2, 8.3, 12.1, 9.7, 14.8, 11.4]);
		});

		it('dailyAmounts returns empty array when no costs', () => {
			expect(costsStore.dailyAmounts).toEqual([]);
		});
	});

	describe('refresh', () => {
		it('fetches both daily and model costs in parallel', async () => {
			vi.mocked(api.getCostsDaily).mockResolvedValue(mockDailyCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			await costsStore.refresh();

			expect(api.getCostsDaily).toHaveBeenCalledOnce();
			expect(api.getCostsByModel).toHaveBeenCalledOnce();
			expect(costsStore.dailyCosts).toEqual(mockDailyCosts);
			expect(costsStore.modelCosts).toEqual(mockModelCosts);
			expect(costsStore.error).toBeNull();
		});

		it('handles fetch error from getCostsDaily', async () => {
			const error = new Error('Daily costs error');
			vi.mocked(api.getCostsDaily).mockRejectedValue(error);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			await costsStore.refresh();

			expect(costsStore.dailyCosts).toEqual([]);
			expect(costsStore.error).toBe('Daily costs error');
		});

		it('handles fetch error from getCostsByModel', async () => {
			const error = new Error('Model costs error');
			vi.mocked(api.getCostsDaily).mockResolvedValue(mockDailyCosts);
			vi.mocked(api.getCostsByModel).mockRejectedValue(error);

			await costsStore.refresh();

			expect(costsStore.modelCosts).toEqual([]);
			expect(costsStore.error).toBe('Model costs error');
		});

		it('handles non-Error exceptions', async () => {
			vi.mocked(api.getCostsDaily).mockRejectedValue('string error');
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			await costsStore.refresh();

			expect(costsStore.error).toBe('Failed to fetch costs');
		});
	});

	describe('polling lifecycle', () => {
		it('startPolling initiates immediate fetch', async () => {
			vi.mocked(api.getCostsDaily).mockResolvedValue(mockDailyCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			costsStore.startPolling();
			await vi.waitFor(() => expect(api.getCostsDaily).toHaveBeenCalled());

			expect(api.getCostsDaily).toHaveBeenCalledOnce();
			expect(api.getCostsByModel).toHaveBeenCalledOnce();
		});

		it('startPolling sets up interval at 60s', async () => {
			vi.mocked(api.getCostsDaily).mockResolvedValue(mockDailyCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			costsStore.startPolling();
			await vi.waitFor(() => expect(api.getCostsDaily).toHaveBeenCalled());

			// Fast-forward 60 seconds
			vi.advanceTimersByTime(60_000);
			await vi.waitFor(() => expect(api.getCostsDaily).toHaveBeenCalledTimes(2));

			// Fast-forward another 60 seconds
			vi.advanceTimersByTime(60_000);
			await vi.waitFor(() => expect(api.getCostsDaily).toHaveBeenCalledTimes(3));
		});

		it('startPolling is idempotent', async () => {
			vi.mocked(api.getCostsDaily).mockResolvedValue(mockDailyCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			costsStore.startPolling();
			costsStore.startPolling();
			costsStore.startPolling();

			await vi.waitFor(() => expect(api.getCostsDaily).toHaveBeenCalled());

			vi.advanceTimersByTime(60_000);
			await vi.waitFor(() => expect(api.getCostsDaily).toHaveBeenCalledTimes(2));
		});

		it('stopPolling clears the interval', async () => {
			vi.mocked(api.getCostsDaily).mockResolvedValue(mockDailyCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			costsStore.startPolling();
			await vi.waitFor(() => expect(api.getCostsDaily).toHaveBeenCalled());

			costsStore.stopPolling();

			const callCount = vi.mocked(api.getCostsDaily).mock.calls.length;

			// Advance time - should not trigger more calls
			vi.advanceTimersByTime(120_000);
			await vi.waitFor(() => Promise.resolve());

			expect(api.getCostsDaily).toHaveBeenCalledTimes(callCount);
		});

		it('stopPolling can be called when not polling', () => {
			expect(() => costsStore.stopPolling()).not.toThrow();
		});
	});

	describe('error handling during polling', () => {
		it('continues polling after error', async () => {
			vi.mocked(api.getCostsDaily)
				.mockRejectedValueOnce(new Error('First error'))
				.mockResolvedValueOnce(mockDailyCosts);
			vi.mocked(api.getCostsByModel).mockResolvedValue(mockModelCosts);

			costsStore.startPolling();
			await vi.waitFor(() => expect(costsStore.error).toBe('First error'));

			vi.advanceTimersByTime(60_000);
			await vi.waitFor(() => expect(costsStore.dailyCosts).toEqual(mockDailyCosts));

			expect(costsStore.error).toBeNull();
		});
	});
});
