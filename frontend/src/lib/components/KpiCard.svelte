<script lang="ts">
	import Sparkline from './Sparkline.svelte';
	import type { KpiCardData } from '$lib/types';

	interface Props {
		data: KpiCardData;
	}

	let { data }: Props = $props();
</script>

<div class="kpi" class:kpi-alert={data.alert}>
	<div class="kpi-header">
		<span class="kpi-label">{data.label}</span>
		{#if data.trend}
			<span class="trend trend-{data.trend.direction}">
				{data.trend.direction === 'up' ? '↑' : data.trend.direction === 'down' ? '↓' : '→'} {data.trend.text}
			</span>
		{/if}
	</div>
	<div class="kpi-value">
		<span class="num">{data.value}</span>
	</div>
	{#if data.sparkline && data.sparkline.length > 0}
		<Sparkline data={data.sparkline} width={80} height={20} />
	{/if}
</div>

<style>
	.kpi-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 6px;
	}
</style>
