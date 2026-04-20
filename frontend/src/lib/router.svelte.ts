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

function getHashView(): ViewId {
	if (typeof window === 'undefined') return 'command';
	const h = window.location.hash.replace('#', '') || 'command';
	return VALID_VIEWS.includes(h as ViewId) ? (h as ViewId) : 'command';
}

if (typeof window !== 'undefined') {
	window.addEventListener('hashchange', () => {
		currentHash = getHashView();
	});
}

export const currentView = {
	get value() {
		return currentHash;
	},
	subscribe(fn: (v: ViewId) => void) {
		// Svelte store contract - call immediately and on changes
		fn(currentHash);
		return () => {}; // Cleanup
	},
	navigate(view: ViewId) {
		window.location.hash = view;
	}
};
