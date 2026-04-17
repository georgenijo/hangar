import type { LogLine, LogSource } from '../types';

const MAX_LINES = 10000;

let lines: LogLine[] = $state([]);
let sources: LogSource[] = $state([]);
let activeSources: Set<string> = $state(new Set());
let searchPattern: string = $state('');
let searchRegex: RegExp | null = $state(null);
let paused: boolean = $state(false);
let autoScroll: boolean = $state(true);
let initialTailComplete: boolean = $state(false);
let connected: boolean = $state(false);
let pauseBuffer: LogLine[] = $state([]);

function applyFilter(all: LogLine[]): LogLine[] {
	let result = all;
	if (activeSources.size > 0) {
		result = result.filter((l) => activeSources.has(l.source));
	}
	if (searchRegex) {
		const re = searchRegex;
		result = result.filter((l) => re.test(l.body));
	}
	return result;
}

export const logsStore = {
	get lines() {
		return lines;
	},
	get sources() {
		return sources;
	},
	get activeSources() {
		return activeSources;
	},
	get searchPattern() {
		return searchPattern;
	},
	get searchRegex() {
		return searchRegex;
	},
	get paused() {
		return paused;
	},
	get autoScroll() {
		return autoScroll;
	},
	get initialTailComplete() {
		return initialTailComplete;
	},
	get connected() {
		return connected;
	},
	get filteredLines() {
		return applyFilter(lines);
	},

	setSources(s: LogSource[]) {
		sources = s;
	},

	toggleSource(name: string) {
		const next = new Set(activeSources);
		if (next.has(name)) {
			next.delete(name);
		} else {
			next.add(name);
		}
		activeSources = next;
	},

	setSearch(pattern: string) {
		searchPattern = pattern;
		if (!pattern) {
			searchRegex = null;
			return;
		}
		try {
			searchRegex = new RegExp(pattern, 'i');
		} catch {
			searchRegex = null;
		}
	},

	togglePause() {
		if (paused) {
			// Resume: flush buffer
			const merged = [...lines, ...pauseBuffer];
			lines = merged.slice(-MAX_LINES);
			pauseBuffer = [];
		}
		paused = !paused;
	},

	setAutoScroll(val: boolean) {
		autoScroll = val;
	},

	addLine(line: LogLine) {
		if (paused) {
			pauseBuffer = [...pauseBuffer, line].slice(-MAX_LINES);
		} else {
			lines = [...lines, line].slice(-MAX_LINES);
		}
	},

	setInitialTailComplete(val: boolean) {
		initialTailComplete = val;
	},

	setConnected(val: boolean) {
		connected = val;
	},

	clear() {
		lines = [];
		pauseBuffer = [];
	},

	reset() {
		lines = [];
		pauseBuffer = [];
		initialTailComplete = false;
		connected = false;
		paused = false;
		autoScroll = true;
	}
};
