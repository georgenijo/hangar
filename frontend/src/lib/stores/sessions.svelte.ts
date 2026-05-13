import { listSessions, normalizeLabels } from '../api';
import type { Session, LabelEntry } from '../types';

let sessions: Session[] = $state([]);
let loading: boolean = $state(true);
let error: string | null = $state(null);
let selectedLabels: LabelEntry[] = $state([]);

let intervalId: ReturnType<typeof setInterval> | null = null;
let consecutiveFailures = 0;

function filterByLabels(list: Session[], labels: LabelEntry[]): Session[] {
	if (labels.length === 0) return list;
	return list.filter((s) => {
		const normalized = normalizeLabels(s.labels);
		return labels.every((filter) =>
			normalized.some((l) => l.key === filter.key && (filter.value === '' || l.value === filter.value))
		);
	});
}

function computeAllLabels(list: Session[]): LabelEntry[] {
	const seen = new Map<string, LabelEntry>();
	for (const s of list) {
		for (const l of normalizeLabels(s.labels)) {
			const k = `${l.key}=${l.value}`;
			if (!seen.has(k)) seen.set(k, l);
		}
	}
	return Array.from(seen.values());
}

export const sessionsStore = {
	get sessions() {
		return sessions;
	},
	get loading() {
		return loading;
	},
	get error() {
		return error;
	},
	get selectedLabels() {
		return selectedLabels;
	},
	get filteredSessions() {
		const sorted = [...sessions].sort((a, b) => {
			const aActive = a.state !== 'exited' ? 1 : 0;
			const bActive = b.state !== 'exited' ? 1 : 0;
			if (aActive !== bActive) return bActive - aActive;
			return b.last_activity_at - a.last_activity_at;
		});
		return filterByLabels(sorted, selectedLabels);
	},
	get hasActiveSessions() {
		return sessions.some((s) => ['streaming', 'awaiting', 'booting'].includes(s.state));
	},
	get hasStreamingSessions() {
		return sessions.some((s) => s.state === 'streaming');
	},
	get allLabels() {
		return computeAllLabels(sessions);
	},

	getSessionById(id: string): Session | undefined {
		return sessions.find((s) => s.id === id);
	},

	removeSession(id: string) {
		sessions = sessions.filter((s) => s.id !== id);
	},

	toggleLabelFilter(label: LabelEntry) {
		const idx = selectedLabels.findIndex((l) => l.key === label.key && l.value === label.value);
		if (idx >= 0) {
			selectedLabels = selectedLabels.filter((_, i) => i !== idx);
		} else {
			selectedLabels = [...selectedLabels, label];
		}
	},

	clearLabelFilters() {
		selectedLabels = [];
	},

	startPolling() {
		if (intervalId !== null) return;

		const tick = async () => {
			try {
				const data = await listSessions();
				sessions = data;
				loading = false;
				error = null;
				consecutiveFailures = 0;
			} catch (e) {
				consecutiveFailures++;
				if (consecutiveFailures >= 3) {
					error = e instanceof Error ? e.message : 'Failed to load sessions';
				}
			}

			if (intervalId !== null) {
				clearInterval(intervalId);
				const isActive = sessions.some((s) => ['streaming', 'awaiting', 'booting'].includes(s.state));
				const pollMs =
					consecutiveFailures >= 3 ? 10000 : isActive ? 500 : 3000;
				intervalId = setInterval(tick, pollMs);
			}
		};

		const pollMs = 1000;
		intervalId = setInterval(tick, pollMs);
		tick();
	},

	stopPolling() {
		if (intervalId !== null) {
			clearInterval(intervalId);
			intervalId = null;
		}
	},

	retry() {
		consecutiveFailures = 0;
		error = null;
	}
};
