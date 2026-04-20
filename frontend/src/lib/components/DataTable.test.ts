import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import DataTable from './DataTable.svelte';
import type { DataTableColumn } from '$lib/types';

describe('DataTable', () => {
	interface TestRow {
		id: number;
		name: string;
		value: number;
		status: string;
	}

	const columns: DataTableColumn<TestRow>[] = [
		{ key: 'id', label: 'ID', sortable: true },
		{ key: 'name', label: 'Name', sortable: true },
		{ key: 'value', label: 'Value', sortable: false },
		{ key: 'status', label: 'Status', sortable: true }
	];

	const rows: TestRow[] = [
		{ id: 1, name: 'Alice', value: 100, status: 'active' },
		{ id: 2, name: 'Bob', value: 200, status: 'idle' },
		{ id: 3, name: 'Charlie', value: 150, status: 'active' }
	];

	it('renders thead with column labels', () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const headers = container.querySelectorAll('thead th');
		expect(headers.length).toBe(4);
		expect(headers[0].textContent).toContain('ID');
		expect(headers[1].textContent).toContain('Name');
		expect(headers[2].textContent).toContain('Value');
		expect(headers[3].textContent).toContain('Status');
	});

	it('renders tbody with rows', () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const bodyRows = container.querySelectorAll('tbody tr');
		expect(bodyRows.length).toBe(3);

		const firstRowCells = bodyRows[0].querySelectorAll('td');
		expect(firstRowCells[0].textContent).toBe('1');
		expect(firstRowCells[1].textContent).toBe('Alice');
		expect(firstRowCells[2].textContent).toBe('100');
		expect(firstRowCells[3].textContent).toBe('active');
	});

	it('sorts rows in ascending order when clicking sortable column', async () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const nameHeader = container.querySelectorAll('thead th')[1];
		await fireEvent.click(nameHeader);

		const bodyRows = container.querySelectorAll('tbody tr');
		const names = Array.from(bodyRows).map((row) => row.querySelectorAll('td')[1].textContent);
		expect(names).toEqual(['Alice', 'Bob', 'Charlie']);
	});

	it('sorts rows in descending order when clicking sortable column twice', async () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const nameHeader = container.querySelectorAll('thead th')[1];
		await fireEvent.click(nameHeader);
		await fireEvent.click(nameHeader);

		const bodyRows = container.querySelectorAll('tbody tr');
		const names = Array.from(bodyRows).map((row) => row.querySelectorAll('td')[1].textContent);
		expect(names).toEqual(['Charlie', 'Bob', 'Alice']);
	});

	it('toggles between asc and desc when clicking same column multiple times', async () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const idHeader = container.querySelectorAll('thead th')[0];

		// First click: asc
		await fireEvent.click(idHeader);
		let bodyRows = container.querySelectorAll('tbody tr');
		let ids = Array.from(bodyRows).map((row) => row.querySelectorAll('td')[0].textContent);
		expect(ids).toEqual(['1', '2', '3']);

		// Second click: desc
		await fireEvent.click(idHeader);
		bodyRows = container.querySelectorAll('tbody tr');
		ids = Array.from(bodyRows).map((row) => row.querySelectorAll('td')[0].textContent);
		expect(ids).toEqual(['3', '2', '1']);

		// Third click: asc again
		await fireEvent.click(idHeader);
		bodyRows = container.querySelectorAll('tbody tr');
		ids = Array.from(bodyRows).map((row) => row.querySelectorAll('td')[0].textContent);
		expect(ids).toEqual(['1', '2', '3']);
	});

	it('does not sort when clicking non-sortable column', async () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const valueHeader = container.querySelectorAll('thead th')[2];
		await fireEvent.click(valueHeader);

		// Rows should remain in original order
		const bodyRows = container.querySelectorAll('tbody tr');
		const values = Array.from(bodyRows).map((row) => row.querySelectorAll('td')[2].textContent);
		expect(values).toEqual(['100', '200', '150']);
	});

	it('calls onRowClick when row is clicked', async () => {
		const onRowClick = vi.fn();
		const { container } = render(DataTable<TestRow>, {
			props: { columns, rows, onRowClick }
		});

		const firstRow = container.querySelector('tbody tr');
		expect(firstRow).toBeTruthy();

		await fireEvent.click(firstRow!);

		expect(onRowClick).toHaveBeenCalledTimes(1);
		expect(onRowClick).toHaveBeenCalledWith(rows[0]);
	});

	it('does not call onRowClick when not provided', async () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const firstRow = container.querySelector('tbody tr');
		expect(firstRow).toBeTruthy();

		// Should not throw when clicking
		await fireEvent.click(firstRow!);
	});

	it('applies clickable class when onRowClick is provided', () => {
		const onRowClick = vi.fn();
		const { container } = render(DataTable<TestRow>, {
			props: { columns, rows, onRowClick }
		});

		const firstRow = container.querySelector('tbody tr');
		expect(firstRow?.classList.contains('clickable')).toBe(true);
	});

	it('does not apply clickable class when onRowClick is not provided', () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const firstRow = container.querySelector('tbody tr');
		expect(firstRow?.classList.contains('clickable')).toBe(false);
	});

	it('renders with custom render function', () => {
		const customColumns: DataTableColumn<TestRow>[] = [
			{
				key: 'name',
				label: 'User',
				sortable: true,
				render: (row) => `User: ${row.name}`
			},
			{
				key: 'value',
				label: 'Amount',
				sortable: false,
				render: (row) => `$${row.value.toFixed(2)}`
			}
		];

		const { container } = render(DataTable<TestRow>, {
			props: { columns: customColumns, rows }
		});

		const firstRowCells = container.querySelectorAll('tbody tr')[0].querySelectorAll('td');
		expect(firstRowCells[0].textContent).toBe('User: Alice');
		expect(firstRowCells[1].textContent).toBe('$100.00');
	});

	it('shows sort indicator on sorted column', async () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const nameHeader = container.querySelectorAll('thead th')[1];
		await fireEvent.click(nameHeader);

		const sortIndicator = nameHeader.querySelector('.sort-indicator');
		expect(sortIndicator).toBeTruthy();
		expect(sortIndicator?.textContent).toBe('↑');
	});

	it('shows descending sort indicator after second click', async () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const nameHeader = container.querySelectorAll('thead th')[1];
		await fireEvent.click(nameHeader);
		await fireEvent.click(nameHeader);

		const sortIndicator = nameHeader.querySelector('.sort-indicator');
		expect(sortIndicator).toBeTruthy();
		expect(sortIndicator?.textContent).toBe('↓');
	});

	it('handles empty rows array', () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows: [] } });

		const bodyRows = container.querySelectorAll('tbody tr');
		expect(bodyRows.length).toBe(0);
	});

	it('sorts correctly with different data types', async () => {
		interface MixedRow {
			str: string;
			num: number;
		}

		const mixedColumns: DataTableColumn<MixedRow>[] = [
			{ key: 'str', label: 'String', sortable: true },
			{ key: 'num', label: 'Number', sortable: true }
		];

		const mixedRows: MixedRow[] = [
			{ str: 'banana', num: 3 },
			{ str: 'apple', num: 1 },
			{ str: 'cherry', num: 2 }
		];

		const { container } = render(DataTable<MixedRow>, {
			props: { columns: mixedColumns, rows: mixedRows }
		});

		// Sort by string
		const strHeader = container.querySelectorAll('thead th')[0];
		await fireEvent.click(strHeader);

		let bodyRows = container.querySelectorAll('tbody tr');
		let strs = Array.from(bodyRows).map((row) => row.querySelectorAll('td')[0].textContent);
		expect(strs).toEqual(['apple', 'banana', 'cherry']);

		// Sort by number
		const numHeader = container.querySelectorAll('thead th')[1];
		await fireEvent.click(numHeader);

		bodyRows = container.querySelectorAll('tbody tr');
		const nums = Array.from(bodyRows).map((row) => row.querySelectorAll('td')[1].textContent);
		expect(nums).toEqual(['1', '2', '3']);
	});

	it('applies sortable class to sortable column headers', () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		const headers = container.querySelectorAll('thead th');
		expect(headers[0].classList.contains('sortable')).toBe(true); // ID
		expect(headers[1].classList.contains('sortable')).toBe(true); // Name
		expect(headers[2].classList.contains('sortable')).toBe(false); // Value
		expect(headers[3].classList.contains('sortable')).toBe(true); // Status
	});

	it('maintains sort when switching between different columns', async () => {
		const { container } = render(DataTable<TestRow>, { props: { columns, rows } });

		// Sort by ID desc
		const idHeader = container.querySelectorAll('thead th')[0];
		await fireEvent.click(idHeader);
		await fireEvent.click(idHeader);

		let bodyRows = container.querySelectorAll('tbody tr');
		let ids = Array.from(bodyRows).map((row) => row.querySelectorAll('td')[0].textContent);
		expect(ids).toEqual(['3', '2', '1']);

		// Sort by name asc (should reset to asc)
		const nameHeader = container.querySelectorAll('thead th')[1];
		await fireEvent.click(nameHeader);

		bodyRows = container.querySelectorAll('tbody tr');
		const names = Array.from(bodyRows).map((row) => row.querySelectorAll('td')[1].textContent);
		expect(names).toEqual(['Alice', 'Bob', 'Charlie']);
	});
});
