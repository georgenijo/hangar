import type { Session, StoredEvent, CreateSessionRequest, LabelEntry, SessionKind, LogSource } from './types';

export class ApiError extends Error {
	constructor(
		public status: number,
		public body: string
	) {
		super(`HTTP ${status}: ${body}`);
	}
}

async function checkOk(res: Response): Promise<Response> {
	if (!res.ok) {
		const body = await res.text().catch(() => '');
		throw new ApiError(res.status, body);
	}
	return res;
}

export async function listSessions(): Promise<Session[]> {
	const res = await checkOk(await fetch('/api/v1/sessions'));
	return res.json();
}

export async function getSessionEvents(
	id: string,
	opts?: { since?: number; kind?: string; limit?: number }
): Promise<StoredEvent[]> {
	const params = new URLSearchParams();
	if (opts?.since !== undefined) params.set('since', String(opts.since));
	if (opts?.kind) params.set('kind', opts.kind);
	if (opts?.limit !== undefined) params.set('limit', String(opts.limit));
	const qs = params.toString();
	const res = await checkOk(await fetch(`/api/v1/sessions/${id}/events${qs ? '?' + qs : ''}`));
	return res.json();
}

export async function getSessionOutput(
	id: string,
	opts?: { offset?: number; len?: number }
): Promise<{ data: ArrayBuffer; head: number; capacity: number }> {
	const params = new URLSearchParams();
	if (opts?.offset !== undefined) params.set('offset', String(opts.offset));
	if (opts?.len !== undefined) params.set('len', String(opts.len));
	const qs = params.toString();
	const res = await checkOk(await fetch(`/api/v1/sessions/${id}/output${qs ? '?' + qs : ''}`));
	const head = parseInt(res.headers.get('X-Ring-Head') ?? '0', 10);
	const capacity = parseInt(res.headers.get('X-Ring-Capacity') ?? '0', 10);
	const data = await res.arrayBuffer();
	return { data, head, capacity };
}

export async function createSession(req: CreateSessionRequest): Promise<Session> {
	const res = await checkOk(
		await fetch('/api/v1/sessions', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(req)
		})
	);
	return res.json();
}

export async function resizeSession(id: string, cols: number, rows: number): Promise<void> {
	await checkOk(
		await fetch(`/api/v1/sessions/${id}/resize`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ cols, rows })
		})
	);
}

export function normalizeLabels(raw: unknown): LabelEntry[] {
	if (raw === null || raw === undefined) return [];
	if (Array.isArray(raw)) {
		return raw.flatMap((item) => {
			if (typeof item === 'string') return [{ key: item, value: '' }];
			if (item && typeof item === 'object' && 'key' in item) {
				return [{ key: String((item as Record<string, unknown>).key), value: String((item as Record<string, unknown>).value ?? '') }];
			}
			return [];
		});
	}
	if (typeof raw === 'object') {
		return Object.entries(raw as Record<string, unknown>).map(([k, v]) => ({
			key: k,
			value: String(v ?? '')
		}));
	}
	return [];
}

export function kindLabel(k: SessionKind): string {
	switch (k.type) {
		case 'shell':
			return 'Shell';
		case 'claude_code':
			return 'Claude Code';
		case 'raw_bytes':
			return 'Raw Bytes';
	}
}

export async function listLogSources(): Promise<LogSource[]> {
	const res = await checkOk(await fetch('/api/v1/logs/sources'));
	return res.json();
}

export function kindIcon(k: SessionKind): string {
	switch (k.type) {
		case 'shell':
			return 'terminal';
		case 'claude_code':
			return 'bot';
		case 'raw_bytes':
			return 'binary';
	}
}
