<script lang="ts">
	import type { HostMetrics } from '$lib/types';
	import RingGauge from './RingGauge.svelte';

	let {
		metrics,
		sessionsCount,
		loading,
		error,
		onRetry
	}: {
		metrics: HostMetrics | null;
		sessionsCount: number;
		loading: boolean;
		error: string | null;
		onRetry?: () => void;
	} = $props();

	function safeGb(total: number, pct: number, decimals: number): string {
		const v = (total * pct) / 100;
		return Number.isFinite(v) ? v.toFixed(decimals) : '—';
	}

	let ramUsed = $derived(metrics ? safeGb(metrics.ram_total_gb, metrics.ram_pct, 1) : '—');
	let diskUsed = $derived(metrics ? safeGb(metrics.disk_total_gb, metrics.disk_pct, 0) : '—');
</script>

<div class="host-card">
	<div class="card-header">
		<h2 class="hostname">{metrics?.hostname ?? (loading ? '…' : '—')}</h2>
		<span class="sessions-badge">{sessionsCount} sessions</span>
	</div>

	{#if metrics}
		<div class="spec-line">
			RAM {Number.isFinite(metrics.ram_total_gb) ? metrics.ram_total_gb.toFixed(0) : '—'} GB ·
			Disk {Number.isFinite(metrics.disk_total_gb) ? metrics.disk_total_gb.toFixed(0) : '—'} GB
		</div>
	{:else if !loading}
		<div class="spec-line">—</div>
	{/if}

	{#if error}
		<div class="card-error">
			<span>{error}</span>
			{#if onRetry}
				<button class="retry-btn" onclick={onRetry}>Retry</button>
			{/if}
		</div>
	{/if}

	<div class="gauges-row">
		<RingGauge
			pct={metrics?.cpu_pct ?? 0}
			label="CPU"
		/>
		<RingGauge
			pct={metrics?.ram_pct ?? 0}
			label="RAM"
			sublabel="{ramUsed} / {metrics && Number.isFinite(metrics.ram_total_gb) ? metrics.ram_total_gb.toFixed(0) : '—'} GB"
		/>
		<RingGauge
			pct={metrics?.disk_pct ?? 0}
			label="Disk"
			sublabel="{diskUsed} / {metrics && Number.isFinite(metrics.disk_total_gb) ? metrics.disk_total_gb.toFixed(0) : '—'} GB"
		/>
	</div>
</div>

<style>
	.host-card {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 16px 20px;
	}

	.card-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 6px;
	}

	.hostname {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
		font-family: var(--font-mono);
		color: var(--text);
	}

	.sessions-badge {
		background: color-mix(in srgb, var(--accent) 15%, transparent);
		color: var(--accent);
		font-size: 0.75rem;
		font-weight: 600;
		padding: 2px 10px;
		border-radius: 12px;
	}

	.spec-line {
		font-size: 0.82rem;
		font-family: var(--font-mono);
		color: var(--text-muted);
		margin-bottom: 16px;
	}

	.gauges-row {
		display: flex;
		gap: 28px;
		justify-content: center;
		padding-top: 8px;
	}

	.card-error {
		display: flex;
		align-items: center;
		gap: 10px;
		background: color-mix(in srgb, #f44336 12%, transparent);
		border: 1px solid color-mix(in srgb, #f44336 30%, transparent);
		border-radius: var(--radius);
		padding: 6px 12px;
		font-size: 0.82rem;
		color: #f44336;
		margin-bottom: 12px;
	}

	.retry-btn {
		background: none;
		border: 1px solid #f44336;
		border-radius: var(--radius);
		color: #f44336;
		cursor: pointer;
		font-size: 0.78rem;
		padding: 2px 8px;
	}

	.retry-btn:hover {
		background: color-mix(in srgb, #f44336 15%, transparent);
	}
</style>
