import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

describe('router', () => {
	let originalHash: string;
	let originalLocation: Location;

	beforeEach(() => {
		// Save original state
		originalHash = window.location.hash;
		originalLocation = window.location;

		// Mock window.location
		delete (window as any).location;
		window.location = {
			...originalLocation,
			hash: ''
		} as Location;
	});

	afterEach(() => {
		// Restore original state
		window.location = originalLocation;
		window.location.hash = originalHash;
		// Clear any modules that might have cached the old state
		vi.resetModules();
	});

	it('getHashView returns "command" for empty hash', async () => {
		window.location.hash = '';
		const { currentView } = await import('./router.svelte');
		expect(currentView.value).toBe('command');
	});

	it('getHashView returns "command" for missing #', async () => {
		window.location.hash = '';
		const { currentView } = await import('./router.svelte');
		expect(currentView.value).toBe('command');
	});

	it('getHashView returns valid view for valid hash', async () => {
		window.location.hash = '#sessions';
		vi.resetModules();
		const { currentView } = await import('./router.svelte');
		expect(currentView.value).toBe('sessions');
	});

	it('getHashView validates against VALID_VIEWS', async () => {
		window.location.hash = '#invalid-view';
		vi.resetModules();
		const { currentView } = await import('./router.svelte');
		expect(currentView.value).toBe('command');
	});

	it('navigate sets window.location.hash', async () => {
		window.location.hash = '';
		vi.resetModules();
		const { currentView } = await import('./router.svelte');

		currentView.navigate('pipeline');
		expect(window.location.hash).toBe('#pipeline');
	});

	it('validates all VALID_VIEWS members', async () => {
		const validViews = [
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

		for (const view of validViews) {
			window.location.hash = `#${view}`;
			vi.resetModules();
			const { currentView } = await import('./router.svelte');
			expect(currentView.value).toBe(view);
		}
	});

	it('handles hash with extra characters', async () => {
		window.location.hash = '#sessions?id=123';
		vi.resetModules();
		const { currentView } = await import('./router.svelte');
		// Should treat the whole string as invalid and return 'command'
		expect(currentView.value).toBe('command');
	});

	it('handles hash without # prefix in navigate', async () => {
		window.location.hash = '';
		vi.resetModules();
		const { currentView } = await import('./router.svelte');

		currentView.navigate('costs');
		// Navigate adds the view to hash, browser adds #
		expect(window.location.hash).toBe('#costs');
	});

	it('currentView.value getter returns current hash', async () => {
		window.location.hash = '#fleet';
		vi.resetModules();
		const { currentView } = await import('./router.svelte');
		expect(currentView.value).toBe('fleet');
	});
});
