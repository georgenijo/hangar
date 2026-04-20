<script lang="ts" generics="T extends Record<string, unknown>">
	import type { DataTableColumn } from '$lib/types';

	interface Props {
		columns: DataTableColumn<T>[];
		rows: T[];
		onRowClick?: (row: T) => void;
	}

	let { columns, rows, onRowClick }: Props = $props();

	let sortKey: string | null = $state(null);
	let sortDir: 'asc' | 'desc' = $state('asc');

	function handleSort(col: DataTableColumn<T>) {
		if (!col.sortable) return;
		const key = String(col.key);
		if (sortKey === key) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortKey = key;
			sortDir = 'asc';
		}
	}

	const sortedRows = $derived.by(() => {
		if (!sortKey) return rows;
		return [...rows].sort((a, b) => {
			const aVal = a[sortKey as keyof T];
			const bVal = b[sortKey as keyof T];

			// Handle null/undefined
			if (aVal == null && bVal == null) return 0;
			if (aVal == null) return 1;
			if (bVal == null) return -1;

			// Compare values with proper type handling
			let cmp = 0;
			if (typeof aVal === 'number' && typeof bVal === 'number') {
				cmp = aVal - bVal;
			} else {
				// String comparison for non-numeric values
				cmp = String(aVal) < String(bVal) ? -1 : String(aVal) > String(bVal) ? 1 : 0;
			}

			return sortDir === 'asc' ? cmp : -cmp;
		});
	});
</script>

<table class="data-table">
	<thead>
		<tr>
			{#each columns as col}
				<th
					class:sortable={col.sortable}
					style:width={col.width}
					onclick={() => handleSort(col)}
				>
					{col.label}
					{#if col.sortable && sortKey === String(col.key)}
						<span class="sort-indicator">{sortDir === 'asc' ? '↑' : '↓'}</span>
					{/if}
				</th>
			{/each}
		</tr>
	</thead>
	<tbody>
		{#each sortedRows as row}
			<tr class:clickable={!!onRowClick} onclick={() => onRowClick?.(row)}>
				{#each columns as col}
					<td>{col.render ? col.render(row) : String(row[col.key as keyof T] ?? '')}</td>
				{/each}
			</tr>
		{/each}
	</tbody>
</table>

<style>
	.sortable {
		cursor: pointer;
		user-select: none;
	}
	.sortable:hover {
		color: var(--text);
	}
	.sort-indicator {
		margin-left: 4px;
		color: var(--accent);
		font-size: 10px;
	}
	.clickable {
		cursor: pointer;
	}
</style>
