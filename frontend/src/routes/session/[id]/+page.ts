import { error } from '@sveltejs/kit';
import { listSessions } from '$lib/api';

export async function load({ params }: { params: { id: string } }) {
	const sessions = await listSessions();
	const session = sessions.find((s) => s.id === params.id);
	if (!session) throw error(404, 'Session not found');
	return { session };
}
