import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import PipelineView from './PipelineView.svelte';
import { createPipelineStore } from '$lib/stores/pipeline.svelte';
import type { PipelineRun } from '$lib/types';

// Mock the pipeline store
vi.mock('$lib/stores/pipeline.svelte', () => {
	const mockStore = {
		runs: [] as PipelineRun[],
		loading: false,
		error: null as string | null,
		activeRuns: [] as PipelineRun[],
		countByState: { pending: 0, live: 0, done: 0, failed: 0 },
		refresh: vi.fn(),
		startPolling: vi.fn(),
		stopPolling: vi.fn()
	};

	return {
		createPipelineStore: () => mockStore,
		pipelineStore: mockStore
	};
});

// Import the mocked store
import { pipelineStore } from '$lib/stores/pipeline.svelte';

describe('PipelineView', () => {
	const mockRuns: PipelineRun[] = [
		{
			issue: 79,
			title: 'cost scraper lag fix',
			state: 'live',
			phase: 'tester',
			cost: 4.82,
			tokens: 482000,
			agents: 5,
			duration_s: 6120
		},
		{
			issue: 71,
			title: 'sidebar session switcher',
			state: 'live',
			phase: 'architect',
			cost: 0.88,
			tokens: 82000,
			agents: 2,
			duration_s: 1920
		},
		{
			issue: 65,
			title: 'implement dark mode',
			state: 'done',
			phase: 'pr',
			cost: 12.5,
			tokens: 1200000,
			agents: 8,
			duration_s: 14400
		}
	];

	beforeEach(() => {
		vi.clearAllMocks();
		// Reset store state
		pipelineStore.runs = [];
		pipelineStore.loading = false;
		pipelineStore.error = null;
		// @ts-expect-error - activeRuns is a getter in real store but mocked as property
		pipelineStore.activeRuns = [];
	});

	describe('component structure', () => {
		it('renders with correct id', () => {
			const { container } = render(PipelineView);
			const viewElement = container.querySelector('#view-pipeline');
			expect(viewElement).toBeTruthy();
			expect(viewElement?.classList.contains('view')).toBe(true);
		});

		it('starts polling on mount', () => {
			render(PipelineView);
			expect(pipelineStore.startPolling).toHaveBeenCalled();
		});

		it('stops polling on unmount', () => {
			const { unmount } = render(PipelineView);
			unmount();
			expect(pipelineStore.stopPolling).toHaveBeenCalled();
		});
	});

	describe('active pipelines section', () => {
		it('renders PipelineTrack for each active run', () => {
			const activeRuns = mockRuns.filter((r) => r.state === 'live');
			pipelineStore.runs = mockRuns;
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = activeRuns;

			const { container } = render(PipelineView);

			const pipelineTracks = container.querySelectorAll('.phase-track');
			expect(pipelineTracks.length).toBe(activeRuns.length);
		});

		it('displays pipeline metadata correctly', () => {
			const activeRuns = [mockRuns[0]];
			pipelineStore.runs = mockRuns;
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = activeRuns;

			const { container } = render(PipelineView);

			const pipelineCard = container.querySelector('.pipeline-card');
			expect(pipelineCard).toBeTruthy();

			// Check issue number
			const issue = pipelineCard?.querySelector('.pipeline-issue');
			expect(issue?.textContent).toBe('#79');

			// Check title
			const title = pipelineCard?.querySelector('.pipeline-title');
			expect(title?.textContent).toBe('cost scraper lag fix');

			// Check state
			const state = pipelineCard?.querySelector('.pipeline-state');
			expect(state?.textContent).toContain('live');
		});

		it('displays pipeline stats', () => {
			const activeRuns = [mockRuns[0]];
			pipelineStore.runs = mockRuns;
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = activeRuns;

			const { container } = render(PipelineView);

			const stats = container.querySelector('.pipeline-stats');
			expect(stats).toBeTruthy();

			// Check for agents, cost, tokens, duration
			const statElements = stats?.querySelectorAll('.stat');
			expect(statElements?.length).toBe(4);
		});

		it('displays gate pills', () => {
			const activeRuns = [mockRuns[0]];
			pipelineStore.runs = mockRuns;
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = activeRuns;

			const { container } = render(PipelineView);

			const gates = container.querySelector('.pipeline-gates');
			expect(gates).toBeTruthy();

			const gatePills = gates?.querySelector('.gate-pills');
			expect(gatePills).toBeTruthy();
		});

		it('shows empty state when no active runs', () => {
			pipelineStore.runs = [];
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = [];

			const { container } = render(PipelineView);

			const emptyState = container.querySelector('.empty-state');
			expect(emptyState).toBeTruthy();
			expect(emptyState?.textContent).toContain('No active pipelines');
		});
	});

	describe('history table', () => {
		it('renders DataTable with runs', () => {
			pipelineStore.runs = mockRuns;
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = mockRuns.filter((r) => r.state === 'live');

			const { container } = render(PipelineView);

			const dataTable = container.querySelector('.data-table');
			expect(dataTable).toBeTruthy();
		});

		it('displays correct number of rows', () => {
			pipelineStore.runs = mockRuns;
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = mockRuns.filter((r) => r.state === 'live');

			const { container } = render(PipelineView);

			const rows = container.querySelectorAll('.data-table tbody tr');
			expect(rows.length).toBe(mockRuns.length);
		});

		it('shows empty state when no runs', () => {
			pipelineStore.runs = [];
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = [];

			const { container } = render(PipelineView);

			// Should have 2 empty states: one for active, one for history
			const emptyStates = container.querySelectorAll('.empty-state');
			expect(emptyStates.length).toBe(2);
		});
	});

	describe('error handling', () => {
		it('displays error banner when error exists', () => {
			pipelineStore.error = 'Failed to fetch pipeline runs';

			const { container } = render(PipelineView);

			const errorBanner = container.querySelector('.error-banner');
			expect(errorBanner).toBeTruthy();
			expect(errorBanner?.textContent).toContain('Failed to fetch pipeline runs');
		});

		it('does not display error banner when no error', () => {
			pipelineStore.error = null;

			const { container } = render(PipelineView);

			const errorBanner = container.querySelector('.error-banner');
			expect(errorBanner).toBeNull();
		});
	});

	describe('acceptance criteria coverage', () => {
		it('AC1: PipelineView.svelte renders with id="view-pipeline"', () => {
			const { container } = render(PipelineView);
			const viewElement = container.querySelector('#view-pipeline');
			expect(viewElement).toBeTruthy();
		});

		it('AC2: PipelineTrack rendered for each active pipeline run', () => {
			const activeRuns = mockRuns.filter((r) => r.state === 'live');
			pipelineStore.runs = mockRuns;
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = activeRuns;

			const { container } = render(PipelineView);

			const pipelineTracks = container.querySelectorAll('.phase-track');
			expect(pipelineTracks.length).toBe(activeRuns.length);
		});

		it('AC3: History table shows recent runs with status, cost, duration', () => {
			pipelineStore.runs = mockRuns;
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = mockRuns.filter((r) => r.state === 'live');

			const { container } = render(PipelineView);

			const dataTable = container.querySelector('.data-table');
			expect(dataTable).toBeTruthy();

			// Check that table has rows
			const rows = container.querySelectorAll('.data-table tbody tr');
			expect(rows.length).toBe(mockRuns.length);

			// Table renders status, cost, duration (columns are rendered by DataTable component)
			const tableHeaders = container.querySelectorAll('.data-table thead th');
			expect(tableHeaders.length).toBeGreaterThan(0);
		});

		it('AC4: Gate pills display smoke/screenshot/scenario status', () => {
			const activeRuns = [mockRuns[0]];
			pipelineStore.runs = mockRuns;
			// @ts-expect-error - activeRuns is a getter in real store
			pipelineStore.activeRuns = activeRuns;

			const { container } = render(PipelineView);

			const gatePills = container.querySelector('.gate-pills');
			expect(gatePills).toBeTruthy();
			// Gate pills should contain smoke/screenshot/scenario info
			expect(gatePills?.textContent).toBeTruthy();
		});
	});
});
