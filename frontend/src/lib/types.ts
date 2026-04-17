export type SessionKind =
	| { type: 'shell' }
	| { type: 'claude_code'; config_override?: string | null; project_dir?: string | null }
	| { type: 'raw_bytes' }
	| { type: 'codex'; project_dir?: string | null };

export type SessionState = 'booting' | 'idle' | 'streaming' | 'awaiting' | 'error' | 'exited';

export interface AgentMeta {
	name: string;
	version?: string | null;
	model?: string | null;
	tokens_used: number;
	last_tool_call?: string | null;
}

export interface ExitInfo {
	code?: number | null;
	signal?: string | null;
	reason: string;
}

export interface Session {
	id: string;
	slug: string;
	node_id: string;
	kind: SessionKind;
	state: SessionState;
	cwd: string;
	env: unknown;
	agent_meta?: AgentMeta | null;
	labels: unknown;
	created_at: number;
	last_activity_at: number;
	exit?: ExitInfo | null;
}

export interface LabelEntry {
	key: string;
	value: string;
}

export type TurnRole = 'system' | 'user' | 'assistant';

export type AgentEvent =
	| { type: 'turn_started'; turn_id: number; role: TurnRole; content_start?: string | null }
	| { type: 'turn_finished'; turn_id: number; tokens_used: number; duration_ms: number }
	| { type: 'thinking_block'; turn_id: number; len_chars: number }
	| {
			type: 'tool_call_started';
			turn_id: number;
			call_id: string;
			tool: string;
			args_preview: string;
	  }
	| {
			type: 'tool_call_finished';
			turn_id: number;
			call_id: string;
			ok: boolean;
			result_preview: string;
	  }
	| { type: 'awaiting_permission'; tool: string; prompt: string }
	| { type: 'model_changed'; model: string }
	| { type: 'error'; message: string }
	| { type: 'context_window_size_changed'; pct_used: number; tokens: number };

export type Event =
	| { type: 'session_created' }
	| { type: 'state_changed'; from: SessionState; to: SessionState }
	| { type: 'output_appended'; offset: number; len: number }
	| { type: 'input_received'; data: number[] }
	| { type: 'resized'; cols: number; rows: number }
	| { type: 'metrics_updated' }
	| { type: 'agent_event'; id: string; event: AgentEvent };

export interface StoredEvent {
	id: number;
	session_id: string;
	ts: number;
	kind: string;
	event: Event;
}

export interface CreateSessionRequest {
	slug: string;
	kind: SessionKind;
	cols?: number;
	rows?: number;
}

export type LogLevel = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7;

export interface LogLine {
	type: 'log';
	source: string;
	ts_us: number;
	level: LogLevel;
	body: string;
	unit?: string;
}

export interface LogSource {
	name: string;
	kind: string;
	active: boolean;
}

export type LogWsMessage =
	| LogLine
	| { type: 'initial_tail_complete' }
	| { type: 'lagged'; dropped: number }
	| { type: 'set_sources'; sources: string[] };
