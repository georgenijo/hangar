import { getCostsDaily, getCostsByModel } from '../api';
import type { DailyCost, ModelCost } from '../types';

export function createCostsStore() {
	let dailyCosts: DailyCost[] = $state([]);
	let modelCosts: ModelCost[] = $state([]);
	let loading: boolean = $state(false);
	let error: string | null = $state(null);
	let intervalId: ReturnType<typeof setInterval> | null = null;

	const POLL_INTERVAL_MS = 60_000; // 60 seconds (costs don't change rapidly)

	async function fetchCosts(): Promise<void> {
		try {
			loading = true;
			error = null;
			// Fetch both in parallel
			const [daily, byModel] = await Promise.all([getCostsDaily(), getCostsByModel()]);
			dailyCosts = daily;
			modelCosts = byModel;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch costs';
			dailyCosts = []; // Clear stale data on error
			modelCosts = []; // Clear stale data on error
		} finally {
			loading = false;
		}
	}

	return {
		get dailyCosts() {
			return dailyCosts;
		},
		get modelCosts() {
			return modelCosts;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},

		// Derived: total spend over loaded period
		get totalSpend(): number {
			return dailyCosts.reduce((sum, d) => sum + d.dollars, 0);
		},

		// Derived: last 7 days spend
		get last7DaysSpend(): number {
			const last7 = dailyCosts.slice(-7);
			return last7.reduce((sum, d) => sum + d.dollars, 0);
		},

		// Derived: daily amounts as array for sparkline
		get dailyAmounts(): number[] {
			return dailyCosts.map((d) => d.dollars);
		},

		async refresh(): Promise<void> {
			await fetchCosts();
		},

		startPolling(): void {
			if (intervalId !== null) return;
			fetchCosts();
			intervalId = setInterval(fetchCosts, POLL_INTERVAL_MS);
		},

		stopPolling(): void {
			if (intervalId !== null) {
				clearInterval(intervalId);
				intervalId = null;
			}
		}
	};
}

export const costsStore = createCostsStore();
