import { listLogSources } from '$lib/api';
import type { LogSource } from '$lib/types';

export const ssr = false;

export async function load(): Promise<{ sources: LogSource[] }> {
	try {
		const sources = await listLogSources();
		return { sources };
	} catch {
		return { sources: [] };
	}
}
