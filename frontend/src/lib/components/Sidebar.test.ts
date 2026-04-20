import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Sidebar from './Sidebar.svelte';

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
			navigate: vi.fn((view: string) => {
				mockCurrentView = view;
				subscribers.forEach((fn) => fn(mockCurrentView));
			}),
			// Helper for tests to set the current view
			_setMockView(view: string) {
				mockCurrentView = view;
				subscribers.forEach((fn) => fn(mockCurrentView));
			}
		}
	};
});

describe('Sidebar', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('renders sidebar with class "sidebar"', () => {
		const { container } = render(Sidebar);
		const sidebar = container.querySelector('.sidebar');
		expect(sidebar).toBeTruthy();
	});

	it('renders 9 top-level navigation items', () => {
		const { container } = render(Sidebar);
		const navLinks = container.querySelectorAll('.nav-link');
		expect(navLinks.length).toBe(9); // 9 top-level nav items (session-detail is not in sidebar)
	});

	it('renders all navigation items with correct labels', () => {
		render(Sidebar);

		// Verify all required labels are present
		expect(screen.getByText('Command')).toBeTruthy();
		expect(screen.getByText('Sessions')).toBeTruthy();
		expect(screen.getByText('Pipelines')).toBeTruthy();
		expect(screen.getByText('Conductor')).toBeTruthy();
		expect(screen.getByText('Fleet')).toBeTruthy();
		expect(screen.getByText('Search')).toBeTruthy();
		expect(screen.getByText('Costs')).toBeTruthy();
		expect(screen.getByText('Voice')).toBeTruthy();
		expect(screen.getByText('Settings')).toBeTruthy();
	});

	it('renders all navigation items with icons', () => {
		const { container } = render(Sidebar);
		const icons = container.querySelectorAll('.icon');
		expect(icons.length).toBe(9);

		// Verify some icons are present
		const iconTexts = Array.from(icons).map((icon) => icon.textContent);
		expect(iconTexts).toContain('◎'); // Command
		expect(iconTexts).toContain('⬡'); // Sessions
		expect(iconTexts).toContain('$'); // Costs
	});

	it('applies active class to current view', () => {
		const { container } = render(Sidebar);
		const commandLink = container.querySelector('.nav-link');

		// Command should be active by default (mocked as 'command')
		expect(commandLink?.classList.contains('active')).toBe(true);
	});

	it('calls navigate when nav item is clicked', async () => {
		const { currentView } = await import('$lib/router.svelte');
		const { container } = render(Sidebar);

		const sessionsLink = Array.from(container.querySelectorAll('.nav-link')).find((link) =>
			link.textContent?.includes('Sessions')
		);

		expect(sessionsLink).toBeTruthy();

		// Click the sessions link
		sessionsLink?.dispatchEvent(new MouseEvent('click', { bubbles: true }));

		expect(currentView.navigate).toHaveBeenCalledWith('sessions');
	});

	it('updates active state when currentView changes', async () => {
		const { currentView } = await import('$lib/router.svelte');
		const { container } = render(Sidebar);

		// Initially, command should be active
		let commandLink = Array.from(container.querySelectorAll('.nav-link')).find((link) =>
			link.textContent?.includes('Command')
		) as HTMLElement;
		let sessionsLink = Array.from(container.querySelectorAll('.nav-link')).find((link) =>
			link.textContent?.includes('Sessions')
		) as HTMLElement;

		expect(commandLink.classList.contains('active')).toBe(true);
		expect(sessionsLink.classList.contains('active')).toBe(false);

		// Change the view
		(currentView as any)._setMockView('sessions');

		// Re-query the elements after state change
		commandLink = Array.from(container.querySelectorAll('.nav-link')).find((link) =>
			link.textContent?.includes('Command')
		) as HTMLElement;
		sessionsLink = Array.from(container.querySelectorAll('.nav-link')).find((link) =>
			link.textContent?.includes('Sessions')
		) as HTMLElement;

		// Now sessions should be active
		expect(commandLink.classList.contains('active')).toBe(false);
		expect(sessionsLink.classList.contains('active')).toBe(true);
	});

	it('renders brand with "Hangar" wordmark', () => {
		render(Sidebar);
		expect(screen.getByText('Hangar')).toBeTruthy();
	});

	it('renders user footer with avatar and name', () => {
		render(Sidebar);
		expect(screen.getByText('george')).toBeTruthy();
		expect(screen.getByText('G')).toBeTruthy(); // Avatar initial
	});

	it('nav items are buttons with type="button"', () => {
		const { container } = render(Sidebar);
		const navButtons = container.querySelectorAll('.nav-link');

		navButtons.forEach((button) => {
			expect(button.tagName).toBe('BUTTON');
			expect(button.getAttribute('type')).toBe('button');
		});
	});
});
