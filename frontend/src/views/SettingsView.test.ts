import { describe, it, expect } from 'vitest';
import { render, fireEvent, waitFor } from '@testing-library/svelte';
import SettingsView from './SettingsView.svelte';

describe('SettingsView', () => {
	it('renders with correct id', () => {
		const { container } = render(SettingsView);
		const viewElement = container.querySelector('#view-settings');
		expect(viewElement).toBeTruthy();
		expect(viewElement?.classList.contains('view')).toBe(true);
	});

	it('displays settings layout with nav and body', () => {
		const { container } = render(SettingsView);
		const settingsLayout = container.querySelector('.settings-layout');
		expect(settingsLayout).toBeTruthy();

		const nav = container.querySelector('.settings-nav');
		const body = container.querySelector('.settings-body');
		expect(nav).toBeTruthy();
		expect(body).toBeTruthy();
	});

	it('displays exactly 9 navigation links', () => {
		const { container } = render(SettingsView);
		const navLinks = container.querySelectorAll('.settings-link');
		expect(navLinks.length).toBe(9);
	});

	it('displays all required navigation items', () => {
		const { container } = render(SettingsView);
		const navLinks = container.querySelectorAll('.settings-link');
		const navText = Array.from(navLinks).map(link => link.textContent?.trim());

		expect(navText).toContain('General');
		expect(navText).toContain('Models');
		expect(navText).toContain('Fleet');
		expect(navText).toContain('Dispatch');
		expect(navText).toContain('Sandbox');
		expect(navText).toContain('Notifications');
		expect(navText).toContain('MCP');
		expect(navText).toContain('API Keys');
		expect(navText).toContain('Danger Zone');
	});

	it('highlights active navigation section', () => {
		const { container } = render(SettingsView);
		const activeLink = container.querySelector('.settings-link.settings-active');
		expect(activeLink).toBeTruthy();
	});

	it('displays general section by default', () => {
		const { container } = render(SettingsView);
		const activeLink = container.querySelector('.settings-link.settings-active');
		expect(activeLink?.textContent).toContain('General');
	});

	it('displays model selection radio grid', async () => {
		const { container } = render(SettingsView);
		// Click on Models nav item
		const navLinks = container.querySelectorAll('.settings-link');
		const modelsLink = Array.from(navLinks).find(link => link.textContent?.includes('Models'));
		if (modelsLink instanceof HTMLElement) {
			await fireEvent.click(modelsLink);
		}

		await waitFor(() => {
			const radioGrid = container.querySelector('.radio-grid');
			expect(radioGrid).toBeTruthy();

			const radioCards = container.querySelectorAll('.radio-card');
			expect(radioCards.length).toBe(3); // opus, sonnet, haiku
		});
	});

	it('displays range inputs for dispatch settings', async () => {
		const { container } = render(SettingsView);
		// Click on Dispatch nav item
		const navLinks = container.querySelectorAll('.settings-link');
		const dispatchLink = Array.from(navLinks).find(link => link.textContent?.includes('Dispatch'));
		if (dispatchLink instanceof HTMLElement) {
			await fireEvent.click(dispatchLink);
		}

		await waitFor(() => {
			const rangeRows = container.querySelectorAll('.range-row');
			expect(rangeRows.length).toBeGreaterThan(0);

			const rangeInputs = container.querySelectorAll('input[type="range"]');
			expect(rangeInputs.length).toBe(2); // soft and hard budgets
		});
	});

	it('displays toggle switches for notifications', async () => {
		const { container } = render(SettingsView);
		// Click on Notifications nav item
		const navLinks = container.querySelectorAll('.settings-link');
		const notifLink = Array.from(navLinks).find(link => link.textContent?.includes('Notifications'));
		if (notifLink instanceof HTMLElement) {
			await fireEvent.click(notifLink);
		}

		await waitFor(() => {
			const toggleList = container.querySelector('.toggle-list');
			expect(toggleList).toBeTruthy();

			const toggleRows = container.querySelectorAll('.toggle-row');
			expect(toggleRows.length).toBe(3); // complete, error, review

			const toggles = container.querySelectorAll('.toggle');
			expect(toggles.length).toBe(3);
		});
	});

	it('displays setting section titles and descriptions', () => {
		const { container } = render(SettingsView);
		const settingTitle = container.querySelector('.setting-title');
		const settingDesc = container.querySelector('.setting-desc');

		expect(settingTitle).toBeTruthy();
		expect(settingDesc).toBeTruthy();
	});

	it('displays danger zone section with destructive action', async () => {
		const { container } = render(SettingsView);
		// Click on Danger Zone nav item
		const navLinks = container.querySelectorAll('.settings-link');
		const dangerLink = Array.from(navLinks).find(link => link.textContent?.includes('Danger'));
		if (dangerLink instanceof HTMLElement) {
			await fireEvent.click(dangerLink);
		}

		await waitFor(() => {
			const dangerButton = container.querySelector('.btn-danger-ghost');
			expect(dangerButton).toBeTruthy();
			expect(dangerButton?.textContent).toContain('Clear all session data');
		});
	});

	it('displays select mock elements', () => {
		const { container } = render(SettingsView);
		const selectMock = container.querySelector('.select-mock');
		expect(selectMock).toBeTruthy();

		const chevron = container.querySelector('.chev');
		expect(chevron).toBeTruthy();
		expect(chevron?.textContent).toBe('▼');
	});
});
