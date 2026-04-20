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

	it('subscribe calls callback immediately with current value', async () => {
		window.location.hash = '#pipeline';
		vi.resetModules();
		const { currentView } = await import('./router.svelte');

		const callback = vi.fn();
		currentView.subscribe(callback);

		expect(callback).toHaveBeenCalledTimes(1);
		expect(callback).toHaveBeenCalledWith('pipeline');
	});

	it('subscribe notifies on hashchange', async () => {
		window.location.hash = '#command';
		vi.resetModules();
		const { currentView } = await import('./router.svelte');

		const callback = vi.fn();
		const unsubscribe = currentView.subscribe(callback);

		// Clear the initial call
		callback.mockClear();

		// Simulate hashchange event
		window.location.hash = '#costs';
		window.dispatchEvent(new Event('hashchange'));

		expect(callback).toHaveBeenCalledTimes(1);
		expect(callback).toHaveBeenCalledWith('costs');

		unsubscribe();
	});

	it('unsubscribe stops receiving notifications', async () => {
		window.location.hash = '#command';
		vi.resetModules();
		const { currentView } = await import('./router.svelte');

		const callback = vi.fn();
		const unsubscribe = currentView.subscribe(callback);

		callback.mockClear();
		unsubscribe();

		// Change hash after unsubscribe
		window.location.hash = '#sessions';
		window.dispatchEvent(new Event('hashchange'));

		expect(callback).not.toHaveBeenCalled();
	});

	it('multiple subscribers all receive notifications', async () => {
		window.location.hash = '#command';
		vi.resetModules();
		const { currentView } = await import('./router.svelte');

		const callback1 = vi.fn();
		const callback2 = vi.fn();
		const callback3 = vi.fn();

		currentView.subscribe(callback1);
		currentView.subscribe(callback2);
		currentView.subscribe(callback3);

		callback1.mockClear();
		callback2.mockClear();
		callback3.mockClear();

		// Change hash
		window.location.hash = '#fleet';
		window.dispatchEvent(new Event('hashchange'));

		expect(callback1).toHaveBeenCalledWith('fleet');
		expect(callback2).toHaveBeenCalledWith('fleet');
		expect(callback3).toHaveBeenCalledWith('fleet');
	});
});
