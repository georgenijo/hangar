<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { costsStore } from '$lib/stores/costs.svelte';
	import { pipelineStore } from '$lib/stores/pipeline.svelte';
	import { hostStore } from '$lib/stores/host.svelte';
	import KpiCard from '$lib/components/KpiCard.svelte';
	import type { KpiCardData } from '$lib/types';

	onMount(() => {
		sessionsStore.startPolling();
		costsStore.startPolling();
		pipelineStore.startPolling();
		hostStore.startPolling();
	});

	onDestroy(() => {
		sessionsStore.stopPolling();
		costsStore.stopPolling();
		pipelineStore.stopPolling();
		hostStore.stopPolling();
	});

	// Derive KPI data from stores
	const activeSessions = $derived(
		sessionsStore.sessions.filter((s) => s.state === 'streaming' || s.state === 'idle').length
	);

	const kpis = $derived<KpiCardData[]>([
		{
			label: 'Active Sessions',
			value: String(activeSessions),
			alert: activeSessions > 10
		},
		{
			label: '7d Spend',
			value: `$${costsStore.last7DaysSpend.toFixed(2)}`,
			sparkline: costsStore.dailyAmounts.slice(-7)
		},
		{
			label: 'Live Pipelines',
			value: String(pipelineStore.activeRuns.length)
		},
		{
			label: 'CPU',
			value: `${Math.round(hostStore.cpuPct)}%`,
			alert: hostStore.cpuPct > 75
		},
		{
			label: 'RAM',
			value: `${Math.round(hostStore.ramPct)}%`,
			alert: hostStore.ramPct > 75
		}
	]);
</script>

<div id="view-command" class="view">
	<div class="kpi-grid">
		{#each kpis as kpi}
			<KpiCard data={kpi} />
		{/each}
	</div>
</div>

<style>
	.kpi-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 16px;
		margin-bottom: 24px;
	}
</style>
