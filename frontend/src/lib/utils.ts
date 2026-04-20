import type { SessionState } from './types';

export function formatIdleTime(lastActivityAt: number): string {
	const diffMs = Date.now() - lastActivityAt;
	const secs = Math.floor(diffMs / 1000);
	if (secs < 60) return `${secs}s`;
	const mins = Math.floor(secs / 60);
	if (mins < 60) return `${mins}m`;
	const hours = Math.floor(mins / 60);
	if (hours < 24) return `${hours}h`;
	const days = Math.floor(hours / 24);
	return `${days}d`;
}

/**
 * Format a dollar amount with $ prefix and 2 decimal places
 * @example formatCost(12.84) => "$12.84"
 * @example formatCost(0.99) => "$0.99"
 */
export function formatCost(dollars: number): string {
	return `$${dollars.toFixed(2)}`;
}

/**
 * Format token count with k/M suffix
 * @example formatTokens(1500) => "2k"
 * @example formatTokens(72104) => "72k"
 * @example formatTokens(1500000) => "1.5M"
 */
export function formatTokens(tokens: number): string {
	if (tokens >= 1_000_000) {
		const m = tokens / 1_000_000;
		return m % 1 === 0 ? `${m}M` : `${m.toFixed(1)}M`;
	}
	if (tokens >= 1000) {
		const k = Math.round(tokens / 1000);
		return `${k}k`;
	}
	return String(tokens);
}

/**
 * Format duration in seconds to human readable
 * @example formatDuration(65) => "1m 5s"
 * @example formatDuration(3661) => "1h 1m"
 */
export function formatDuration(seconds: number): string {
	if (seconds < 60) return `${seconds}s`;
	if (seconds < 3600) {
		const m = Math.floor(seconds / 60);
		const s = seconds % 60;
		return s > 0 ? `${m}m ${s}s` : `${m}m`;
	}
	const h = Math.floor(seconds / 3600);
	const m = Math.floor((seconds % 3600) / 60);
	return m > 0 ? `${h}h ${m}m` : `${h}h`;
}

/**
 * Format bytes to human readable with appropriate unit
 * @example formatBytes(1073741824) => "1.0 GB"
 */
export function formatBytes(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/**
 * Format percentage with % suffix
 * @example formatPct(0.28) => "28%"
 */
export function formatPct(ratio: number): string {
	return `${Math.round(ratio * 100)}%`;
}

/**
 * Clamp a value between min and max
 */
export function clamp(value: number, min: number, max: number): number {
	return Math.min(Math.max(value, min), max);
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
