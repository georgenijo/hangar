import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import SessionsView from './SessionsView.svelte';
import { sessionsStore } from '$lib/stores/sessions.svelte';
import type { Session } from '$lib/types';

// Mock the currentView router
vi.mock('$lib/router.svelte', () => ({
	currentView: {
		navigate: vi.fn()
	}
}));

// Import after mock
import { currentView } from '$lib/router.svelte';

describe('SessionsView', () => {
	// Sample sessions for testing
	const mockSessions: Session[] = [
		{
			id: 'session-1',
			slug: 'test-session-1',
			node_id: 'host-1',
			kind: { type: 'claude_code' },
			state: 'streaming',
			cwd: '/test',
			env: {},
			agent_meta: {
				name: 'claude-code',
				version: '1.0',
				model: 'opus-4',
				tokens_used: 5000,
				last_tool_call: 'Read'
			},
			labels: {},
			created_at: Date.now() - 60000,
			last_activity_at: Date.now() - 1000
		},
		{
			id: 'session-2',
			slug: 'test-session-2',
			node_id: 'host-2',
			kind: { type: 'shell' },
			state: 'idle',
			cwd: '/test',
			env: {},
			agent_meta: null,
			labels: {},
			created_at: Date.now() - 120000,
			last_activity_at: Date.now() - 30000
		},
		{
			id: 'session-3',
			slug: 'test-session-3',
			node_id: 'host-1',
			kind: { type: 'claude_code' },
			state: 'exited',
			cwd: '/test',
			env: {},
			agent_meta: null,
			labels: {},
			created_at: Date.now() - 180000,
			last_activity_at: Date.now() - 60000,
			exit: { code: 0, signal: null, reason: 'completed' }
		}
	];

	beforeEach(() => {
		// Mock the sessionsStore
		vi.spyOn(sessionsStore, 'startPolling').mockImplementation(() => {});
		vi.spyOn(sessionsStore, 'stopPolling').mockImplementation(() => {});

		// Mock the filtered sessions getter
		Object.defineProperty(sessionsStore, 'filteredSessions', {
			get: () => mockSessions,
			configurable: true
		});
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('renders with correct id', () => {
		const { container } = render(SessionsView);
		const viewElement = container.querySelector('#view-sessions');
		expect(viewElement).toBeTruthy();
		expect(viewElement?.classList.contains('view')).toBe(true);
	});

	it('renders filter chips', () => {
		const { container } = render(SessionsView);
		const filterChips = container.querySelectorAll('.filter-chip');
		expect(filterChips.length).toBe(4);

		const chipTexts = Array.from(filterChips).map((chip) => chip.textContent?.trim());
		expect(chipTexts).toEqual(['All', 'Active', 'Idle', 'Exited']);
	});

	it('has "All" filter active by default', () => {
		const { container } = render(SessionsView);
		const filterChips = container.querySelectorAll('.filter-chip');
		expect(filterChips[0].classList.contains('active')).toBe(true);
		expect(filterChips[1].classList.contains('active')).toBe(false);
	});

	it('toggles filter active state on click', async () => {
		const { container } = render(SessionsView);
		const filterChips = container.querySelectorAll('.filter-chip');

		// Click on "Active" filter
		await fireEvent.click(filterChips[1]);

		expect(filterChips[0].classList.contains('active')).toBe(false);
		expect(filterChips[1].classList.contains('active')).toBe(true);
	});

	it('renders DataTable with session rows', () => {
		const { container } = render(SessionsView);
		const dataTable = container.querySelector('.data-table');
		expect(dataTable).toBeTruthy();

		const rows = container.querySelectorAll('tbody tr');
		expect(rows.length).toBe(3);
	});

	it('displays correct columns in DataTable', () => {
		const { container } = render(SessionsView);
		const headers = container.querySelectorAll('thead th');

		const headerTexts = Array.from(headers).map((h) => h.textContent?.replace(/[↑↓]/g, '').trim());
		expect(headerTexts).toEqual(['Status', 'Slug', 'Agent', 'Host', 'ctx%', 'Tokens', 'Cost']);
	});

	it('filters sessions when Active filter is clicked', async () => {
		const { container } = render(SessionsView);
		const filterChips = container.querySelectorAll('.filter-chip');

		// Click "Active" filter
		await fireEvent.click(filterChips[1]);

		const rows = container.querySelectorAll('tbody tr');
		// Should only show streaming sessions
		expect(rows.length).toBe(1);
	});

	it('filters sessions when Idle filter is clicked', async () => {
		const { container } = render(SessionsView);
		const filterChips = container.querySelectorAll('.filter-chip');

		// Click "Idle" filter
		await fireEvent.click(filterChips[2]);

		const rows = container.querySelectorAll('tbody tr');
		// Should show idle and awaiting sessions
		expect(rows.length).toBe(1);
	});

	it('filters sessions when Exited filter is clicked', async () => {
		const { container } = render(SessionsView);
		const filterChips = container.querySelectorAll('.filter-chip');

		// Click "Exited" filter
		await fireEvent.click(filterChips[3]);

		const rows = container.querySelectorAll('tbody tr');
		// Should only show exited sessions
		expect(rows.length).toBe(1);
	});

	it('shows all sessions when All filter is clicked', async () => {
		const { container } = render(SessionsView);
		const filterChips = container.querySelectorAll('.filter-chip');

		// First click a different filter
		await fireEvent.click(filterChips[1]);

		// Then click "All"
		await fireEvent.click(filterChips[0]);

		const rows = container.querySelectorAll('tbody tr');
		// Should show all sessions
		expect(rows.length).toBe(3);
	});

	it('calls currentView.navigate on row click', async () => {
		const { container } = render(SessionsView);

		const firstRow = container.querySelector('tbody tr');
		expect(firstRow).toBeTruthy();

		await fireEvent.click(firstRow!);

		expect(currentView.navigate).toHaveBeenCalledWith('session-detail');
	});

	it('stores session ID in sessionStorage on row click', async () => {
		const { container } = render(SessionsView);

		const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

		const firstRow = container.querySelector('tbody tr');
		await fireEvent.click(firstRow!);

		expect(setItemSpy).toHaveBeenCalledWith('selectedSessionId', 'session-1');

		setItemSpy.mockRestore();
	});

	it('starts polling on mount', () => {
		render(SessionsView);
		expect(sessionsStore.startPolling).toHaveBeenCalled();
	});

	it('stops polling on unmount', () => {
		const { unmount } = render(SessionsView);
		unmount();
		expect(sessionsStore.stopPolling).toHaveBeenCalled();
	});
});
