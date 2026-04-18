import { getSessionEvents } from '../api';
import type { StoredEvent, AgentEvent } from '../types';

let events: StoredEvent[] = $state([]);
let sessionId: string | null = $state(null);
let polling: boolean = $state(false);
let lastEventTs: number = $state(0);

let intervalId: ReturnType<typeof setInterval> | null = null;

interface ToolCall {
	callId: string;
	tool: string;
	argsPreview: string;
	ok?: boolean;
	resultPreview?: string;
	finished: boolean;
}

interface TurnGroup {
	turnId: number;
	role: string;
	contentStart?: string | null;
	thinkingBlocks: Array<{ lenChars: number }>;
	toolCalls: ToolCall[];
	isComplete: boolean;
}

function buildChatMessages(evs: StoredEvent[]): TurnGroup[] {
	const groups = new Map<number, TurnGroup>();

	for (const storedEv of evs) {
		if (storedEv.event.type !== 'agent_event') continue;
		const ae: AgentEvent = storedEv.event.event;

		if (ae.type === 'turn_started') {
			groups.set(ae.turn_id, {
				turnId: ae.turn_id,
				role: ae.role,
				contentStart: ae.content_start,
				thinkingBlocks: [],
				toolCalls: [],
				isComplete: false
			});
		} else if (ae.type === 'turn_finished') {
			const g = groups.get(ae.turn_id);
			if (g) g.isComplete = true;
		} else if (ae.type === 'thinking_block') {
			const g = groups.get(ae.turn_id);
			if (g) g.thinkingBlocks.push({ lenChars: ae.len_chars });
		} else if (ae.type === 'tool_call_started') {
			const g = groups.get(ae.turn_id);
			if (g) {
				g.toolCalls.push({
					callId: ae.call_id,
					tool: ae.tool,
					argsPreview: ae.args_preview,
					finished: false
				});
			}
		} else if (ae.type === 'tool_call_finished') {
			const g = groups.get(ae.turn_id);
			if (g) {
				const tc = g.toolCalls.find((t) => t.callId === ae.call_id);
				if (tc) {
					tc.ok = ae.ok;
					tc.resultPreview = ae.result_preview;
					tc.finished = true;
				}
			}
		}
	}

	return Array.from(groups.values());
}

function findAwaitingPermission(evs: StoredEvent[]): { tool: string; prompt: string } | null {
	for (let i = evs.length - 1; i >= 0; i--) {
		const ev = evs[i];
		if (ev.event.type !== 'agent_event') continue;
		const ae: AgentEvent = ev.event.event;
		if (ae.type === 'awaiting_permission') return { tool: ae.tool, prompt: ae.prompt };
		if (ae.type === 'tool_call_started') return null;
	}
	return null;
}

function findContextUsage(evs: StoredEvent[]): { pctUsed: number; tokens: number } | null {
	for (let i = evs.length - 1; i >= 0; i--) {
		const ev = evs[i];
		if (ev.event.type !== 'agent_event') continue;
		const ae: AgentEvent = ev.event.event;
		if (ae.type === 'context_window_size_changed')
			return { pctUsed: ae.pct_used, tokens: ae.tokens };
	}
	return null;
}

function findRecentToolCalls(evs: StoredEvent[]): ToolCall[] {
	const finishedMap = new Map<string, { ok: boolean; resultPreview: string }>();

	for (const storedEv of evs) {
		if (storedEv.event.type !== 'agent_event') continue;
		const ae: AgentEvent = storedEv.event.event;
		if (ae.type === 'tool_call_finished') {
			finishedMap.set(ae.call_id, { ok: ae.ok, resultPreview: ae.result_preview });
		}
	}

	const result: ToolCall[] = [];
	for (let i = evs.length - 1; i >= 0 && result.length < 5; i--) {
		const ev = evs[i];
		if (ev.event.type !== 'agent_event') continue;
		const ae: AgentEvent = ev.event.event;
		if (ae.type === 'tool_call_started') {
			const finished = finishedMap.get(ae.call_id);
			result.push({
				callId: ae.call_id,
				tool: ae.tool,
				argsPreview: ae.args_preview,
				ok: finished?.ok,
				resultPreview: finished?.resultPreview,
				finished: !!finished
			});
		}
	}

	return result.reverse();
}

// Output tokens only — input cost not tracked. Estimate is approximate.
const OUTPUT_RATES: Record<string, number> = {
	'claude-sonnet-4': 15 / 1000,
	'claude-opus-4': 75 / 1000,
	'claude-haiku-3.5': 4 / 1000
};
const DEFAULT_OUTPUT_RATE = 15 / 1000;

function computeOutputCost(evs: StoredEvent[]): {
	totalTokens: number;
	estimatedCost: number;
	model: string | null;
} {
	let totalTokens = 0;
	let model: string | null = null;

	for (const storedEv of evs) {
		if (storedEv.event.type !== 'agent_event') continue;
		const ae: AgentEvent = storedEv.event.event;
		if (ae.type === 'turn_finished') {
			totalTokens += ae.tokens_used;
		} else if (ae.type === 'model_changed') {
			model = ae.model;
		}
	}

	const rateKey = model
		? Object.keys(OUTPUT_RATES).find((k) => model!.startsWith(k))
		: undefined;
	const rate = rateKey ? OUTPUT_RATES[rateKey] : DEFAULT_OUTPUT_RATE;
	const estimatedCost = (totalTokens / 1000) * rate;

	return { totalTokens, estimatedCost, model };
}

function findCurrentModel(evs: StoredEvent[]): string | null {
	for (let i = evs.length - 1; i >= 0; i--) {
		const ev = evs[i];
		if (ev.event.type !== 'agent_event') continue;
		const ae: AgentEvent = ev.event.event;
		if (ae.type === 'model_changed') return ae.model;
	}
	return null;
}

export const eventsStore = {
	get events() {
		return events;
	},
	get polling() {
		return polling;
	},
	get chatMessages(): TurnGroup[] {
		return buildChatMessages(events);
	},
	get awaitingPermission() {
		return findAwaitingPermission(events);
	},
	get contextUsage() {
		return findContextUsage(events);
	},
	get recentToolCalls(): ToolCall[] {
		return findRecentToolCalls(events);
	},
	get outputCost() {
		return computeOutputCost(events);
	},
	get currentModel(): string | null {
		return findCurrentModel(events);
	},

	async loadInitialEvents(id: string): Promise<void> {
		const data = await getSessionEvents(id, { limit: 100 });
		events = data;
		if (data.length > 0) {
			lastEventTs = Math.max(...data.map((e) => e.ts));
		}
	},

	startEventPolling(id: string, isStreaming = false) {
		if (intervalId !== null) clearInterval(intervalId);
		sessionId = id;
		polling = true;

		const tick = async () => {
			try {
				const data = await getSessionEvents(id, { since: lastEventTs, limit: 200 });
				if (data.length > 0) {
					events = [...events, ...data];
					lastEventTs = Math.max(...data.map((e) => e.ts));
				}
			} catch {
				// silent — polling will retry
			}
		};

		const ms = isStreaming ? 500 : 1000;
		intervalId = setInterval(tick, ms);
	},

	stopEventPolling() {
		if (intervalId !== null) {
			clearInterval(intervalId);
			intervalId = null;
		}
		polling = false;
		events = [];
		sessionId = null;
		lastEventTs = 0;
	}
};
