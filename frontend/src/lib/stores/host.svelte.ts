import { getHostMetrics } from '../api';
import type { HostMetrics } from '../types';

export function createHostStore() {
	// Reactive state using Svelte 5 runes
	let metrics: HostMetrics | null = $state(null);
	let loading: boolean = $state(false);
	let error: string | null = $state(null);
	let intervalId: ReturnType<typeof setInterval> | null = null;

	const POLL_INTERVAL_MS = 10_000; // 10 seconds

	async function fetchMetrics(): Promise<void> {
		try {
			loading = true;
			error = null;
			metrics = await getHostMetrics();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch host metrics';
			metrics = null; // Clear stale data on error
		} finally {
			loading = false;
		}
	}

	return {
		// Getters expose reactive state
		get metrics() {
			return metrics;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},

		// Derived values for common access patterns
		get cpuPct() {
			return metrics?.cpu_pct ?? 0;
		},
		get ramPct() {
			if (!metrics || metrics.ram_total_bytes === 0) return 0;
			return (metrics.ram_used_bytes / metrics.ram_total_bytes) * 100;
		},
		get diskPct() {
			if (!metrics || metrics.disk_total_bytes === 0) return 0;
			return (metrics.disk_used_bytes / metrics.disk_total_bytes) * 100;
		},

		/** Fetch metrics once */
		async refresh(): Promise<void> {
			await fetchMetrics();
		},

		/** Start polling at 10s intervals */
		startPolling(): void {
			if (intervalId !== null) return; // Already polling
			fetchMetrics(); // Immediate fetch
			intervalId = setInterval(fetchMetrics, POLL_INTERVAL_MS);
		},

		/** Stop polling and clear state */
		stopPolling(): void {
			if (intervalId !== null) {
				clearInterval(intervalId);
				intervalId = null;
			}
		}
	};
}

export const hostStore = createHostStore();
