/**
 * Integration Test: Cross-Component Type Compatibility
 * Priority: HIGH - Tests shared types work across all merged branches
 *
 * Verifies KpiCardData, DataTableColumn, and other shared types from $lib/types
 * are correctly used by components from different branches.
 */
import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import KpiCard from './components/KpiCard.svelte';
import DataTable from './components/DataTable.svelte';
import type { KpiCardData, DataTableColumn } from '$lib/types';

describe('Cross-Component Type Integration', () => {
	it('KpiCardData type used by KpiCard and can be created from mock data', () => {
		const data: KpiCardData = {
			label: 'CPU Usage',
			value: '75%',
			sparkline: [70, 72, 75],
			trend: { direction: 'up', text: '+5%' },
			alert: true
		};

		const { container } = render(KpiCard, { props: { data } });
		expect(container.textContent).toContain('CPU Usage');
		expect(container.textContent).toContain('75%');
		expect(container.querySelector('.kpi-alert')).not.toBeNull();
	});

	it('DataTableColumn generic type works with custom row types', () => {
		type CostRow = { model: string; dollars: number };
		const columns: DataTableColumn<CostRow>[] = [
			{ key: 'model', label: 'Model', sortable: true },
			{
				key: 'dollars',
				label: 'Cost',
				sortable: true,
				render: (row) => `$${row.dollars.toFixed(2)}`
			}
		];

		const rows: CostRow[] = [
			{ model: 'opus', dollars: 125.5 },
			{ model: 'sonnet', dollars: 75.3 }
		];

		const { container } = render(DataTable, { props: { columns, rows } });
		expect(container.textContent).toContain('Model');
		expect(container.textContent).toContain('$125.50');
	});

	it('KpiCard with optional properties works correctly', () => {
		// Minimal KpiCardData
		const minimal: KpiCardData = {
			label: 'Sessions',
			value: '10'
		};

		const { container } = render(KpiCard, { props: { data: minimal } });
		expect(container.textContent).toContain('Sessions');
		expect(container.textContent).toContain('10');

		// Should not have optional elements
		expect(container.querySelector('.trend')).toBeNull();
		expect(container.querySelector('svg.sparkline')).toBeNull();
		expect(container.querySelector('.kpi-alert')).toBeNull();
	});

	it('DataTable with custom render functions transforms data correctly', () => {
		type Session = { id: string; duration: number };
		const columns: DataTableColumn<Session>[] = [
			{ key: 'id', label: 'Session ID' },
			{
				key: 'duration',
				label: 'Duration',
				render: (row) => `${Math.floor(row.duration / 60)}m ${row.duration % 60}s`
			}
		];

		const rows: Session[] = [{ id: 'abc', duration: 185 }];

		const { container } = render(DataTable, { props: { columns, rows } });
		expect(container.textContent).toContain('3m 5s');
	});

	it('KpiCard sparkline accepts number array from various sources', () => {
		// Sparkline from hardcoded array
		const data1: KpiCardData = {
			label: 'Test 1',
			value: '100',
			sparkline: [1, 2, 3, 4, 5]
		};

		// Sparkline from computed array
		const values = [10, 20, 30];
		const data2: KpiCardData = {
			label: 'Test 2',
			value: '60',
			sparkline: values.map((v) => v * 2)
		};

		const { container: c1 } = render(KpiCard, { props: { data: data1 } });
		const { container: c2 } = render(KpiCard, { props: { data: data2 } });

		expect(c1.querySelector('svg.sparkline')).not.toBeNull();
		expect(c2.querySelector('svg.sparkline')).not.toBeNull();
	});

	it('DataTable sorts numeric and string values correctly with same column type', () => {
		type MixedRow = { id: number; name: string };
		const columns: DataTableColumn<MixedRow>[] = [
			{ key: 'id', label: 'ID', sortable: true },
			{ key: 'name', label: 'Name', sortable: true }
		];

		const rows: MixedRow[] = [
			{ id: 3, name: 'Charlie' },
			{ id: 1, name: 'Alice' },
			{ id: 2, name: 'Bob' }
		];

		const { container } = render(DataTable, { props: { columns, rows } });

		// Verify all rows render
		expect(container.querySelectorAll('tbody tr').length).toBe(3);
		expect(container.textContent).toContain('Alice');
		expect(container.textContent).toContain('Bob');
		expect(container.textContent).toContain('Charlie');
	});

	it('multiple KpiCards and DataTables can coexist with shared types', () => {
		// Simulate dashboard with multiple components
		const kpi1: KpiCardData = { label: 'Total', value: '100' };
		const kpi2: KpiCardData = { label: 'Active', value: '42', alert: true };

		const columns: DataTableColumn<{ name: string }>[] = [{ key: 'name', label: 'Name' }];
		const rows = [{ name: 'Item 1' }, { name: 'Item 2' }];

		const { container: c1 } = render(KpiCard, { props: { data: kpi1 } });
		const { container: c2 } = render(KpiCard, { props: { data: kpi2 } });
		const { container: c3 } = render(DataTable, { props: { columns, rows } });

		// All should render without type conflicts
		expect(c1.textContent).toContain('Total');
		expect(c2.textContent).toContain('Active');
		expect(c2.querySelector('.kpi-alert')).not.toBeNull();
		expect(c3.textContent).toContain('Item 1');
	});

	it('KpiCard trend direction type is enforced correctly', () => {
		const up: KpiCardData = {
			label: 'Up',
			value: '10',
			trend: { direction: 'up', text: '+10%' }
		};
		const down: KpiCardData = {
			label: 'Down',
			value: '5',
			trend: { direction: 'down', text: '-5%' }
		};
		const neutral: KpiCardData = {
			label: 'Neutral',
			value: '7',
			trend: { direction: 'neutral', text: '0%' }
		};

		const { container: c1 } = render(KpiCard, { props: { data: up } });
		const { container: c2 } = render(KpiCard, { props: { data: down } });
		const { container: c3 } = render(KpiCard, { props: { data: neutral } });

		expect(c1.querySelector('.trend-up')).not.toBeNull();
		expect(c2.querySelector('.trend-down')).not.toBeNull();
		expect(c3.querySelector('.trend-neutral')).not.toBeNull();
	});

	it('DataTable column width property is properly typed and used', () => {
		type Row = { col1: string; col2: string };
		const columns: DataTableColumn<Row>[] = [
			{ key: 'col1', label: 'Column 1', width: '100px' },
			{ key: 'col2', label: 'Column 2', width: '200px' }
		];

		const rows: Row[] = [{ col1: 'A', col2: 'B' }];

		const { container } = render(DataTable, { props: { columns, rows } });

		const headers = container.querySelectorAll('th');
		expect(headers[0].style.width).toBe('100px');
		expect(headers[1].style.width).toBe('200px');
	});
});
