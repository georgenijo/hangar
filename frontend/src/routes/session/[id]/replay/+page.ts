import { error } from '@sveltejs/kit';
import { listSessions, getSessionEvents } from '$lib/api';

export async function load({ params }: { params: { id: string } }) {
	const sessions = await listSessions();
	const session = sessions.find((s) => s.id === params.id);
	if (!session) throw error(404, 'Session not found');

	// Fetch all events for replay — limit raised to 50K server-side
	const events = await getSessionEvents(params.id, { limit: 50000 });

	return { session, events };
}
