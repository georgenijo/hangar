import { getPipelineRuns } from '../api';
import type { PipelineRun } from '../types';

let runs: PipelineRun[] = $state([]);
let loading: boolean = $state(false);
let error: string | null = $state(null);
let intervalId: ReturnType<typeof setInterval> | null = null;

const POLL_INTERVAL_MS = 5_000; // 5 seconds (pipelines update frequently)

async function fetchRuns(): Promise<void> {
	try {
		loading = true;
		error = null;
		runs = await getPipelineRuns();
	} catch (e) {
		error = e instanceof Error ? e.message : 'Failed to fetch pipeline runs';
	} finally {
		loading = false;
	}
}

export const pipelineStore = {
	get runs() {
		return runs;
	},
	get loading() {
		return loading;
	},
	get error() {
		return error;
	},

	// Derived: active (live) runs
	get activeRuns(): PipelineRun[] {
		return runs.filter((r) => r.state === 'live');
	},

	// Derived: count by state
	get countByState(): Record<string, number> {
		const counts: Record<string, number> = { pending: 0, live: 0, done: 0, failed: 0 };
		for (const r of runs) {
			counts[r.state] = (counts[r.state] || 0) + 1;
		}
		return counts;
	},

	async refresh(): Promise<void> {
		await fetchRuns();
	},

	startPolling(): void {
		if (intervalId !== null) return;
		fetchRuns();
		intervalId = setInterval(fetchRuns, POLL_INTERVAL_MS);
	},

	stopPolling(): void {
		if (intervalId !== null) {
			clearInterval(intervalId);
			intervalId = null;
		}
	}
};
