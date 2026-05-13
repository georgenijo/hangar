import type { SessionState } from './types';

export function formatIdleTime(lastActivityAt: number): string {
	if (!Number.isFinite(lastActivityAt)) return '—';
	const diffMs = Date.now() - lastActivityAt;
	const secs = Math.max(0, Math.floor(diffMs / 1000));
	if (secs < 60) return `${secs}s`;
	const mins = Math.floor(secs / 60);
	if (mins < 60) return `${mins}m`;
	const hours = Math.floor(mins / 60);
	if (hours < 24) return `${hours}h`;
	const days = Math.floor(hours / 24);
	return `${days}d`;
}

export function formatTokens(n: number): string {
	if (n == null || !Number.isFinite(n)) return '—';
	return new Intl.NumberFormat().format(n);
}

export function truncate(s: string, maxLen: number): string {
	if (s.length <= maxLen) return s;
	return s.slice(0, maxLen - 3) + '...';
}

export function stateColor(state: SessionState): string {
	switch (state) {
		case 'booting':
			return '#f5c518';
		case 'idle':
			return '#5b8bd4';
		case 'streaming':
			return '#4caf50';
		case 'awaiting':
			return '#ff9800';
		case 'error':
			return '#f44336';
		case 'exited':
			return '#3a3a3a';
	}
}

export function isActiveState(state: SessionState): boolean {
	return state === 'booting' || state === 'idle' || state === 'streaming' || state === 'awaiting';
}

const IN_FLIGHT_STATES = new Set(['running', 'queued', 'pending', 'in_progress']);
export function isPipelineInFlight(state: string): boolean {
	return IN_FLIGHT_STATES.has(state.toLowerCase());
}
