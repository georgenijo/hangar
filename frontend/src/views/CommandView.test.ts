import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import CommandView from './CommandView.svelte';
import * as sessionsStore from '$lib/stores/sessions.svelte';
import * as costsStoreModule from '$lib/stores/costs.svelte';
import * as pipelineStoreModule from '$lib/stores/pipeline.svelte';
import * as hostStoreModule from '$lib/stores/host.svelte';
import type { Session, DailyCost, PipelineRun, HostMetrics } from '$lib/types';

// Mock all store modules
vi.mock('$lib/stores/sessions.svelte');
vi.mock('$lib/stores/costs.svelte');
vi.mock('$lib/stores/pipeline.svelte');
vi.mock('$lib/stores/host.svelte');

describe('CommandView', () => {
	let mockSessionsStore: {
		sessions: Session[];
		startPolling: ReturnType<typeof vi.fn>;
		stopPolling: ReturnType<typeof vi.fn>;
	};

	let mockCostsStore: {
		dailyCosts: DailyCost[];
		last7DaysSpend: number;
		dailyAmounts: number[];
		startPolling: ReturnType<typeof vi.fn>;
		stopPolling: ReturnType<typeof vi.fn>;
	};

	let mockPipelineStore: {
		runs: PipelineRun[];
		activeRuns: PipelineRun[];
		startPolling: ReturnType<typeof vi.fn>;
		stopPolling: ReturnType<typeof vi.fn>;
	};

	let mockHostStore: {
		metrics: HostMetrics | null;
		cpuPct: number;
		ramPct: number;
		diskPct: number;
		startPolling: ReturnType<typeof vi.fn>;
		stopPolling: ReturnType<typeof vi.fn>;
	};

	beforeEach(() => {
		// Create fresh mock stores for each test
		mockSessionsStore = {
			sessions: [],
			startPolling: vi.fn(),
			stopPolling: vi.fn()
		};

		mockCostsStore = {
			dailyCosts: [],
			last7DaysSpend: 0,
			dailyAmounts: [],
			startPolling: vi.fn(),
			stopPolling: vi.fn()
		};

		mockPipelineStore = {
			runs: [],
			activeRuns: [],
			startPolling: vi.fn(),
			stopPolling: vi.fn()
		};

		mockHostStore = {
			metrics: null,
			cpuPct: 0,
			ramPct: 0,
			diskPct: 0,
			startPolling: vi.fn(),
			stopPolling: vi.fn()
		};

		// Mock the store exports
		vi.mocked(sessionsStore).sessionsStore = mockSessionsStore as any;
		vi.mocked(costsStoreModule).costsStore = mockCostsStore as any;
		vi.mocked(pipelineStoreModule).pipelineStore = mockPipelineStore as any;
		vi.mocked(hostStoreModule).hostStore = mockHostStore as any;
	});

	afterEach(() => {
		vi.clearAllMocks();
	});

	describe('rendering', () => {
		it('renders with id="view-command"', () => {
			const { container } = render(CommandView);
			const viewElement = container.querySelector('#view-command');
			expect(viewElement).toBeTruthy();
		});

		it('renders KPI grid', () => {
			const { container } = render(CommandView);
			const kpiGrid = container.querySelector('.kpi-grid');
			expect(kpiGrid).toBeTruthy();
		});

		it('renders 5 KPI cards', () => {
			const { container } = render(CommandView);
			const kpiCards = container.querySelectorAll('.kpi');
			expect(kpiCards.length).toBe(5);
		});
	});

	describe('KPI data from stores', () => {
		it('displays active sessions count from sessionsStore', () => {
			const mockSessions: Session[] = [
				{
					id: '1',
					slug: 'session-1',
					node_id: 'host-1',
					kind: { type: 'shell' },
					state: 'streaming',
					cwd: '/home',
					env: {},
					labels: {},
					created_at: Date.now(),
					last_activity_at: Date.now()
				},
				{
					id: '2',
					slug: 'session-2',
					node_id: 'host-1',
					kind: { type: 'shell' },
					state: 'idle',
					cwd: '/home',
					env: {},
					labels: {},
					created_at: Date.now(),
					last_activity_at: Date.now()
				}
			];

			mockSessionsStore.sessions = mockSessions;
			const { container } = render(CommandView);

			const kpiCards = container.querySelectorAll('.kpi');
			const activeSessionsCard = Array.from(kpiCards).find((card) =>
				card.textContent?.includes('Active Sessions')
			);

			expect(activeSessionsCard).toBeTruthy();
			expect(activeSessionsCard?.textContent).toContain('2');
		});

		it('displays 7d spend from costsStore', () => {
			mockCostsStore.last7DaysSpend = 42.5;
			mockCostsStore.dailyAmounts = [5, 6, 7, 8, 9, 10, 11];

			const { container } = render(CommandView);
			const kpiCards = container.querySelectorAll('.kpi');
			const spendCard = Array.from(kpiCards).find((card) => card.textContent?.includes('7d Spend'));

			expect(spendCard).toBeTruthy();
			expect(spendCard?.textContent).toContain('$42.50');
		});

		it('displays live pipelines count from pipelineStore', () => {
			const mockRuns: PipelineRun[] = [
				{
					issue: 1,
					title: 'Test',
					state: 'live',
					phase: 'tester',
					cost: 5.0,
					tokens: 1000,
					agents: 1,
					duration_s: 100
				}
			];

			mockPipelineStore.activeRuns = mockRuns;
			const { container } = render(CommandView);

			const kpiCards = container.querySelectorAll('.kpi');
			const pipelineCard = Array.from(kpiCards).find((card) =>
				card.textContent?.includes('Live Pipelines')
			);

			expect(pipelineCard).toBeTruthy();
			expect(pipelineCard?.textContent).toContain('1');
		});

		it('displays CPU percentage from hostStore', () => {
			mockHostStore.cpuPct = 45.8;
			const { container } = render(CommandView);

			const kpiCards = container.querySelectorAll('.kpi');
			const cpuCard = Array.from(kpiCards).find((card) => card.textContent?.includes('CPU'));

			expect(cpuCard).toBeTruthy();
			expect(cpuCard?.textContent).toContain('46%');
		});

		it('displays RAM percentage from hostStore', () => {
			mockHostStore.ramPct = 62.3;
			const { container } = render(CommandView);

			const kpiCards = container.querySelectorAll('.kpi');
			const ramCard = Array.from(kpiCards).find((card) => card.textContent?.includes('RAM'));

			expect(ramCard).toBeTruthy();
			expect(ramCard?.textContent).toContain('62%');
		});
	});

	describe('lifecycle', () => {
		it('calls startPolling on all stores when mounted', () => {
			render(CommandView);

			expect(mockSessionsStore.startPolling).toHaveBeenCalledOnce();
			expect(mockCostsStore.startPolling).toHaveBeenCalledOnce();
			expect(mockPipelineStore.startPolling).toHaveBeenCalledOnce();
			expect(mockHostStore.startPolling).toHaveBeenCalledOnce();
		});

		it('calls stopPolling on all stores when unmounted', () => {
			const { unmount } = render(CommandView);

			// Clear the calls from mount
			vi.clearAllMocks();

			unmount();

			expect(mockSessionsStore.stopPolling).toHaveBeenCalledOnce();
			expect(mockCostsStore.stopPolling).toHaveBeenCalledOnce();
			expect(mockPipelineStore.stopPolling).toHaveBeenCalledOnce();
			expect(mockHostStore.stopPolling).toHaveBeenCalledOnce();
		});
	});

	describe('alert states', () => {
		it('marks active sessions as alert when > 10', () => {
			const mockSessions: Session[] = Array.from({ length: 12 }, (_, i) => ({
				id: String(i),
				slug: `session-${i}`,
				node_id: 'host-1',
				kind: { type: 'shell' },
				state: 'streaming' as const,
				cwd: '/home',
				env: {},
				labels: {},
				created_at: Date.now(),
				last_activity_at: Date.now()
			}));

			mockSessionsStore.sessions = mockSessions;
			const { container } = render(CommandView);

			const kpiCards = container.querySelectorAll('.kpi');
			const activeSessionsCard = Array.from(kpiCards).find((card) =>
				card.textContent?.includes('Active Sessions')
			);

			expect(activeSessionsCard?.classList.contains('kpi-alert')).toBe(true);
		});

		it('marks CPU as alert when > 75%', () => {
			mockHostStore.cpuPct = 85;
			const { container } = render(CommandView);

			const kpiCards = container.querySelectorAll('.kpi');
			const cpuCard = Array.from(kpiCards).find((card) => card.textContent?.includes('CPU'));

			expect(cpuCard?.classList.contains('kpi-alert')).toBe(true);
		});

		it('marks RAM as alert when > 75%', () => {
			mockHostStore.ramPct = 90;
			const { container } = render(CommandView);

			const kpiCards = container.querySelectorAll('.kpi');
			const ramCard = Array.from(kpiCards).find((card) => card.textContent?.includes('RAM'));

			expect(ramCard?.classList.contains('kpi-alert')).toBe(true);
		});
	});
});
