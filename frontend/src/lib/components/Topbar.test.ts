import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Topbar from './Topbar.svelte';

// Mock the router module
vi.mock('$lib/router.svelte', () => {
	let mockCurrentView = 'command';
	const subscribers = new Set<(v: string) => void>();

	return {
		currentView: {
			get value() {
				return mockCurrentView;
			},
			subscribe(fn: (v: string) => void) {
				fn(mockCurrentView);
				subscribers.add(fn);
				return () => {
					subscribers.delete(fn);
				};
			},
			navigate: vi.fn(),
			// Helper for tests to set the current view
			_setMockView(view: string) {
				mockCurrentView = view;
				subscribers.forEach((fn) => fn(mockCurrentView));
			}
		}
	};
});

describe('Topbar', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		// Reset to default view
		vi.resetModules();
	});

	it('renders topbar with class "topbar"', () => {
		const { container } = render(Topbar);
		const topbar = container.querySelector('.topbar');
		expect(topbar).toBeTruthy();
	});

	it('renders breadcrumb with current view title', () => {
		render(Topbar);
		// Default mocked view is 'command', which should display as 'Command Center'
		expect(screen.getByText('Command Center')).toBeTruthy();
	});

	it('updates breadcrumb when currentView changes', async () => {
		const { currentView } = await import('$lib/router.svelte');
		const { container } = render(Topbar);

		// Initially shows Command Center
		expect(screen.getByText('Command Center')).toBeTruthy();

		// Change view to sessions
		(currentView as any)._setMockView('sessions');

		// Should now show Sessions
		expect(screen.getByText('Sessions')).toBeTruthy();
	});

	it('displays correct title for all views', async () => {
		const { currentView } = await import('$lib/router.svelte');

		const viewTitles = [
			{ view: 'command', title: 'Command Center' },
			{ view: 'sessions', title: 'Sessions' },
			{ view: 'session-detail', title: 'Workspace' },
			{ view: 'pipeline', title: 'Pipelines' },
			{ view: 'conductor', title: 'Conductor' },
			{ view: 'fleet', title: 'Fleet' },
			{ view: 'search', title: 'Search' },
			{ view: 'costs', title: 'Costs' },
			{ view: 'voice', title: 'Voice' },
			{ view: 'settings', title: 'Settings' }
		];

		for (const { view, title } of viewTitles) {
			(currentView as any)._setMockView(view);
			const { container } = render(Topbar);
			expect(screen.getByText(title)).toBeTruthy();
			container.remove();
		}
	});

	it('uses custom title prop when provided', () => {
		render(Topbar, { props: { title: 'Custom Title' } });
		expect(screen.getByText('Custom Title')).toBeTruthy();
	});

	it('custom title overrides default breadcrumb', () => {
		const { container } = render(Topbar, { props: { title: 'Override Title' } });
		expect(screen.getByText('Override Title')).toBeTruthy();
		// Should not show the default view title
		expect(container.textContent).not.toContain('Command Center');
	});

	it('renders subtitle when provided', () => {
		render(Topbar, { props: { subtitle: 'This is a subtitle' } });
		expect(screen.getByText('This is a subtitle')).toBeTruthy();
	});

	it('does not render subtitle when not provided', () => {
		const { container } = render(Topbar);
		const subtitle = container.querySelector('.subtitle');
		expect(subtitle).toBeFalsy();
	});

	it('renders search input', () => {
		const { container } = render(Topbar);
		const searchInput = container.querySelector('input[type="text"]');
		expect(searchInput).toBeTruthy();
		expect(searchInput?.getAttribute('placeholder')).toBe('Search...');
	});

	it('renders topbar actions section', () => {
		const { container } = render(Topbar);
		const actions = container.querySelector('.topbar-actions');
		expect(actions).toBeTruthy();
	});

	it('renders action button in topbar', () => {
		const { container } = render(Topbar);
		const actionButton = container.querySelector('.btn');
		expect(actionButton).toBeTruthy();
	});

	it('renders breadcrumb in topbar-left section', () => {
		const { container } = render(Topbar);
		const topbarLeft = container.querySelector('.topbar-left');
		expect(topbarLeft).toBeTruthy();

		const crumb = topbarLeft?.querySelector('.crumb');
		expect(crumb).toBeTruthy();
	});

	it('fallback to "Hangar" for unknown view', async () => {
		const { currentView } = await import('$lib/router.svelte');
		(currentView as any)._setMockView('unknown-view' as any);

		render(Topbar);
		expect(screen.getByText('Hangar')).toBeTruthy();
	});
});
