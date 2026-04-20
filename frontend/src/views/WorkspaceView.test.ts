import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import WorkspaceView from './WorkspaceView.svelte';
import type { Session } from '$lib/types';

// Mock the API module
vi.mock('$lib/api', () => ({
	listSessions: vi.fn(),
	getSessionEvents: vi.fn()
}));

// Import mocked functions
import { listSessions, getSessionEvents } from '$lib/api';

// Mock session data
const mockSession: Session = {
	id: 'test-session-123',
	slug: 'test-workspace',
	node_id: 'localhost',
	kind: { type: 'claude_code' },
	state: 'streaming',
	cwd: '/workspace/test',
	env: {},
	agent_meta: {
		name: 'claude-code',
		version: '1.0.0',
		model: 'claude-opus-4-7',
		tokens_used: 50000,
		last_tool_call: 'Read'
	},
	labels: {},
	created_at: Date.now() - 600000, // 10 minutes ago
	last_activity_at: Date.now() - 30000 // 30 seconds ago
};

describe('WorkspaceView', () => {
	beforeEach(() => {
		// Clear sessionStorage before each test
		sessionStorage.clear();
		// Reset all mocks
		vi.clearAllMocks();
	});

	it('renders with id="view-session-detail"', async () => {
		sessionStorage.setItem('selectedSessionId', mockSession.id);
		vi.mocked(listSessions).mockResolvedValue([mockSession]);
		vi.mocked(getSessionEvents).mockResolvedValue([]);

		const { container } = render(WorkspaceView);

		// Check that the view has the correct id
		const viewElement = container.querySelector('#view-session-detail');
		expect(viewElement).toBeTruthy();
	});

	it('retrieves session ID from sessionStorage on mount', async () => {
		const sessionId = 'test-session-456';
		sessionStorage.setItem('selectedSessionId', sessionId);

		const mockSessionData = { ...mockSession, id: sessionId };
		vi.mocked(listSessions).mockResolvedValue([mockSessionData]);
		vi.mocked(getSessionEvents).mockResolvedValue([]);

		render(WorkspaceView);

		// Wait for async mount to complete
		await vi.waitFor(() => {
			expect(listSessions).toHaveBeenCalled();
		});

		// Verify that listSessions was called to find the session
		expect(listSessions).toHaveBeenCalledTimes(1);
	});

	it('displays error when session ID is missing from sessionStorage', async () => {
		// Don't set any session ID in sessionStorage
		render(WorkspaceView);

		// Wait for the error message to appear
		await vi.waitFor(() => {
			const errorMessage = screen.getByText(/No session ID found/i);
			expect(errorMessage).toBeTruthy();
		});
	});

	it('displays error when session is not found', async () => {
		sessionStorage.setItem('selectedSessionId', 'non-existent-session');
		vi.mocked(listSessions).mockResolvedValue([mockSession]); // Different session

		render(WorkspaceView);

		// Wait for the error message to appear
		await vi.waitFor(() => {
			const errorMessage = screen.getByText(/Session non-existent-session not found/i);
			expect(errorMessage).toBeTruthy();
		});
	});

	it('renders TerminalView component when session is loaded', async () => {
		sessionStorage.setItem('selectedSessionId', mockSession.id);
		vi.mocked(listSessions).mockResolvedValue([mockSession]);
		vi.mocked(getSessionEvents).mockResolvedValue([]);

		const { container } = render(WorkspaceView);

		// Wait for session to load
		await vi.waitFor(() => {
			const terminalPane = container.querySelector('.terminal-pane');
			expect(terminalPane).toBeTruthy();
		});
	});

	it('renders sidebar sections (context %, cost, tool mix, files)', async () => {
		sessionStorage.setItem('selectedSessionId', mockSession.id);
		vi.mocked(listSessions).mockResolvedValue([mockSession]);
		vi.mocked(getSessionEvents).mockResolvedValue([]);

		const { container } = render(WorkspaceView);

		// Wait for session to load
		await vi.waitFor(() => {
			const sidebar = container.querySelector('.workspace-sidebar');
			expect(sidebar).toBeTruthy();
		});

		// Check for sidebar section headings
		const headings = Array.from(container.querySelectorAll('.sidebar-heading')).map(
			(el) => el.textContent
		);

		expect(headings).toContain('Context Window');
		expect(headings).toContain('Session Cost');
		expect(headings).toContain('Output Rate');
		expect(headings).toContain('Tool Mix');
		expect(headings).toContain('Files Touched');
	});

	it('displays session metadata in header', async () => {
		sessionStorage.setItem('selectedSessionId', mockSession.id);
		vi.mocked(listSessions).mockResolvedValue([mockSession]);
		vi.mocked(getSessionEvents).mockResolvedValue([]);

		render(WorkspaceView);

		// Wait for session to load and check for session title
		await vi.waitFor(() => {
			const title = screen.getByText(mockSession.slug);
			expect(title).toBeTruthy();
		});

		// Check for session metadata
		const hostBadge = screen.getByText(mockSession.node_id);
		expect(hostBadge).toBeTruthy();

		const stateBadge = screen.getByText(mockSession.state);
		expect(stateBadge).toBeTruthy();
	});

	it('updates metrics from events', async () => {
		sessionStorage.setItem('selectedSessionId', mockSession.id);
		vi.mocked(listSessions).mockResolvedValue([mockSession]);

		// Mock events with context and cost updates
		const mockEvents = [
			{
				id: 1,
				session_id: mockSession.id,
				ts: Date.now(),
				kind: 'agent_event',
				event: {
					type: 'agent_event' as const,
					id: 'evt-1',
					event: {
						type: 'context_window_size_changed' as const,
						pct_used: 0.42,
						tokens: 42000
					}
				}
			},
			{
				id: 2,
				session_id: mockSession.id,
				ts: Date.now(),
				kind: 'agent_event',
				event: {
					type: 'agent_event' as const,
					id: 'evt-2',
					event: {
						type: 'cost_updated' as const,
						dollars: 5.67
					}
				}
			},
			{
				id: 3,
				session_id: mockSession.id,
				ts: Date.now(),
				kind: 'agent_event',
				event: {
					type: 'agent_event' as const,
					id: 'evt-3',
					event: {
						type: 'tool_call_started' as const,
						turn_id: 1,
						call_id: 'call-1',
						tool: 'Read',
						args_preview: 'file.ts'
					}
				}
			},
			{
				id: 4,
				session_id: mockSession.id,
				ts: Date.now(),
				kind: 'agent_event',
				event: {
					type: 'agent_event' as const,
					id: 'evt-4',
					event: {
						type: 'tool_call_started' as const,
						turn_id: 1,
						call_id: 'call-2',
						tool: 'Write',
						args_preview: 'file.ts'
					}
				}
			}
		];

		vi.mocked(getSessionEvents).mockResolvedValue(mockEvents);

		const { container } = render(WorkspaceView);

		// Wait for metrics to be processed
		await vi.waitFor(() => {
			// Check that cost is displayed
			const costElement = container.querySelector('.cost-value');
			expect(costElement?.textContent).toContain('$5.67');
		});

		// Check for tool mix
		await vi.waitFor(() => {
			const toolNames = Array.from(container.querySelectorAll('.tool-name')).map(
				(el) => el.textContent
			);
			expect(toolNames).toContain('Read');
			expect(toolNames).toContain('Write');
		});
	});

	it('shows loading state initially', () => {
		sessionStorage.setItem('selectedSessionId', mockSession.id);
		vi.mocked(listSessions).mockImplementation(
			() => new Promise((resolve) => setTimeout(() => resolve([mockSession]), 100))
		);

		render(WorkspaceView);

		// Check for loading message
		const loadingMessage = screen.getByText(/Loading session/i);
		expect(loadingMessage).toBeTruthy();
	});

	it('provides back to sessions link when error occurs', async () => {
		// Don't set session ID to trigger error
		render(WorkspaceView);

		await vi.waitFor(() => {
			const backLink = screen.getByText(/Back to Sessions/i);
			expect(backLink).toBeTruthy();
			expect(backLink.getAttribute('href')).toBe('#sessions');
		});
	});
});
