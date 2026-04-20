<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import RingGauge from '$lib/components/RingGauge.svelte';
	import { hostStore } from '$lib/stores/host.svelte';

	onMount(() => {
		hostStore.startPolling();
	});

	onDestroy(() => {
		hostStore.stopPolling();
	});

	// Static mock data for additional hosts (multi-host data as specified)
	const mockHosts = [
		{
			hostname: 'macmini',
			cpu_pct: 42,
			ramPct: 68,
			diskPct: 55
		},
		{
			hostname: 'optiplex',
			cpu_pct: 28,
			ramPct: 45,
			diskPct: 72
		}
	];
</script>

<div id="view-fleet" class="view">
	<div class="fleet-grid">
		<!-- Local host card - uses real data from hostStore -->
		<div class="host-card">
			<div class="host-header">
				<h3>{hostStore.metrics?.hostname || 'localhost'}</h3>
				<span class="host-status live">●</span>
			</div>
			<div class="host-metrics">
				<RingGauge value={hostStore.cpuPct} label="CPU" />
				<RingGauge value={hostStore.ramPct} label="RAM" />
				<RingGauge value={hostStore.diskPct} label="DISK" />
			</div>
		</div>

		<!-- Mock hosts - static data -->
		{#each mockHosts as host}
			<div class="host-card">
				<div class="host-header">
					<h3>{host.hostname}</h3>
					<span class="host-status live">●</span>
				</div>
				<div class="host-metrics">
					<RingGauge value={host.cpu_pct} label="CPU" />
					<RingGauge value={host.ramPct} label="RAM" />
					<RingGauge value={host.diskPct} label="DISK" />
				</div>
			</div>
		{/each}
	</div>
</div>

<style>
	.view {
		padding: 24px;
	}

	.fleet-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: 24px;
	}

	.host-card {
		background: var(--surface-2, #1a1a1a);
		border-radius: 8px;
		padding: 24px;
		border: 1px solid var(--border, #333);
	}

	.host-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 20px;
	}

	.host-header h3 {
		margin: 0;
		font-size: 18px;
		font-weight: 600;
		color: var(--text, #fff);
	}

	.host-status {
		font-size: 12px;
	}

	.host-status.live {
		color: var(--success, #22c55e);
	}

	.host-metrics {
		display: flex;
		justify-content: space-around;
		gap: 16px;
	}
</style>
