import type { ViewId } from './types';

const VALID_VIEWS: ViewId[] = [
	'command',
	'sessions',
	'session-detail',
	'pipeline',
	'conductor',
	'fleet',
	'search',
	'costs',
	'voice',
	'settings'
];

let currentHash: ViewId = $state(getHashView());
const subscribers = new Set<(v: ViewId) => void>();

function getHashView(): ViewId {
	if (typeof window === 'undefined') return 'command';
	const h = window.location.hash.replace('#', '') || 'command';
	return VALID_VIEWS.includes(h as ViewId) ? (h as ViewId) : 'command';
}

function notifySubscribers() {
	subscribers.forEach((fn) => fn(currentHash));
}

if (typeof window !== 'undefined') {
	window.addEventListener('hashchange', () => {
		currentHash = getHashView();
		notifySubscribers();
	});
}

export const currentView = {
	get value() {
		return currentHash;
	},
	subscribe(fn: (v: ViewId) => void) {
		// Svelte store contract - call immediately and on changes
		fn(currentHash);
		subscribers.add(fn);
		return () => {
			subscribers.delete(fn);
		};
	},
	navigate(view: ViewId) {
		window.location.hash = view;
	}
};
