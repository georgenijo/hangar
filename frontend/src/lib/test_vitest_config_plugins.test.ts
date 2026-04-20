/**
 * Integration Test: Vitest Config Plugin Compatibility
 * Priority: CRITICAL - Tests conflict resolution in vitest.config.ts
 *
 * CONFLICT RESOLVED: vitest.config.ts combined:
 * - svelte() plugin (all branches)
 * - svelteTesting() plugin (issue-21)
 *
 * This test verifies components from different branches render with both plugins.
 */
import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import VoiceView from '../views/VoiceView.svelte';
import SettingsView from '../views/SettingsView.svelte';
import KpiCard from './components/KpiCard.svelte';
import DataTable from './components/DataTable.svelte';
import Sidebar from './components/Sidebar.svelte';
import Topbar from './components/Topbar.svelte';
import type { KpiCardData, DataTableColumn, NavItem } from '$lib/types';

describe('Vitest Config: svelte() + svelteTesting() Plugin Integration', () => {
	it('renders VoiceView from issue-21 (requires svelteTesting plugin)', () => {
		const { container } = render(VoiceView);
		expect(container.textContent).toContain('Push to Talk');
		expect(container.textContent).toContain('Transcript');
		expect(container.querySelector('#view-voice')).not.toBeNull();
	});

	it('renders SettingsView from issue-21 (requires svelteTesting plugin)', () => {
		const { container } = render(SettingsView);
		expect(container.textContent).toContain('General');
		expect(container.textContent).toContain('Models');
		expect(container.querySelector('#view-settings')).not.toBeNull();
	});

	it('renders KpiCard from issue-20 (requires svelte plugin)', () => {
		const data: KpiCardData = { label: 'Test', value: '42' };
		const { container } = render(KpiCard, { props: { data } });
		expect(container.textContent).toContain('Test');
		expect(container.textContent).toContain('42');
	});

	it('renders DataTable from issue-20 (requires svelte plugin)', () => {
		const columns: DataTableColumn<{ id: number }>[] = [{ key: 'id', label: 'ID' }];
		const rows = [{ id: 1 }];
		const { container } = render(DataTable, { props: { columns, rows } });
		expect(container.querySelector('table.data-table')).not.toBeNull();
	});

	it('renders Sidebar from issue-19 (requires svelte plugin)', () => {
		const items: NavItem[] = [{ id: 'command', label: 'Command', icon: '⌘' }];
		const { container } = render(Sidebar, { props: { items, activeView: 'command' } });
		expect(container.textContent).toContain('Command');
	});

	it('renders Topbar from issue-19 (requires svelte plugin)', () => {
		const { container } = render(Topbar, { props: { title: 'Custom Title' } });
		expect(container.textContent).toContain('Custom Title');
		expect(container.querySelector('.topbar')).not.toBeNull();
	});

	it('all components render simultaneously without plugin conflicts', () => {
		// Critical test: components from different branches all work together
		const { container: c1 } = render(VoiceView);
		const { container: c2 } = render(SettingsView);
		const { container: c3 } = render(KpiCard, {
			props: { data: { label: 'CPU', value: '50%' } }
		});
		const { container: c4 } = render(DataTable, {
			props: { columns: [{ key: 'id', label: 'ID' }], rows: [] }
		});

		expect(c1.querySelector('#view-voice')).not.toBeNull();
		expect(c2.querySelector('#view-settings')).not.toBeNull();
		expect(c3.querySelector('.kpi')).not.toBeNull();
		expect(c4.querySelector('table.data-table')).not.toBeNull();
	});

	it('Svelte 5 runes work with combined plugins ($state)', () => {
		const { container } = render(SettingsView);
		expect(container.textContent).toContain('General');
	});

	it('Svelte 5 runes work with combined plugins ($derived)', () => {
		const columns: DataTableColumn<{ value: number }>[] = [
			{ key: 'value', label: 'Value', sortable: true }
		];
		const rows = [{ value: 30 }, { value: 10 }, { value: 20 }];
		const { container } = render(DataTable, { props: { columns, rows } });
		expect(container.querySelectorAll('tbody tr').length).toBe(3);
	});
});
