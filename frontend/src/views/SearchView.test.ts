import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import SearchView from './SearchView.svelte';

describe('SearchView', () => {
	it('renders with id="view-search"', () => {
		const { container } = render(SearchView);
		const viewElement = container.querySelector('#view-search');
		expect(viewElement).toBeTruthy();
	});

	it('renders search input', () => {
		const { container } = render(SearchView);
		const input = container.querySelector('input.search-input');
		expect(input).toBeTruthy();
		expect(input?.getAttribute('placeholder')).toContain('Search');
	});

	it('renders filter chips', () => {
		const { container } = render(SearchView);
		const filterChips = container.querySelectorAll('.filter-chip');
		expect(filterChips.length).toBeGreaterThanOrEqual(5); // all, sessions, events, code, prs
	});

	it('renders "All" filter chip', () => {
		const { container } = render(SearchView);
		const allChip = Array.from(container.querySelectorAll('.filter-chip')).find(
			(el) => el.textContent?.trim() === 'All'
		);
		expect(allChip).toBeTruthy();
	});

	it('renders "Sessions" filter chip', () => {
		const { container } = render(SearchView);
		const sessionsChip = Array.from(container.querySelectorAll('.filter-chip')).find(
			(el) => el.textContent?.trim() === 'Sessions'
		);
		expect(sessionsChip).toBeTruthy();
	});

	it('renders "Events" filter chip', () => {
		const { container } = render(SearchView);
		const eventsChip = Array.from(container.querySelectorAll('.filter-chip')).find(
			(el) => el.textContent?.trim() === 'Events'
		);
		expect(eventsChip).toBeTruthy();
	});

	it('"All" filter chip is active by default', () => {
		const { container } = render(SearchView);
		const allChip = Array.from(container.querySelectorAll('.filter-chip')).find(
			(el) => el.textContent?.trim() === 'All'
		);
		expect(allChip?.classList.contains('active')).toBe(true);
	});
});
