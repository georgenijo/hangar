import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { createPipelineStore } from './pipeline.svelte';
import * as api from '../api';
import type { PipelineRun } from '../types';

// Mock the API module
vi.mock('../api');

describe('pipelineStore', () => {
	let pipelineStore: ReturnType<typeof createPipelineStore>;
	let mockRuns: PipelineRun[];

	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers();

		// Create fresh store instance for each test
		pipelineStore = createPipelineStore();

		mockRuns = [
			{
				issue: 79,
				title: 'cost scraper lag fix',
				state: 'live',
				phase: 'tester',
				cost: 4.82,
				tokens: 482000,
				agents: 5,
				duration_s: 6120
			},
			{
				issue: 71,
				title: 'sidebar session switcher',
				state: 'live',
				phase: 'architect',
				cost: 0.88,
				tokens: 82000,
				agents: 2,
				duration_s: 1920
			},
			{
				issue: 65,
				title: 'implement dark mode',
				state: 'done',
				phase: 'pr',
				cost: 12.5,
				tokens: 1200000,
				agents: 8,
				duration_s: 14400
			},
			{
				issue: 55,
				title: 'add authentication',
				state: 'failed',
				phase: 'builder',
				cost: 3.2,
				tokens: 320000,
				agents: 4,
				duration_s: 3600
			},
			{
				issue: 45,
				title: 'optimize queries',
				state: 'pending',
				phase: 'planner',
				cost: 0,
				tokens: 0,
				agents: 0,
				duration_s: 0
			}
		];
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	describe('initial state', () => {
		it('starts with empty runs array', () => {
			expect(pipelineStore.runs).toEqual([]);
		});

		it('starts with loading false', () => {
			expect(pipelineStore.loading).toBe(false);
		});

		it('starts with no error', () => {
			expect(pipelineStore.error).toBeNull();
		});
	});

	describe('derived values', () => {
		it('activeRuns returns empty array when runs is empty', () => {
			expect(pipelineStore.activeRuns).toEqual([]);
		});

		it('activeRuns returns only live runs', async () => {
			vi.mocked(api.getPipelineRuns).mockResolvedValue(mockRuns);

			await pipelineStore.refresh();

			const activeRuns = pipelineStore.activeRuns;
			expect(activeRuns).toHaveLength(2);
			expect(activeRuns[0].issue).toBe(79);
			expect(activeRuns[1].issue).toBe(71);
			expect(activeRuns.every((r) => r.state === 'live')).toBe(true);
		});

		it('activeRuns handles no live runs', async () => {
			const noLiveRuns = mockRuns.filter((r) => r.state !== 'live');
			vi.mocked(api.getPipelineRuns).mockResolvedValue(noLiveRuns);

			await pipelineStore.refresh();

			expect(pipelineStore.activeRuns).toEqual([]);
		});

		it('countByState returns correct counts', async () => {
			vi.mocked(api.getPipelineRuns).mockResolvedValue(mockRuns);

			await pipelineStore.refresh();

			const counts = pipelineStore.countByState;
			expect(counts.live).toBe(2);
			expect(counts.done).toBe(1);
			expect(counts.failed).toBe(1);
			expect(counts.pending).toBe(1);
		});

		it('countByState returns zeros for empty runs', () => {
			const counts = pipelineStore.countByState;
			expect(counts.live).toBe(0);
			expect(counts.done).toBe(0);
			expect(counts.failed).toBe(0);
			expect(counts.pending).toBe(0);
		});

		it('countByState handles all same state', async () => {
			const allLive = mockRuns.map((r) => ({ ...r, state: 'live' as const }));
			vi.mocked(api.getPipelineRuns).mockResolvedValue(allLive);

			await pipelineStore.refresh();

			const counts = pipelineStore.countByState;
			expect(counts.live).toBe(5);
			expect(counts.done).toBe(0);
			expect(counts.failed).toBe(0);
			expect(counts.pending).toBe(0);
		});
	});

	describe('refresh', () => {
		it('fetches runs successfully', async () => {
			vi.mocked(api.getPipelineRuns).mockResolvedValue(mockRuns);

			await pipelineStore.refresh();

			expect(api.getPipelineRuns).toHaveBeenCalledOnce();
			expect(pipelineStore.runs).toEqual(mockRuns);
			expect(pipelineStore.error).toBeNull();
		});

		it('handles fetch error', async () => {
			const error = new Error('Pipeline fetch error');
			vi.mocked(api.getPipelineRuns).mockRejectedValue(error);

			await pipelineStore.refresh();

			expect(pipelineStore.runs).toEqual([]);
			expect(pipelineStore.error).toBe('Pipeline fetch error');
		});

		it('handles non-Error exceptions', async () => {
			vi.mocked(api.getPipelineRuns).mockRejectedValue('string error');

			await pipelineStore.refresh();

			expect(pipelineStore.error).toBe('Failed to fetch pipeline runs');
		});
	});

	describe('polling lifecycle', () => {
		it('startPolling initiates immediate fetch', async () => {
			vi.mocked(api.getPipelineRuns).mockResolvedValue(mockRuns);

			pipelineStore.startPolling();
			await vi.waitFor(() => expect(api.getPipelineRuns).toHaveBeenCalled());

			expect(api.getPipelineRuns).toHaveBeenCalledOnce();
		});

		it('startPolling sets up interval at 5s', async () => {
			vi.mocked(api.getPipelineRuns).mockResolvedValue(mockRuns);

			pipelineStore.startPolling();
			await vi.waitFor(() => expect(api.getPipelineRuns).toHaveBeenCalled());

			// Fast-forward 5 seconds
			vi.advanceTimersByTime(5_000);
			await vi.waitFor(() => expect(api.getPipelineRuns).toHaveBeenCalledTimes(2));

			// Fast-forward another 5 seconds
			vi.advanceTimersByTime(5_000);
			await vi.waitFor(() => expect(api.getPipelineRuns).toHaveBeenCalledTimes(3));

			// Fast-forward another 5 seconds
			vi.advanceTimersByTime(5_000);
			await vi.waitFor(() => expect(api.getPipelineRuns).toHaveBeenCalledTimes(4));
		});

		it('startPolling is idempotent', async () => {
			vi.mocked(api.getPipelineRuns).mockResolvedValue(mockRuns);

			pipelineStore.startPolling();
			pipelineStore.startPolling();
			pipelineStore.startPolling();

			await vi.waitFor(() => expect(api.getPipelineRuns).toHaveBeenCalled());

			vi.advanceTimersByTime(5_000);
			await vi.waitFor(() => expect(api.getPipelineRuns).toHaveBeenCalledTimes(2));
		});

		it('stopPolling clears the interval', async () => {
			vi.mocked(api.getPipelineRuns).mockResolvedValue(mockRuns);

			pipelineStore.startPolling();
			await vi.waitFor(() => expect(api.getPipelineRuns).toHaveBeenCalled());

			pipelineStore.stopPolling();

			const callCount = vi.mocked(api.getPipelineRuns).mock.calls.length;

			// Advance time - should not trigger more calls
			vi.advanceTimersByTime(30_000);
			await vi.waitFor(() => Promise.resolve());

			expect(api.getPipelineRuns).toHaveBeenCalledTimes(callCount);
		});

		it('stopPolling can be called when not polling', () => {
			expect(() => pipelineStore.stopPolling()).not.toThrow();
		});

		it('multiple start/stop cycles work correctly', async () => {
			vi.mocked(api.getPipelineRuns).mockResolvedValue(mockRuns);

			// First cycle
			pipelineStore.startPolling();
			await vi.waitFor(() => expect(api.getPipelineRuns).toHaveBeenCalledOnce());
			pipelineStore.stopPolling();

			// Second cycle
			vi.clearAllMocks();
			pipelineStore.startPolling();
			await vi.waitFor(() => expect(api.getPipelineRuns).toHaveBeenCalledOnce());
			pipelineStore.stopPolling();

			expect(api.getPipelineRuns).toHaveBeenCalledOnce();
		});
	});

	describe('error handling during polling', () => {
		it('continues polling after error', async () => {
			vi.mocked(api.getPipelineRuns)
				.mockRejectedValueOnce(new Error('First error'))
				.mockResolvedValueOnce(mockRuns);

			pipelineStore.startPolling();
			await vi.waitFor(() => expect(pipelineStore.error).toBe('First error'));

			vi.advanceTimersByTime(5_000);
			await vi.waitFor(() => expect(pipelineStore.runs).toEqual(mockRuns));

			expect(pipelineStore.error).toBeNull();
		});
	});

	describe('real-world scenarios', () => {
		it('handles run state transitions', async () => {
			const initialRuns = [{ ...mockRuns[0], state: 'pending' as const }];
			const updatedRuns = [{ ...mockRuns[0], state: 'live' as const }];

			vi.mocked(api.getPipelineRuns).mockResolvedValueOnce(initialRuns);

			await pipelineStore.refresh();
			expect(pipelineStore.activeRuns).toHaveLength(0);

			vi.mocked(api.getPipelineRuns).mockResolvedValueOnce(updatedRuns);
			await pipelineStore.refresh();
			expect(pipelineStore.activeRuns).toHaveLength(1);
		});

		it('handles empty response', async () => {
			vi.mocked(api.getPipelineRuns).mockResolvedValue([]);

			await pipelineStore.refresh();

			expect(pipelineStore.runs).toEqual([]);
			expect(pipelineStore.activeRuns).toEqual([]);
			expect(pipelineStore.countByState.live).toBe(0);
		});
	});
});
