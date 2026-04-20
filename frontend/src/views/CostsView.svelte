<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { costsStore } from '$lib/stores/costs.svelte';
	import KpiCard from '$lib/components/KpiCard.svelte';
	import BarChart from '$lib/components/BarChart.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import type { KpiCardData, DataTableColumn, ModelCost } from '$lib/types';
	import { formatCost } from '$lib/utils';

	onMount(() => {
		costsStore.startPolling();
	});

	onDestroy(() => {
		costsStore.stopPolling();
	});

	// KPI cards for costs summary
	const kpis = $derived<KpiCardData[]>([
		{
			label: '30d Spend',
			value: formatCost(costsStore.totalSpend),
			sparkline: costsStore.dailyAmounts.slice(-30)
		},
		{
			label: '7d Spend',
			value: formatCost(costsStore.last7DaysSpend),
			sparkline: costsStore.dailyAmounts.slice(-7)
		},
		{
			label: 'Total Models',
			value: String(costsStore.modelCosts.length)
		},
		{
			label: 'Avg Daily',
			value: formatCost(
				costsStore.dailyCosts.length > 0
					? costsStore.totalSpend / costsStore.dailyCosts.length
					: 0
			)
		}
	]);

	// Daily bar chart data (last 30 days)
	const dailyBars = $derived(
		costsStore.dailyCosts.slice(-30).map((d) => ({
			label: d.date.slice(5), // Show MM-DD
			value: d.dollars
		}))
	);

	// By-model horizontal bars
	const modelBars = $derived(
		costsStore.modelCosts.map((m) => ({
			label: m.model,
			value: m.dollars
		}))
	);

	// Cost-per-PR table (mock data for now since we don't have real PR data)
	const prTableColumns: DataTableColumn<{
		pr: number;
		title: string;
		cost: number;
		agents: number;
		tokens: number;
	}>[] = [
		{ key: 'pr', label: 'PR', sortable: true, width: '60px' },
		{ key: 'title', label: 'Title', sortable: true },
		{
			key: 'cost',
			label: 'Cost',
			sortable: true,
			width: '100px',
			render: (row) => formatCost(row.cost)
		},
		{ key: 'agents', label: 'Agents', sortable: true, width: '80px' },
		{
			key: 'tokens',
			label: 'Tokens',
			sortable: true,
			width: '100px',
			render: (row) => row.tokens.toLocaleString()
		}
	];

	// Mock PR data (would come from backend in real implementation)
	const prTableRows = [
		{ pr: 79, title: 'Cost scraper lag fix', cost: 4.82, agents: 5, tokens: 482000 },
		{ pr: 76, title: 'Pipeline isolation', cost: 3.21, agents: 4, tokens: 321000 },
		{ pr: 74, title: 'Tester gate', cost: 2.14, agents: 3, tokens: 214000 }
	];
</script>

<div id="view-costs" class="view">
	<div class="kpi-grid">
		{#each kpis as kpi}
			<KpiCard data={kpi} />
		{/each}
	</div>

	<div class="charts-section">
		<div class="chart-card">
			<h3 class="chart-title">Daily Spend (Last 30 Days)</h3>
			{#if dailyBars.length > 0}
				<BarChart data={dailyBars} width={800} height={200} horizontal={false} />
			{:else}
				<div class="empty-chart">No data available</div>
			{/if}
		</div>

		<div class="chart-card">
			<h3 class="chart-title">Cost by Model</h3>
			{#if modelBars.length > 0}
				<BarChart data={modelBars} width={600} height={200} horizontal={true} />
			{:else}
				<div class="empty-chart">No data available</div>
			{/if}
		</div>
	</div>

	<div class="table-section">
		<h3 class="section-title">Cost per PR</h3>
		<DataTable columns={prTableColumns} rows={prTableRows} />
	</div>
</div>

<style>
	.view {
		display: flex;
		flex-direction: column;
		gap: 24px;
		height: 100%;
		overflow-y: auto;
	}

	.kpi-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 16px;
	}

	.charts-section {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.chart-card {
		background: var(--surface-2);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 20px;
	}

	.chart-title {
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--text);
		margin: 0 0 16px 0;
		font-family: var(--mono);
	}

	.empty-chart {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 200px;
		color: var(--text-3);
		font-size: 0.9rem;
	}

	.table-section {
		background: var(--surface-2);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 20px;
	}

	.section-title {
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--text);
		margin: 0 0 16px 0;
		font-family: var(--mono);
	}
</style>
